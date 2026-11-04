//! `qd` command line (contracts/cli.md). JSON on stdout, JSON errors on stderr.

use crate::deck::{Block, BlockKind, Deck, Slide};
use crate::error::{QdError, Result};
use crate::ops::{self, AttrChange, Patch, Pos, SlideSpec};
use crate::store::Store;
use clap::{Args, CommandFactory, Parser, Subcommand};
use serde_json::{Value, json};
use std::io::Read;
use std::path::{Path, PathBuf};

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
            r#"{"version","slide_level","title_slide","slides":[{"id","explicit","index","kind","title","summary","blocks","boxes","opaque","anchor"}]}"#
        }
        "show" => {
            r#"{"version","slide":{"id","explicit","index","kind","level","title","attrs":{"id","classes","kv"},"opaque","source","blocks":[{"id","explicit","kind","text","attrs"}]}}"#
        }
        "meta" => r#"{"version","front_matter":{...}}"#,
        "check" => {
            r#"{"ok","version","problems":[{"severity":"error|warning","code","slide","message"}]}"#
        }
        "serve" => r#"{"url","port","pid"} (one line, then serves until stopped)"#,
        "url" => r#"{"url"}"#,
        "stop" => r#"{"ok","pid"}"#,
        "schema" => r#"{"commands":[...],"errors":[...]}"#,
        "skill" => r#"{"skill":"<markdown>"} or {"ok","path"} with --install"#,
        "undo" | "redo" => r#"{"ok","op","undone|redone","version"}"#,
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
        "E_IO"
    ]);
    v["refs"] = json!({
        "slide": "explicit id | auto id | @N (1-based) | first | last",
        "element": "<slide>/<index> (0-based among the slide's blocks) | #id"
    });
    v
}

fn find_deck(arg: &Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = arg {
        return Ok(p.clone());
    }
    let qmds: Vec<PathBuf> = std::fs::read_dir(".")?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.extension().is_some_and(|e| e == "qmd")
                && !p.file_name().unwrap().to_string_lossy().starts_with('.')
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
    json!({"id": b.id, "explicit": b.id_explicit, "kind": b.kind, "text": d.slice(b.span), "attrs": attrs_json(b.attrs.as_ref())})
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
                "summary": summary(d, s), "blocks": s.blocks.len(),
                "boxes": s.blocks.iter().filter(|b| b.kind == BlockKind::Box).count(), "opaque": s.opaque,
                "anchor": crate::render::preview_id(s),
            })
        })
        .collect();
    json!({"version": d.version, "slide_level": d.slide_level, "title_slide": d.has_title_slide(), "slides": slides})
}

pub fn show_json(d: &Deck, s: &Slide) -> Value {
    json!({"version": d.version, "slide": {
        "id": s.id, "explicit": s.id_explicit, "index": s.index, "kind": s.kind, "level": s.level, "title": s.title,
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
    if obj.contains_key("file") || obj.get("md").is_some_and(|m| m == "-") {
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
        Cmd::Check { render } => {
            let (s, d) = read()?;
            let mut v = check(&d, s.path.parent().unwrap_or(Path::new(".")));
            if *render {
                let out = s.cache_dir().join("check");
                let res = crate::render::render(&s.path, &d.text, "check", &out);
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
        Cmd::Url { slide } => {
            let (s, d) = read()?;
            let url = crate::server::running_url(&s)?;
            Ok(match slide {
                Some(r) => {
                    json!({"url": format!("{url}#/{}", crate::render::preview_id(d.slide(r)?))})
                }
                None => json!({"url": url}),
            })
        }
        Cmd::Stop => {
            let pid = crate::server::running_pid(&ctx.store()?)?;
            std::process::Command::new("kill")
                .arg(pid.to_string())
                .status()?;
            Ok(json!({"ok": true, "pid": pid}))
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
        Cmd::Move { slide, to } => {
            let pos = match (&to.after, &to.before) {
                (Some(a), _) => Pos::After(a),
                (_, Some(b)) => Pos::Before(b),
                _ => unreachable!("clap group is required"),
            };
            ctx.edit(|d| ops::mv(d, slide, pos))
        }
        Cmd::Rm { slide } => ctx.edit(|d| ops::rm(d, slide)),
        Cmd::Title { slide, text } => ctx.edit(|d| ops::title(d, slide, text)),
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
        Cmd::Edit { element, input } => {
            let md = required_input(input)?;
            ctx.edit(|d| ops::edit(d, element, &md))
        }
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
        let line = json!({"url": s.url, "port": s.port, "pid": std::process::id()});
        let mut out = std::io::stdout();
        let _ = writeln!(out, "{line}");
        let _ = out.flush();
        if open {
            open_browser(&s.url);
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
                    if s["opaque"] == true {
                        "  [opaque]"
                    } else {
                        ""
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
            0
        }
        Err(e) => {
            eprintln!("{}", e.to_json());
            e.exit_code()
        }
    }
}
