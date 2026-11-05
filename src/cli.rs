//! `qd` command line (contracts/cli.md). JSON on stdout, JSON errors on stderr.

use crate::deck::{Block, BlockKind, Deck, Slide};
use crate::error::{QdError, Result};
use crate::ops::{self, AttrChange, BoxContent, Patch, Pos, SlideSpec};
use crate::store::Store;
use clap::{Args, CommandFactory, Parser, Subcommand};
use serde_json::{Value, json};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const SKILL: &str = include_str!("../skills/quartz-deck/SKILL.md");

#[derive(Parser)]
#[command(
    name = "qd",
    version,
    about = "Edit Quarto reveal.js decks (.qmd) from the command line. JSON in, JSON out."
)]
pub struct Cli {
    /// Deck to operate on (default: $QD_DECK, else the only *.qmd in the current directory)
    #[arg(long, short = 'd', global = true, env = "QD_DECK")]
    deck: Option<PathBuf>,
    /// Human-readable output instead of JSON
    #[arg(long, global = true)]
    human: bool,
    /// For edits: print the diff without writing
    #[arg(long, global = true)]
    dry_run: bool,
    /// For edits: fail with E_CONFLICT unless the file is at this version
    #[arg(long, global = true, value_name = "VERSION")]
    expect: Option<String>,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Args, Default)]
#[group(multiple = false)]
struct Input {
    /// Markdown text (`-` reads stdin)
    #[arg(long, allow_hyphen_values = true)]
    md: Option<String>,
    /// Read markdown from a file
    #[arg(long)]
    file: Option<PathBuf>,
}

#[derive(Args)]
#[group(required = true, multiple = false)]
struct Where {
    /// Place after this slide
    #[arg(long)]
    after: Option<String>,
    /// Place before this slide
    #[arg(long)]
    before: Option<String>,
}

impl Where {
    fn pos(&self) -> Pos<'_> {
        match (&self.after, &self.before) {
            (Some(a), _) => Pos::After(a),
            (_, Some(b)) => Pos::Before(b),
            _ => unreachable!("clap group is required"),
        }
    }
}

#[derive(Subcommand)]
enum Cmd {
    /// List slides with ids, titles and summaries
    Slides,
    /// Show one slide: attributes, source and blocks
    Show { slide: String },
    /// Print front matter, or set/unset a top-level key
    Meta {
        #[command(subcommand)]
        action: Option<MetaCmd>,
    },
    /// Validate the deck (duplicate ids, unsafe constructs, missing files)
    Check {
        /// Also render with Quarto and report render errors (slower)
        #[arg(long)]
        render: bool,
        /// Check that the rendered deck works with no network (headless Chrome)
        #[arg(long)]
        offline: bool,
        /// With --offline: check this rendered HTML instead of rendering the deck
        #[arg(long, requires = "offline")]
        html: Option<PathBuf>,
        /// Never download a browser (also QD_NO_DOWNLOAD=1)
        #[arg(long)]
        no_download: bool,
    },
    /// Export a self-contained offline deck (one HTML + files that cannot be inlined), checked offline
    Export {
        /// Destination folder (default: export/ next to the deck)
        #[arg(long)]
        out: Option<PathBuf>,
        /// Also print a PDF
        #[arg(long)]
        pdf: bool,
        /// PDF: speaker notes on separate pages
        #[arg(long, requires = "pdf")]
        notes: bool,
        /// PDF: one page per slide instead of per fragment step
        #[arg(long, requires = "pdf")]
        per_slide: bool,
        /// Never download MathJax or a browser (also QD_NO_DOWNLOAD=1)
        #[arg(long)]
        no_download: bool,
    },
    /// Print a deck to PDF (default: the deck's export) through reveal.js's print mode
    Pdf {
        /// Rendered HTML to print (default: export/<deck>.html)
        #[arg(long)]
        html: Option<PathBuf>,
        /// Output PDF (default: next to the HTML)
        #[arg(long)]
        out: Option<PathBuf>,
        /// Speaker notes on separate pages
        #[arg(long)]
        notes: bool,
        /// One page per slide instead of per fragment step
        #[arg(long)]
        per_slide: bool,
        /// Never download a browser (also QD_NO_DOWNLOAD=1)
        #[arg(long)]
        no_download: bool,
    },
    /// Show the browser used for offline checks/PDF; install or remove qd's copy
    Browser {
        /// Download qd's pinned Chrome for Testing if no browser is found
        #[arg(long, conflicts_with = "remove")]
        install: bool,
        /// Delete qd's downloaded browser
        #[arg(long)]
        remove: bool,
        /// Never download (also QD_NO_DOWNLOAD=1)
        #[arg(long)]
        no_download: bool,
    },
    /// Serve the live preview/editor on 127.0.0.1 (Ctrl-C to stop)
    Serve {
        /// Port (0 = pick a free one)
        #[arg(long, default_value_t = 0)]
        port: u16,
        /// Open the browser
        #[arg(long)]
        open: bool,
        /// Start in the background and return immediately
        #[arg(long)]
        detach: bool,
    },
    /// Your editor settings, the same for every deck and editor (spec 007)
    Settings {
        #[command(subcommand)]
        action: Option<SettingsCmd>,
    },
    /// The deck's reproducible Python environment (uv, or pixi with PyPI packages)
    Env {
        #[command(subcommand)]
        action: Option<EnvCmd>,
    },
    /// Render the deck with Quarto (as `quarto render`) and report the output
    Render {
        /// Output format (default: the deck's own, usually revealjs)
        #[arg(long)]
        to: Option<String>,
    },
    /// Run code cells: in the running `qd serve` session, else a fresh one-shot session
    Run {
        /// Cell refs (`<slide>/<i>`, `#id`)
        #[arg(required_unless_present = "all")]
        cells: Vec<String>,
        /// Every cell in order, stopping at the first error
        #[arg(long, conflicts_with = "cells")]
        all: bool,
        /// Restart and run from the top (through the last given cell)
        #[arg(long)]
        fresh: bool,
        /// Interrupt a cell after this many seconds
        #[arg(long)]
        timeout: Option<u64>,
    },
    /// What the person has in focus in the editor: slide, selected elements and text
    Focus {
        /// Point the editor at this slide or element instead
        #[arg(long, value_name = "REF")]
        set: Option<String>,
    },
    /// Stored outputs of code cells (all, or the given ones), with `stale`
    Outputs { cells: Vec<String> },
    /// URL of the running preview (optionally opening on a slide)
    Url { slide: Option<String> },
    /// Stop the running preview server for this deck
    Stop,
    /// Print the machine-readable description of every command
    Schema,
    /// Print the agent Skill, or install it into a directory
    Skill {
        /// Write SKILL.md into this directory
        #[arg(long)]
        install: Option<PathBuf>,
    },
    /// Add a slide (default: at the end)
    Add {
        #[arg(long, conflicts_with = "before")]
        after: Option<String>,
        #[arg(long)]
        before: Option<String>,
        #[arg(long, allow_hyphen_values = true)]
        title: Option<String>,
        #[command(flatten)]
        input: Input,
        /// Heading attribute key=value (repeatable)
        #[arg(long = "attr", value_parser = parse_kv)]
        attrs: Vec<(String, String)>,
        /// Heading class (repeatable)
        #[arg(long = "class")]
        classes: Vec<String>,
    },
    /// Duplicate a slide (new ids)
    Dup {
        slide: String,
        #[arg(long)]
        after: Option<String>,
    },
    /// Move a slide
    Move {
        slide: String,
        #[command(flatten)]
        to: Where,
    },
    /// Remove a slide
    Rm { slide: String },
    /// Set a slide's title ("" makes it untitled)
    Title {
        slide: String,
        #[arg(allow_hyphen_values = true)]
        text: String,
        /// Fail with E_CONFLICT unless the current title hashes to this (`title_hash` in `qd show`)
        #[arg(long, value_name = "HASH")]
        expect_text: Option<String>,
    },
    /// Replace a slide's content (positioned boxes are kept)
    Body {
        slide: String,
        #[command(flatten)]
        input: Input,
    },
    /// Change slide heading attributes and classes
    Set {
        slide: String,
        /// key=value (repeatable)
        #[arg(long = "attr", value_parser = parse_kv)]
        attrs: Vec<(String, String)>,
        /// Remove attribute key (repeatable)
        #[arg(long)]
        unset: Vec<String>,
        /// Add class (repeatable)
        #[arg(long = "class")]
        classes: Vec<String>,
        /// Remove class (repeatable)
        #[arg(long)]
        unclass: Vec<String>,
    },
    /// Replace one element's source (ref: <slide>/<index> or #id)
    Edit {
        element: String,
        #[command(flatten)]
        input: Input,
        /// Fail with E_CONFLICT unless the element's source hashes to this (`hash` in `qd show`)
        #[arg(long, value_name = "HASH")]
        expect_text: Option<String>,
    },
    /// Positioned boxes (Quarto `.absolute`; px on the slide, default 1050x700)
    Box {
        #[command(subcommand)]
        action: BoxCmd,
    },
    /// Replace the one occurrence of some text in the deck (refused if missing or repeated)
    Replace {
        /// Text to replace (`-` reads stdin)
        #[arg(long, allow_hyphen_values = true, required_unless_present = "old_file")]
        old: Option<String>,
        /// Read the text to replace from a file
        #[arg(long, conflicts_with = "old")]
        old_file: Option<PathBuf>,
        /// Replacement text (`-` reads stdin)
        #[arg(long, allow_hyphen_values = true, required_unless_present = "new_file")]
        new: Option<String>,
        /// Read the replacement from a file
        #[arg(long, conflicts_with = "new")]
        new_file: Option<PathBuf>,
    },
    /// Single elements of a slide: add, move, remove, duplicate (ref: <slide>/<index> or #id)
    Block {
        #[command(subcommand)]
        action: BlockCmd,
    },
    /// Revert the last qd edit
    Undo,
    /// Re-apply the last undone edit
    Redo,
    /// Create a new minimal reveal.js deck
    New {
        path: PathBuf,
        #[arg(long)]
        title: Option<String>,
    },
}

#[derive(Args)]
#[group(required = true, multiple = false)]
struct BoxSource {
    /// Markdown text for a text box
    #[arg(long, allow_hyphen_values = true)]
    text: Option<String>,
    /// Image path or URL for an image box
    #[arg(long)]
    image: Option<String>,
}

#[derive(Subcommand)]
enum SettingsCmd {
    /// Change one setting (VALUE as JSON when it parses: true, 420; else text: dark)
    Set { key: String, value: String },
    /// Every setting back to its default
    Reset,
}

#[derive(Subcommand)]
enum EnvCmd {
    /// Create the environment (manifest + lockfile) with the base Jupyter packages
    Init {
        #[arg(long, value_enum, default_value = "uv")]
        tool: crate::env::Tool,
    },
    /// Add Python packages from PyPI and relock
    Add {
        #[arg(required = true)]
        packages: Vec<String>,
    },
    /// Install exactly the locked versions (e.g. on another machine)
    Sync,
}

#[derive(Subcommand)]
enum BoxCmd {
    /// Add a text or image box to a slide (default: 300x100, centred)
    Add {
        slide: String,
        #[command(flatten)]
        source: BoxSource,
        /// LEFT,TOP in px
        #[arg(long, value_parser = parse_pair, allow_hyphen_values = true)]
        at: Option<(f64, f64)>,
        /// W,H in px
        #[arg(long, value_parser = parse_pair)]
        size: Option<(f64, f64)>,
    },
    /// Move a box: to LEFT,TOP, or by DX,DY
    Move {
        #[arg(id = "box", value_name = "BOX")]
        bx: String,
        #[arg(long, value_parser = parse_pair, allow_hyphen_values = true, conflicts_with = "by", required_unless_present = "by")]
        at: Option<(f64, f64)>,
        #[arg(long, value_parser = parse_pair, allow_hyphen_values = true)]
        by: Option<(f64, f64)>,
    },
    /// Resize a box to W,H (optionally also moving it to LEFT,TOP)
    Resize {
        #[arg(id = "box", value_name = "BOX")]
        bx: String,
        #[arg(long, value_parser = parse_pair, required = true)]
        size: Option<(f64, f64)>,
        #[arg(long, value_parser = parse_pair, allow_hyphen_values = true)]
        at: Option<(f64, f64)>,
    },
    /// Remove a box
    Rm {
        #[arg(id = "box", value_name = "BOX")]
        bx: String,
    },
}

#[derive(Subcommand)]
enum BlockCmd {
    /// Add markdown as new block(s) on a slide
    Add {
        slide: String,
        #[command(flatten)]
        at: BlockWhere,
        #[command(flatten)]
        input: Input,
    },
    /// Move an element before/after another element of the same slide
    Move {
        element: String,
        #[command(flatten)]
        to: Where,
    },
    /// Remove an element
    Rm { element: String },
    /// Duplicate an element (placed right after it; explicit ids are renewed)
    Dup { element: String },
    /// Add/remove classes on an element (e.g. `--add fragment`: appears on click)
    Class {
        element: String,
        /// Class to add (repeatable)
        #[arg(long)]
        add: Vec<String>,
        /// Class to remove (repeatable)
        #[arg(long)]
        remove: Vec<String>,
    },
}

#[derive(Args)]
#[group(required = true, multiple = false)]
struct BlockWhere {
    /// Place after this element
    #[arg(long)]
    after: Option<String>,
    /// Place before this element
    #[arg(long)]
    before: Option<String>,
    /// Place before the slide's first element
    #[arg(long)]
    start: bool,
    /// Place after the slide's last element
    #[arg(long)]
    end: bool,
}

/// `X,Y` (numbers, px).
fn parse_pair(s: &str) -> std::result::Result<(f64, f64), String> {
    let (a, b) = s
        .split_once(',')
        .ok_or_else(|| format!("expected X,Y, got '{s}'"))?;
    let n = |v: &str| {
        v.trim()
            .parse::<f64>()
            .map_err(|_| format!("'{v}' is not a number"))
    };
    Ok((n(a)?, n(b)?))
}

#[derive(Subcommand)]
enum MetaCmd {
    /// Set a top-level key to a single-line YAML value
    Set {
        key: String,
        #[arg(allow_hyphen_values = true)]
        value: String,
    },
    /// Remove a top-level key
    Unset { key: String },
}

fn parse_kv(s: &str) -> std::result::Result<(String, String), String> {
    s.split_once('=')
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .ok_or_else(|| format!("expected key=value, got '{s}'"))
}

/// Output shapes for `qd schema` (kept next to the commands they describe).
fn output_of(cmd: &str) -> &'static str {
    match cmd {
        "slides" => {
            r#"{"version","slide_level","width","height","title_slide","slides":[{"id","explicit","index","kind","title","summary","blocks","boxes","opaque","anchor","hidden"}]}"#
        }
        "show" => {
            r#"{"version","slide":{"id","explicit","index","kind","level","title","attrs":{"id","classes","kv"},"opaque","source","blocks":[{"id","explicit","kind","text","attrs","geom"?}]}}"#
        }
        "meta" => r#"{"version","front_matter":{...}}"#,
        "check" => {
            r#"{"ok","version","problems":[{"severity":"error|warning","code","slide","message"}]} or with --offline {"kind":"offline_check","ok","html","slides","steps","problems":[{"code":"P_*","severity","slide","message","hint","url"?}]}"#
        }
        "serve" => r#"{"url","port","pid"} (one line, then serves until stopped)"#,
        "url" => r#"{"url"}"#,
        "pdf" => r#"{"path","pages","per_slide","notes"}"#,
        "export" => {
            r#"{"kind":"export","ok","out","html","files":[...],"check":{offline_check},"assets":[{"name","version","sha256","source","path"}],"pdf"?:{"path","pages","per_slide","notes"},"staging"? (when ok is false)}"#
        }
        "browser" => r#"{"path","source":"env|installed|download","version"} or {"ok","removed"}"#,
        "render" => r#"{"ok","output":"<absolute path>","format","log"}"#,
        "stop" => r#"{"ok","pid"}"#,
        "settings" => {
            r#"{"values":{...},"defs":[{"key","label","description","type","values","default"}],"notices":[...],"path"}; set/reset: {"ok","values"}"#
        }
        "run" => {
            r#"{"ok","op":"run","version","mode":"live|fresh","status":"done|error|stopped","runs":[{"cell","counter","status":"done|error|stopped","outputs":[{"kind":"text","text"}|{"kind":"html","html"}|{"kind":"image","mime","path"}|{"kind":"error","ename","evalue","traceback"}]}]} (ok false: E_RUN on stderr, exit 1)"#
        }
        "outputs" => r#"{"outputs":[{"cell","counter","status","stale","outputs":[...as run]}]}"#,
        "focus" => {
            r#"{"open","age","version","deck_version","slide","elements":[...],"selection":{"text","where":"slide|source","truncated"}|null,"line","editing"} or {"open":false}; with --set {"ok","result":"moved|deferred|no_editor","ref"}"#
        }
        "env" => {
            r#"{"tool","root","manifest","lockfile","lock_current"} (+ "ok","log" for init/add/sync)"#
        }
        "schema" => r#"{"commands":[...],"errors":[...]}"#,
        "skill" => r#"{"skill":"<markdown>"} or {"ok","path"} with --install"#,
        "undo" | "redo" => r#"{"ok","op","undone|redone","version"}"#,
        "box" => {
            r#"{"ok","op","version","diff","assigned_ids","target":"<box id>","converted_units"?}"#
        }
        "new" => r#"{"ok","op":"new","path","version","target"}"#,
        _ => r#"{"ok","op","version","diff","assigned_ids","target","dry_run"?}"#,
    }
}

fn schema() -> Value {
    fn describe(c: &clap::Command, path: &str) -> Value {
        let args: Vec<Value> = c
            .get_arguments()
            .filter(|a| !matches!(a.get_id().as_str(), "help" | "version"))
            .map(|a| {
                json!({
                    "name": a.get_id().as_str(),
                    "long": a.get_long(),
                    "positional": a.is_positional(),
                    "required": a.is_required_set(),
                    "repeatable": matches!(a.get_action(), clap::ArgAction::Append),
                    "takes_value": a.get_action().takes_values(),
                    "global": a.is_global_set(),
                    "help": a.get_help().map(|h| h.to_string()),
                })
            })
            .collect();
        let subs: Vec<Value> = c
            .get_subcommands()
            .filter(|s| s.get_name() != "help")
            .map(|s| describe(s, &format!("{path}{} ", s.get_name())))
            .collect();
        let mut v = json!({"name": c.get_name(), "about": c.get_about().map(|a| a.to_string()), "args": args});
        if !subs.is_empty() {
            v["commands"] = json!(subs);
        }
        if !path.is_empty() {
            v["output"] = json!(output_of(path.split_whitespace().next().unwrap_or("")));
        }
        v
    }
    let mut v = describe(&Cli::command(), "");
    v["errors"] = json!([
        "E_NO_DECK",
        "E_PARSE",
        "E_NOT_FOUND",
        "E_AMBIGUOUS",
        "E_UNSAFE_EDIT",
        "E_CONFLICT",
        "E_INVALID_ARG",
        "E_NO_UNDO",
        "E_NO_QUARTO",
        "E_RENDER",
        "E_NO_SERVER",
        "E_USAGE",
        "E_IO",
        "E_NO_BROWSER",
        "E_DOWNLOAD",
        "E_OFFLINE",
        "E_OFFLINE_INCOMPATIBLE",
        "E_NO_ENV",
        "E_ENV_TOOL",
        "E_LOCK_STALE",
        "E_ENV",
        "E_NOT_CODE",
        "E_ENGINE",
        "E_RUN"
    ]);
    v["refs"] = json!({
        "slide": "explicit id | auto id | @N (1-based) | first | last",
        "element": "<slide>/<index> (0-based among the slide's blocks) | #id"
    });
    v
}

/// `qd run`: through the running server's live session (its outputs show in
/// the editor), else a fresh one-shot session run from the top.
fn run_cells(
    s: &Store,
    cells: &[String],
    all: bool,
    fresh: bool,
    timeout: Option<u64>,
) -> Result<Value> {
    let d = s.read()?;
    let mut r = if let Ok(session) = crate::server::running(s) {
        crate::kernel::plan(&d, cells, all, fresh)?; // refs checked here, errors as CLI codes
        let body =
            json!({"cells": cells, "all": all, "fresh": fresh, "wait": true, "timeout": timeout});
        let (status, text) = to_server(&session, "api/run", Some(&body))?;
        let mut v: Value = serde_json::from_str(&text).unwrap_or_default();
        if status != 200 || !v["runs"].is_array() {
            let error = match &v["error"] {
                Value::Null => {
                    json!({"code": "E_IO", "message": format!("qd serve answered {status}: {text}"),
                    "hint": "restart `qd serve` (it may be from an older qd)"})
                }
                e => e.clone(),
            };
            v = json!({"status": "failed", "error": error, "runs": []});
        }
        v["mode"] = json!("live");
        v
    } else {
        let plan = crate::kernel::plan(&d, cells, all, true)?;
        let out = crate::outputs::Outputs::new(s);
        let mut runs = vec![];
        let res = if plan.is_empty() {
            Ok("done") // nothing to run needs no environment
        } else {
            let dir = s.path.parent().unwrap_or(Path::new("."));
            let k = crate::kernel::Session::start(&crate::env::require(dir)?, dir)?;
            let t = timeout.map(std::time::Duration::from_secs);
            crate::kernel::run_plan(&k, &out, plan, t, |cell, rec| {
                runs.extend(rec.map(|rec| out.entry(cell, rec)));
            })
        };
        // a kernel failure mid-run still reports the cells that finished
        match res {
            Ok(status) => json!({"mode": "fresh", "status": status, "runs": runs}),
            Err(e) => {
                json!({"mode": "fresh", "status": "failed", "error": e.to_json()["error"], "runs": runs})
            }
        }
    };
    r["ok"] = json!(r["status"] == "done");
    r["op"] = json!("run");
    r["version"] = json!(d.version); // runs never change the file: ready for an --expect
    Ok(r)
}

/// POST `body` to the running server (its session file) with the run token:
/// to the port that answered, never a redirect, so the token goes nowhere else.
fn to_server(session: &Value, path: &str, body: Option<&Value>) -> Result<(u16, String)> {
    let agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .max_redirects(0)
        .build()
        .new_agent();
    let url = format!("http://127.0.0.1:{}/{path}", session["port"]);
    let token = session["token"].as_str().unwrap_or_default();
    let mut res = match body {
        Some(b) => agent
            .post(&url)
            .header("X-QD-Token", token)
            .header("Content-Type", "application/json")
            .send(b.to_string()),
        None => agent.get(&url).header("X-QD-Token", token).call(),
    }
    .map_err(std::io::Error::other)?;
    let text = res
        .body_mut()
        .read_to_string()
        .map_err(std::io::Error::other)?;
    Ok((res.status().as_u16(), text))
}

fn find_deck(arg: &Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = arg {
        return Ok(p.clone());
    }
    let qmds: Vec<PathBuf> = std::fs::read_dir(".")?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            let name = p.file_name().unwrap().to_string_lossy();
            // qd's own render copies (`<stem>.qd-<tag>.qmd`) are not decks
            p.extension().is_some_and(|e| e == "qmd")
                && !name.starts_with('.')
                && !name.contains(".qd-")
        })
        .collect();
    match qmds.as_slice() {
        [one] => Ok(one.clone()),
        [] => Err(QdError::NoDeck(
            "no .qmd file in the current directory".into(),
        )),
        many => Err(QdError::NoDeck(format!(
            "{} .qmd files in the current directory",
            many.len()
        ))),
    }
}

fn read_input(input: &Input) -> Result<Option<String>> {
    Ok(match (&input.md, &input.file) {
        (Some(m), _) if m == "-" => {
            let mut s = String::new();
            std::io::stdin().read_to_string(&mut s)?;
            Some(s)
        }
        (Some(m), _) => Some(m.clone()),
        (None, Some(f)) => Some(
            std::fs::read_to_string(f)
                .map_err(|e| QdError::InvalidArg(format!("{}: {e}", f.display())))?,
        ),
        (None, None) => None,
    })
}

fn required_input(input: &Input) -> Result<String> {
    read_input(input)?
        .ok_or_else(|| QdError::Usage("pass --md TEXT, --md - (stdin) or --file PATH".into()))
}

fn attrs_json(a: Option<&crate::attrs::Attrs>) -> Value {
    match a {
        None => Value::Null,
        Some(a) => json!({
            "id": a.id(),
            "classes": a.classes(),
            "kv": a.kv().into_iter().map(|(k, v)| (k.to_string(), Value::from(v))).collect::<serde_json::Map<_, _>>(),
        }),
    }
}

fn block_json(d: &Deck, b: &Block) -> Value {
    let text = d.slice(b.span);
    let mut v = json!({"id": b.id, "explicit": b.id_explicit, "kind": b.kind, "text": text,
        "hash": crate::deck::version_of(text.as_bytes()), "lines": d.lines(b.span), "attrs": attrs_json(b.attrs.as_ref())});
    if matches!(b.kind, BlockKind::Cell | BlockKind::Code) {
        // ```{python} (kind "cell", executable) or ```python (kind "code", shown) → "python"
        let info = text
            .lines()
            .next()
            .and_then(crate::deck::code_open)
            .map_or("", |(_, _, i)| i);
        v["lang"] = json!(
            info.trim()
                .trim_start_matches('{')
                .split(['}', ' ', ','])
                .next()
                .unwrap_or("")
        );
    }
    if b.kind == BlockKind::Box
        && let Some(a) = &b.attrs
    {
        // px values as numbers, anything else (%, em) as strings
        let g = ops::geom(a);
        let len = |l: Option<ops::Length>| match l {
            Some(l) if l.unit == "px" => json!(l.value),
            Some(l) => json!(format!("{}{}", l.value, l.unit)),
            None => Value::Null,
        };
        v["geom"] = json!({"left": len(g.left), "top": len(g.top), "width": len(g.width), "height": len(g.height),
            "right": len(g.right), "bottom": len(g.bottom)});
    }
    v
}

fn summary(d: &Deck, s: &Slide) -> String {
    let first = s.blocks.iter().find(|b| {
        !matches!(
            b.kind,
            BlockKind::Notes | BlockKind::Pause | BlockKind::Html
        )
    });
    let line = first
        .map(|b| {
            d.slice(b.span)
                .lines()
                .next()
                .unwrap_or("")
                .trim()
                .to_string()
        })
        .unwrap_or_default();
    if line.chars().count() > 80 {
        line.chars().take(79).collect::<String>() + "…"
    } else {
        line
    }
}

pub fn slides_json(d: &Deck) -> Value {
    let slides: Vec<Value> = d
        .slides
        .iter()
        .map(|s| {
            json!({
                "id": s.id, "explicit": s.id_explicit, "index": s.index, "kind": s.kind, "title": s.title,
                "summary": summary(d, s), "blocks": s.blocks.len(), "lines": d.lines(s.span),
                "boxes": s.blocks.iter().filter(|b| b.kind == BlockKind::Box).count(), "opaque": s.opaque,
                "anchor": crate::render::preview_id(s),
                "hidden": s.attrs.as_ref().and_then(|a| a.get("visibility")) == Some("hidden"),
            })
        })
        .collect();
    json!({"version": d.version, "slide_level": d.slide_level, "width": d.width, "height": d.height,
        "title_slide": d.has_title_slide(), "slides": slides})
}

pub fn show_json(d: &Deck, s: &Slide) -> Value {
    json!({"version": d.version, "slide": {
        "id": s.id, "explicit": s.id_explicit, "index": s.index, "kind": s.kind, "level": s.level, "title": s.title,
        "lines": d.lines(s.span),
        "title_hash": s.title.as_ref().map(|t| crate::deck::version_of(t.as_bytes())),
        "attrs": attrs_json(s.attrs.as_ref()), "opaque": s.opaque, "source": d.slice(s.span),
        "blocks": s.blocks.iter().map(|b| block_json(d, b)).collect::<Vec<_>>(),
    }})
}

/// Local paths referenced by images `![..](path)` and `{{< include path >}}`.
fn referenced_paths(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("](") {
        rest = &rest[i + 2..];
        let end = rest.find([')', ' ']).unwrap_or(rest.len());
        out.push(rest[..end].to_string());
    }
    for part in text.split("{{< include ").skip(1) {
        if let Some(p) = part.split_whitespace().next() {
            out.push(p.to_string());
        }
    }
    out.retain(|p| {
        !p.is_empty() && !p.contains("://") && !p.starts_with(['#', '{']) && !p.starts_with("data:")
    });
    out
}

fn check(d: &Deck, dir: &Path) -> Value {
    let mut problems = Vec::new();
    let ids = d.explicit_ids();
    let mut seen = std::collections::HashSet::new();
    for id in &ids {
        if !seen.insert(*id) {
            problems.push(json!({"severity": "error", "code": "E_AMBIGUOUS", "slide": null, "message": format!("duplicate id '{id}'")}));
        }
    }
    for s in &d.slides {
        if s.opaque {
            problems.push(json!({"severity": "warning", "code": "E_UNSAFE_EDIT", "slide": s.id,
                "message": "unterminated fence/div/math, or a slide heading inside a div; qd will not restructure this slide"}));
        }
        for b in &s.blocks {
            for p in referenced_paths(d.slice(b.span)) {
                if !dir.join(&p).exists() {
                    problems.push(json!({"severity": "warning", "code": "E_NOT_FOUND", "slide": s.id, "message": format!("missing file '{p}'")}));
                }
            }
        }
    }
    let ok = !problems.iter().any(|p| p["severity"] == "error");
    json!({"ok": ok, "version": d.version, "problems": problems})
}

struct Ctx<'a> {
    cli: &'a Cli,
    /// Set by `run_op` (server): use this store instead of resolving `--deck`.
    store: Option<&'a Store>,
}

impl Ctx<'_> {
    /// qd's cache root: the API's store's root, else the default.
    fn cache_root(&self) -> PathBuf {
        self.store
            .map_or_else(crate::store::cache_root, Store::root)
    }

    fn store(&self) -> Result<Store> {
        if let Some(s) = self.store {
            return Ok(s.clone());
        }
        Store::open(&find_deck(&self.cli.deck)?)
    }

    fn edit(&self, f: impl FnOnce(&Deck) -> Result<Patch>) -> Result<Value> {
        let store = self.store()?;
        let d = store.read()?;
        if let Some(e) = &self.cli.expect
            && e != &d.version
        {
            return Err(QdError::Conflict(format!(
                "expected version {e}, file is at {}",
                d.version
            )));
        }
        let p = f(&d)?;
        let after = ops::apply(&d.text, &p)?;
        let diff = ops::diff(&d.text, &after);
        let mut v = json!({"ok": true, "op": p.op, "diff": diff, "assigned_ids": p.assigned_ids, "target": p.target});
        if p.converted_units {
            v["converted_units"] = json!(true);
        }
        if self.cli.dry_run {
            v["dry_run"] = json!(true);
            v["version"] = json!(d.version);
        } else {
            v["version"] = json!(if after == d.text {
                d.version.clone()
            } else {
                store.write(&d, &after, &p.op)?
            });
        }
        Ok(v)
    }
}

/// Commands the HTTP API may run (FR-010). No server control, no file I/O
/// beyond the deck itself.
const API_OPS: &[&str] = &[
    "export",
    "pdf",
    "render",
    "box add",
    "box move",
    "box resize",
    "box rm",
    "replace",
    "block add",
    "block move",
    "block rm",
    "block dup",
    "block class",
    "slides",
    "show",
    "meta",
    "meta set",
    "meta unset",
    "check",
    "add",
    "dup",
    "move",
    "rm",
    "title",
    "body",
    "set",
    "edit",
    "undo",
    "redo",
];

/// Run a CLI command given as `{op, args}` (the body of `POST /api/op`).
/// `args` keys are the command's positional names and long flags; arrays
/// repeat a flag, `true` sets a switch. The command line is rebuilt and parsed
/// by clap, so the API cannot drift from the CLI.
pub fn run_op(store: &Store, op: &str, args: &Value, expect: Option<&str>) -> Result<Value> {
    if !API_OPS.contains(&op) {
        return Err(QdError::InvalidArg(format!(
            "unknown or disallowed op '{op}'"
        )));
    }
    let empty = serde_json::Map::new();
    let obj = match args {
        Value::Object(m) => m,
        Value::Null => &empty,
        _ => return Err(QdError::InvalidArg("args must be an object".into())),
    };
    let root = Cli::command();
    let mut cmd = &root;
    for w in op.split(' ') {
        cmd = cmd.find_subcommand(w).expect("API_OPS are subcommands");
    }
    let mut argv: Vec<String> = vec!["qd".into()];
    if let Some(e) = expect {
        argv.extend(["--expect".into(), e.into()]);
    }
    if obj.get("dry_run") == Some(&Value::Bool(true)) {
        argv.push("--dry-run".into());
    }
    argv.extend(op.split(' ').map(String::from));
    let scalar = |k: &str, v: &Value| -> Result<String> {
        match v {
            Value::String(s) => Ok(s.clone()),
            Value::Number(n) => Ok(n.to_string()),
            _ => Err(QdError::InvalidArg(format!(
                "'{k}' must be a string or number"
            ))),
        }
    };
    let mut known = vec!["dry_run".to_string()];
    let mut positionals: Vec<&clap::Arg> = cmd.get_positionals().collect();
    positionals.sort_by_key(|a| a.get_index());
    for a in positionals {
        let id = a.get_id().as_str();
        known.push(id.into());
        if let Some(v) = obj.get(id) {
            argv.push(scalar(id, v)?);
        }
    }
    for a in cmd
        .get_arguments()
        .filter(|a| !a.is_positional() && !a.is_global_set())
    {
        let Some(long) = a.get_long() else { continue };
        known.push(long.into());
        match obj.get(long) {
            None | Some(Value::Bool(false)) | Some(Value::Null) => {}
            Some(Value::Bool(true)) => argv.push(format!("--{long}")),
            Some(Value::Array(items)) => {
                for it in items {
                    argv.extend([format!("--{long}"), scalar(long, it)?]);
                }
            }
            Some(v) => argv.extend([format!("--{long}"), scalar(long, v)?]),
        }
    }
    if let Some(k) = obj.keys().find(|k| !known.contains(k)) {
        return Err(QdError::InvalidArg(format!(
            "unknown argument '{k}' for '{op}'"
        )));
    }
    // no reading or writing arbitrary paths through the API (002 T023 may allow
    // paths inside the deck folder)
    if ["html", "out"].iter().any(|k| obj.contains_key(*k)) {
        return Err(QdError::InvalidArg(
            "the API does not take file paths ('html'/'out')".into(),
        ));
    }
    let reads_input = ["file", "old-file", "new-file"]
        .iter()
        .any(|k| obj.contains_key(*k))
        || ["md", "old", "new"]
            .iter()
            .any(|k| obj.get(*k).is_some_and(|m| m == "-"));
    if reads_input {
        return Err(QdError::InvalidArg(
            "the API takes markdown inline via 'md' only".into(),
        ));
    }
    let cli = Cli::try_parse_from(&argv)
        .map_err(|e| QdError::Usage(e.render().to_string().trim().to_string()))?;
    run_with(&cli, Some(store))
}

pub fn run(cli: &Cli) -> Result<Value> {
    run_with(cli, None)
}

fn run_with(cli: &Cli, store: Option<&Store>) -> Result<Value> {
    let ctx = Ctx { cli, store };
    let read = || -> Result<(Store, Deck)> {
        let s = ctx.store()?;
        let d = s.read()?;
        Ok((s, d))
    };
    match &cli.cmd {
        Cmd::Slides => Ok(slides_json(&read()?.1)),
        Cmd::Show { slide } => {
            let d = read()?.1;
            Ok(show_json(&d, d.slide(slide)?))
        }
        Cmd::Meta { action: None } => {
            let d = read()?.1;
            Ok(
                json!({"version": d.version, "front_matter": serde_json::to_value(&d.meta).unwrap_or(Value::Null)}),
            )
        }
        Cmd::Meta {
            action: Some(MetaCmd::Set { key, value }),
        } => ctx.edit(|d| ops::meta_set(d, key, value)),
        Cmd::Meta {
            action: Some(MetaCmd::Unset { key }),
        } => ctx.edit(|d| ops::meta_unset(d, key)),
        Cmd::Check {
            offline: true,
            html,
            no_download,
            ..
        } => {
            let chrome = crate::browser::find(&ctx.cache_root(), allow_download(*no_download))?;
            let report = match html {
                Some(h) => crate::offline::check(h, &chrome)?,
                None => {
                    // render next to the deck so relative media paths resolve as they will for the user
                    let (s, d) = read()?;
                    let dir = s.path.parent().unwrap_or(Path::new(".")).to_path_buf();
                    let out = crate::render::render(&s.path, &d.text, "offline", &dir, false)?;
                    // report the deck that was checked; the temporary render is removed below
                    let res = crate::offline::check(&out, &chrome).map(|mut r| {
                        r.html = s.path.clone();
                        r
                    });
                    let _ = std::fs::remove_file(&out);
                    let _ = std::fs::remove_dir_all(out.with_file_name(format!(
                        "{}_files",
                        out.file_stem().unwrap_or_default().to_string_lossy()
                    )));
                    res?
                }
            };
            Ok(serde_json::to_value(report).unwrap())
        }
        Cmd::Export {
            out,
            pdf,
            notes,
            per_slide,
            no_download,
        } => crate::export::run(
            &ctx.store()?,
            &crate::export::Options {
                out: out.clone(),
                pdf: *pdf,
                notes: *notes,
                per_slide: *per_slide,
                allow_download: allow_download(*no_download),
            },
        ),
        Cmd::Pdf {
            html,
            out,
            notes,
            per_slide,
            no_download,
        } => {
            let html = match html {
                Some(h) => h.clone(),
                None => {
                    let s = ctx.store()?;
                    let stem = s
                        .path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    let h = s
                        .path
                        .parent()
                        .unwrap_or(Path::new("."))
                        .join("export")
                        .join(format!("{stem}.html"));
                    if !h.exists() {
                        return Err(QdError::NotFound(format!(
                            "no export at {}; run `qd export` first (or pass --html)",
                            h.display()
                        )));
                    }
                    h
                }
            };
            let out = out.clone().unwrap_or_else(|| html.with_extension("pdf"));
            let chrome = crate::browser::find(&ctx.cache_root(), allow_download(*no_download))?;
            Ok(serde_json::to_value(crate::offline::print_pdf(
                &html, &out, *notes, *per_slide, &chrome,
            )?)
            .unwrap())
        }
        Cmd::Browser {
            install,
            remove,
            no_download,
        } => {
            let root = crate::store::cache_root();
            if *remove {
                return Ok(json!({"ok": true, "removed": crate::browser::remove(&root)?}));
            }
            let b = if *install && crate::browser::cached(&root).is_none() {
                match crate::browser::find(&root, false) {
                    Ok(b) if b.source == "env" => b,
                    _ => crate::browser::install(&root)?,
                }
            } else {
                crate::browser::find(&root, *install || allow_download(*no_download))?
            };
            Ok(serde_json::to_value(b).unwrap())
        }
        Cmd::Check { render, .. } => {
            let (s, d) = read()?;
            let mut v = check(&d, s.path.parent().unwrap_or(Path::new(".")));
            if *render {
                let out = s.cache_dir().join("check");
                let res = crate::render::render(&s.path, &d.text, "check", &out, false);
                let _ = std::fs::remove_dir_all(&out);
                match res {
                    Ok(_) => {}
                    Err(QdError::Render(log)) => {
                        v["ok"] = json!(false);
                        v["problems"].as_array_mut().unwrap().push(
                            json!({"severity": "error", "code": "E_RENDER", "slide": null, "message": log}),
                        );
                    }
                    Err(e) => return Err(e),
                }
            }
            Ok(v)
        }
        Cmd::Serve { port, open, detach } => serve(&ctx.store()?, *port, *open, *detach),
        Cmd::Settings { action } => {
            let path = crate::settings::path();
            let values = match action {
                None => return Ok(crate::settings::report(&path)),
                Some(SettingsCmd::Set { key, value }) => {
                    let v = serde_json::from_str(value)
                        .unwrap_or_else(|_| Value::String(value.clone()));
                    crate::settings::set(&path, &serde_json::Map::from_iter([(key.clone(), v)]))?
                }
                Some(SettingsCmd::Reset) => crate::settings::reset(&path)?,
            };
            Ok(json!({"ok": true, "values": values}))
        }
        Cmd::Env { action } => {
            let s = ctx.store()?;
            let dir = s.path.parent().unwrap_or(Path::new("."));
            let env = match action {
                Some(EnvCmd::Init { tool }) => crate::env::init(dir, *tool)?,
                _ => crate::env::require(dir)?,
            };
            let log = match action {
                None => return Ok(env.report()),
                Some(EnvCmd::Init { .. }) => String::new(),
                Some(EnvCmd::Add { packages }) => env.add(packages)?,
                Some(EnvCmd::Sync) => env.sync()?,
            };
            let mut r = env.report();
            r["ok"] = json!(true);
            r["log"] = json!(log);
            Ok(r)
        }
        Cmd::Run {
            cells,
            all,
            fresh,
            timeout,
        } => run_cells(&ctx.store()?, cells, *all, *fresh, *timeout),
        Cmd::Focus { set } => {
            let s = ctx.store()?;
            // a bad ref is an error even with no editor to point
            let target = match set {
                Some(r) => Some(crate::focus::resolve(&s.read()?, r)?.1),
                None => None,
            };
            // no running editor is an answer, not an error (FR-003)
            let Ok(session) = crate::server::running(&s) else {
                return Ok(match target {
                    Some(r) => json!({"ok": true, "result": "no_editor", "ref": r}),
                    None => json!({"open": false}),
                });
            };
            let (status, text) = match &target {
                Some(r) => to_server(&session, "api/focus/set", Some(&json!({ "ref": r })))?,
                None => to_server(&session, "api/focus", None)?,
            };
            if status != 200 {
                return Err(QdError::Io(std::io::Error::other(format!(
                    "qd serve answered {status}: {text}; one from an older qd has no focus (restart it)"
                ))));
            }
            Ok(serde_json::from_str(&text).map_err(std::io::Error::other)?)
        }
        Cmd::Outputs { cells } => {
            let (s, d) = read()?;
            let out = crate::outputs::Outputs::new(&s);
            let ids: Vec<String> = cells
                .iter()
                .map(|r| d.element(r).map(|(_, b)| b.id.clone()))
                .collect::<Result<_>>()?;
            let stored = out.report(&d);
            let list: Vec<Value> = d
                .blocks()
                .filter(|b| ids.is_empty() || ids.contains(&b.id))
                .filter_map(|b| {
                    let rec = &stored["cells"][&b.id];
                    let mut e = out.entry(&b.id, rec.as_object().map(|_| rec)?);
                    e["stale"] = rec["stale"].clone();
                    Some(e)
                })
                .collect();
            Ok(json!({ "outputs": list }))
        }
        Cmd::Render { to } => {
            let s = ctx.store()?;
            let (output, log) = crate::render::render_deck(&s.path, to.as_deref())?;
            let format = to.clone().unwrap_or_else(|| "revealjs".into());
            Ok(json!({"ok": true, "op": "render", "output": output, "format": format, "log": log}))
        }
        Cmd::Url { slide } => {
            let (s, d) = read()?;
            let session = crate::server::running(&s)?;
            let (Some(url), Some(token)) = (session["url"].as_str(), session["token"].as_str())
            else {
                return Err(QdError::NoServer);
            };
            // the editor link: the token rides in the fragment, which the editor keeps
            Ok(match slide {
                Some(r) => {
                    let id = crate::render::preview_id(d.slide(r)?);
                    json!({"url": format!("{url}#/{id}&t={token}")})
                }
                None => json!({"url": format!("{url}#t={token}")}),
            })
        }
        Cmd::Stop => {
            // asks the server itself (token-guarded): never signals a pid that may be reused
            let store = ctx.store()?;
            let session = crate::server::running(&store)?;
            let (status, text) = to_server(&session, "api/shutdown", Some(&json!({})))?;
            if status != 200 {
                return Err(QdError::Io(std::io::Error::other(format!(
                    "qd serve refused to stop ({status}: {text}); one from an older qd stops with Ctrl-C"
                ))));
            }
            // returns once it is gone (it removes its session file last, after any
            // render), so a `qd serve` right after never overlaps it
            let file = store.cache_dir().join("session.json");
            let end = Instant::now() + Duration::from_secs(60);
            while file.exists() {
                if Instant::now() > end {
                    return Err(QdError::Io(std::io::Error::other(
                        "qd serve is still finishing a render; run `qd stop` again before `qd serve`",
                    )));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Ok(json!({"ok": true, "pid": session["pid"]}))
        }
        Cmd::Schema => Ok(schema()),
        Cmd::Skill { install: None } => Ok(json!({"skill": SKILL})),
        Cmd::Skill { install: Some(dir) } => {
            std::fs::create_dir_all(dir)?;
            let path = dir.join("SKILL.md");
            std::fs::write(&path, SKILL)?;
            Ok(json!({"ok": true, "path": path}))
        }
        Cmd::Add {
            after,
            before,
            title,
            input,
            attrs,
            classes,
        } => {
            let spec = SlideSpec {
                title: title.clone(),
                body: read_input(input)?,
                attrs: attrs.clone(),
                classes: classes.clone(),
            };
            let pos = match (after, before) {
                (Some(a), _) => Pos::After(a),
                (_, Some(b)) => Pos::Before(b),
                _ => Pos::End,
            };
            ctx.edit(|d| ops::add(d, &spec, pos))
        }
        Cmd::Dup { slide, after } => ctx.edit(|d| ops::dup(d, slide, after.as_deref())),
        Cmd::Move { slide, to } => ctx.edit(|d| ops::mv(d, slide, to.pos())),
        Cmd::Rm { slide } => ctx.edit(|d| ops::rm(d, slide)),
        Cmd::Title {
            slide,
            text,
            expect_text,
        } => ctx.edit(|d| ops::title(d, slide, text, expect_text.as_deref())),
        Cmd::Body { slide, input } => {
            let md = required_input(input)?;
            ctx.edit(|d| ops::body(d, slide, &md))
        }
        Cmd::Set {
            slide,
            attrs,
            unset,
            classes,
            unclass,
        } => {
            let change = AttrChange {
                set: attrs.clone(),
                unset: unset.clone(),
                add_class: classes.clone(),
                remove_class: unclass.clone(),
            };
            ctx.edit(|d| ops::set(d, slide, &change))
        }
        Cmd::Edit {
            element,
            input,
            expect_text,
        } => {
            let md = required_input(input)?;
            ctx.edit(|d| ops::edit(d, element, &md, expect_text.as_deref()))
        }
        Cmd::Box { action } => match action {
            BoxCmd::Add {
                slide,
                source,
                at,
                size,
            } => {
                let content = match (&source.text, &source.image) {
                    (Some(t), _) => BoxContent::Text(t),
                    (_, Some(i)) => BoxContent::Image(i),
                    _ => unreachable!("clap group is required"),
                };
                ctx.edit(|d| ops::box_add(d, slide, content, *at, *size))
            }
            BoxCmd::Move { bx, at, by } => ctx.edit(|d| ops::box_move(d, bx, *at, *by)),
            BoxCmd::Resize { bx, size, at } => {
                ctx.edit(|d| ops::box_resize(d, bx, size.unwrap(), *at))
            }
            BoxCmd::Rm { bx } => ctx.edit(|d| ops::box_rm(d, bx)),
        },
        Cmd::Replace {
            old,
            old_file,
            new,
            new_file,
        } => {
            let old = read_input(&Input {
                md: old.clone(),
                file: old_file.clone(),
            })?
            .unwrap_or_default();
            let new = read_input(&Input {
                md: new.clone(),
                file: new_file.clone(),
            })?
            .unwrap_or_default();
            ctx.edit(|d| ops::replace(d, &old, &new))
        }
        Cmd::Block { action } => match action {
            BlockCmd::Add { slide, at, input } => {
                let md = required_input(input)?;
                let first = format!("{slide}/0"); // --start: before the first element
                let pos = match (&at.after, &at.before, at.start) {
                    (Some(r), _, _) => Pos::After(r),
                    (_, Some(r), _) => Pos::Before(r),
                    (_, _, true) => Pos::Before(&first),
                    _ => Pos::End,
                };
                ctx.edit(|d| ops::block_add(d, slide, pos, &md))
            }
            BlockCmd::Move { element, to } => ctx.edit(|d| ops::block_move(d, element, to.pos())),
            BlockCmd::Rm { element } => ctx.edit(|d| ops::block_rm(d, element)),
            BlockCmd::Dup { element } => ctx.edit(|d| ops::block_dup(d, element)),
            BlockCmd::Class {
                element,
                add,
                remove,
            } => ctx.edit(|d| ops::block_class(d, element, add, remove)),
        },
        Cmd::Undo => {
            let (v, op) = ctx.store()?.undo()?;
            Ok(json!({"ok": true, "op": "undo", "undone": op, "version": v}))
        }
        Cmd::Redo => {
            let (v, op) = ctx.store()?.redo()?;
            Ok(json!({"ok": true, "op": "redo", "redone": op, "version": v}))
        }
        Cmd::New { path, title } => {
            if path.exists() {
                return Err(QdError::InvalidArg(format!(
                    "{} already exists",
                    path.display()
                )));
            }
            if let Some(dir) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
                std::fs::create_dir_all(dir)?;
            }
            let text = ops::new_deck(title.as_deref());
            std::fs::write(path, &text)?;
            let d = Deck::parse(text);
            Ok(
                json!({"ok": true, "op": "new", "path": path, "version": d.version, "target": d.slides[0].id}),
            )
        }
    }
}

/// Downloads are allowed unless `--no-download` or `QD_NO_DOWNLOAD` (non-empty, not "0").
fn allow_download(no_download: bool) -> bool {
    !no_download && std::env::var("QD_NO_DOWNLOAD").map_or(true, |v| v.is_empty() || v == "0")
}

fn open_browser(url: &str) {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = std::process::Command::new(opener).arg(url).status();
}

fn serve(store: &Store, port: u16, open: bool, detach: bool) -> Result<Value> {
    crate::render::quarto()?;
    if detach {
        use std::io::BufRead;
        let mut child = std::process::Command::new(std::env::current_exe()?)
            .arg("--deck")
            .arg(&store.path)
            .args(["serve", "--port", &port.to_string()])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()?;
        let mut line = String::new();
        std::io::BufReader::new(child.stdout.take().unwrap()).read_line(&mut line)?;
        let v: Value = serde_json::from_str(&line).map_err(|_| {
            QdError::Io(std::io::Error::other(
                "preview server failed to start (port in use?)",
            ))
        })?;
        if open {
            open_browser(v["url"].as_str().unwrap_or_default());
        }
        return Ok(v);
    }
    crate::server::serve(Store::open(&store.path)?, port, |s| {
        use std::io::Write;
        let line = json!({"url": s.editor, "port": s.port, "pid": std::process::id()});
        let mut out = std::io::stdout();
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
        if open {
            open_browser(&s.editor);
        }
    })?;
    Ok(json!({"ok": true, "stopped": true}))
}

fn human(v: &Value) -> String {
    if let Some(slides) = v["slides"].as_array() {
        return slides
            .iter()
            .map(|s| {
                format!(
                    "{:>3}  {:<24} {}{}\n",
                    s["index"],
                    s["id"].as_str().unwrap_or(""),
                    s["title"].as_str().unwrap_or("(untitled)"),
                    match (s["opaque"] == true, s["hidden"] == true) {
                        (true, _) => "  [opaque]",
                        (_, true) => "  [hidden]",
                        _ => "",
                    }
                )
            })
            .collect();
    }
    if let Some(src) = v["slide"]["source"].as_str() {
        return src.to_string();
    }
    if let Some(diff) = v["diff"].as_str() {
        return format!(
            "{} {} → {}\n{diff}",
            v["op"].as_str().unwrap_or(""),
            v["target"].as_str().unwrap_or(""),
            v["version"]
        );
    }
    if let Some(skill) = v["skill"].as_str() {
        return skill.to_string();
    }
    serde_json::to_string_pretty(v).unwrap() + "\n"
}

/// Entry point used by `main`: returns the process exit code.
pub fn main() -> i32 {
    let cli = match Cli::try_parse() {
        Ok(c) => c,
        Err(e)
            if matches!(
                e.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) =>
        {
            let _ = e.print();
            return 0;
        }
        Err(e) => {
            let err = QdError::Usage(e.render().to_string().trim().to_string());
            eprintln!("{}", err.to_json());
            return err.exit_code();
        }
    };
    match run(&cli) {
        Ok(v) => {
            use std::io::Write;
            // a detached server's stdout pipe may be gone: ignore write errors
            let mut out = std::io::stdout();
            let _ = if cli.human {
                write!(out, "{}", human(&v))
            } else {
                writeln!(out, "{v}")
            };
            // a run that stopped at a cell: the result is on stdout, the error on stderr
            if v["op"] == "run" && v["ok"] == false {
                if !v["error"].is_null() {
                    // the kernel or server failed (e.g. E_NO_ENV): its own error
                    eprintln!("{}", json!({"error": v["error"]}));
                    return 1;
                }
                let last = v["runs"].as_array().and_then(|r| r.last());
                let e = QdError::Run(
                    last.and_then(|c| c["cell"].as_str())
                        .unwrap_or("a cell")
                        .into(),
                );
                eprintln!("{}", e.to_json());
                return e.exit_code();
            }
            // a failed offline check: the report is on stdout, the error on stderr
            if (v["kind"] == "offline_check" || v["kind"] == "export") && v["ok"] == false {
                let probs = if v["kind"] == "export" {
                    &v["check"]["problems"]
                } else {
                    &v["problems"]
                };
                let n = probs
                    .as_array()
                    .map_or(0, |p| p.iter().filter(|p| p["severity"] == "error").count());
                let e = QdError::Offline(n);
                eprintln!("{}", e.to_json());
                return e.exit_code();
            }
            0
        }
        Err(e) => {
            eprintln!("{}", e.to_json());
            e.exit_code()
        }
    }
}
