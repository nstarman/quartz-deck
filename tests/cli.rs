//! CLI contract tests (contracts/cli.md): JSON on stdout, JSON errors on stderr,
//! stable codes and exit statuses, file untouched on error/dry-run.

use assert_cmd::Command;
use serde_json::{Value, json};
use std::fs;
use std::path::PathBuf;

const BASIC: &str = include_str!("fixtures/basic.qmd");

struct Env {
    dir: tempfile::TempDir,
}

impl Env {
    fn new() -> Env {
        Env::with(BASIC)
    }
    fn with(text: &str) -> Env {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("deck.qmd"), text).unwrap();
        Env { dir }
    }
    fn path(&self) -> PathBuf {
        self.dir.path().join("deck.qmd")
    }
    /// Store using the same cache root the CLI gets via QD_CACHE_DIR.
    fn store(&self) -> quartz_deck::store::Store {
        quartz_deck::store::Store::open_in(&self.path(), &self.dir.path().join(".cache")).unwrap()
    }
    fn text(&self) -> String {
        fs::read_to_string(self.path()).unwrap()
    }
    fn cmd(&self) -> Command {
        let mut c = Command::cargo_bin("qd").unwrap();
        c.current_dir(self.dir.path())
            .env("QD_CACHE_DIR", self.dir.path().join(".cache"))
            .env("QD_CONFIG_DIR", self.dir.path().join(".config"))
            .env_remove("QD_DECK");
        c
    }
    /// Run, expect success, parse stdout JSON.
    fn ok(&self, args: &[&str]) -> Value {
        let out = self.cmd().args(args).output().unwrap();
        assert!(
            out.status.success(),
            "qd {args:?} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice(&out.stdout).unwrap_or_else(|e| {
            panic!(
                "stdout not JSON ({e}): {}",
                String::from_utf8_lossy(&out.stdout)
            )
        })
    }
    /// Run, expect failure with `code` and exit `status`; file must be unchanged.
    fn err(&self, args: &[&str], code: &str, status: i32) -> Value {
        let before = fs::read_to_string(self.path()).ok();
        let out = self.cmd().args(args).output().unwrap();
        assert_eq!(
            out.status.code(),
            Some(status),
            "qd {args:?}: {}",
            String::from_utf8_lossy(&out.stdout)
        );
        assert!(out.stdout.is_empty());
        let v: Value = serde_json::from_slice(&out.stderr).unwrap();
        assert_eq!(v["error"]["code"], code, "{v}");
        assert!(v["error"]["hint"].as_str().is_some_and(|h| !h.is_empty()));
        assert_eq!(
            fs::read_to_string(self.path()).ok(),
            before,
            "file changed on error"
        );
        v
    }
}

fn assert_edit(v: &Value, op: &str) {
    assert_eq!(v["ok"], true);
    assert_eq!(v["op"], op);
    assert_eq!(v["version"].as_str().unwrap().len(), 16);
    assert!(
        v["diff"].is_string() && v["assigned_ids"].is_array() && v["target"].is_string(),
        "{v}"
    );
}

#[test]
fn read_commands() {
    let e = Env::new();
    let v = e.ok(&["slides"]);
    assert_eq!(v["slides"].as_array().unwrap().len(), 4);
    let s0 = &v["slides"][0];
    for k in [
        "id", "explicit", "index", "kind", "title", "summary", "blocks", "boxes", "opaque",
    ] {
        assert!(s0.get(k).is_some(), "missing {k} in {s0}");
    }
    assert_eq!(s0["id"], "intro");
    assert_eq!(s0["summary"], "Hello world.");
    assert_eq!(v["slides"][1]["boxes"], 1);
    assert_eq!(v["title_slide"], true);

    for s in v["slides"].as_array().unwrap() {
        assert_eq!(s["hidden"], false);
    }
    e.ok(&["set", "intro", "--attr", "visibility=hidden"]);
    assert_eq!(e.ok(&["slides"])["slides"][0]["hidden"], true);
    e.ok(&["undo"]);

    let v = e.ok(&["show", "results"]);
    assert_eq!(v["slide"]["attrs"]["classes"][0], "smaller");
    assert_eq!(v["slide"]["blocks"][1]["id"], "b-plot");
    assert_eq!(v["slide"]["blocks"][1]["kind"], "box");
    assert!(
        v["slide"]["source"]
            .as_str()
            .unwrap()
            .starts_with("## Results")
    );

    let v = e.ok(&["meta"]);
    assert_eq!(v["front_matter"]["title"], "Fixture");

    let v = e.ok(&["check"]);
    assert_eq!(v["ok"], true);
    assert_eq!(v["problems"].as_array().unwrap().len(), 1, "{v}"); // plot.png missing
    assert_eq!(v["problems"][0]["severity"], "warning");
}

#[test]
fn edit_commands() {
    let e = Env::new();
    let v = e.ok(&[
        "add",
        "--after",
        "intro",
        "--title",
        "Middle",
        "--md",
        "Mid body.",
        "--attr",
        "background-color=red",
        "--class",
        "center",
    ]);
    assert_edit(&v, "add");
    let id = v["target"].as_str().unwrap().to_string();
    assert!(e.text().contains(&format!(
        "## Middle {{#{id} .center background-color=red}}\n\nMid body."
    )));

    assert_edit(&e.ok(&["dup", "results"]), "dup");
    assert_edit(&e.ok(&["move", &id, "--after", "last"]), "move");
    assert_eq!(
        e.ok(&["slides"])["slides"]
            .as_array()
            .unwrap()
            .last()
            .unwrap()["id"],
        id.as_str()
    );
    assert_edit(&e.ok(&["rm", &id]), "rm");
    assert!(!e.text().contains("Middle"));
    assert_edit(&e.ok(&["title", "intro", "Introduction"]), "title");
    assert!(e.text().contains("## Introduction {#intro}"));
    assert_edit(&e.ok(&["body", "intro", "--md", "New intro."]), "body");
    assert!(e.text().contains("{#intro}\n\nNew intro.\n"));
    assert_edit(
        &e.ok(&[
            "set",
            "intro",
            "--attr",
            "transition=fade",
            "--class",
            "smaller",
        ]),
        "set",
    );
    assert!(
        e.text()
            .contains("## Introduction {#intro .smaller transition=fade}")
    );
    assert_edit(
        &e.ok(&[
            "set",
            "intro",
            "--unset",
            "transition",
            "--unclass",
            "smaller",
        ]),
        "set",
    );
    assert!(e.text().contains("## Introduction {#intro}\n"));
    assert_edit(&e.ok(&["edit", "intro/0", "--md", "Edited."]), "edit");
    assert!(e.text().contains("Edited."));
    assert_edit(&e.ok(&["meta", "set", "theme", "dark"]), "meta");
    assert!(e.text().contains("theme: dark\n"));
    assert_edit(&e.ok(&["meta", "unset", "theme"]), "meta");
    assert!(!e.text().contains("theme:"));
}

#[test]
fn body_from_file_and_stdin() {
    let e = Env::new();
    fs::write(e.dir.path().join("b.md"), "From file.\n").unwrap();
    e.ok(&["body", "intro", "--file", "b.md"]);
    assert!(e.text().contains("From file."));
    let out = e
        .cmd()
        .args(["body", "intro", "--md", "-"])
        .write_stdin("From stdin.\n")
        .output()
        .unwrap();
    assert!(out.status.success());
    assert!(e.text().contains("From stdin."));
    e.err(&["body", "intro"], "E_USAGE", 2);
    // markdown lists start with '-' and must not be parsed as flags
    e.ok(&["body", "intro", "--md", "- a\n- b"]);
    e.ok(&["add", "--title", "-dash", "--md", "- x"]);
    assert!(
        e.text().contains("{#intro}\n\n- a\n- b\n") && e.text().contains("## -dash {#dash}\n\n- x"),
        "{}",
        e.text()
    );
}

#[test]
fn dry_run_and_expect() {
    let e = Env::new();
    let v = e.ok(&["--dry-run", "title", "intro", "X"]);
    assert_eq!(v["dry_run"], true);
    assert!(v["diff"].as_str().unwrap().contains("+## X {#intro}"));
    assert_eq!(e.text(), BASIC);

    let version = e.ok(&["slides"])["version"].as_str().unwrap().to_string();
    e.err(
        &["title", "intro", "X", "--expect", "0000000000000000"],
        "E_CONFLICT",
        1,
    );
    let v = e.ok(&["title", "intro", "X", "--expect", &version]);
    assert_ne!(v["version"], version.as_str());
}

#[test]
fn errors() {
    let e = Env::new();
    e.err(&["show", "nope"], "E_NOT_FOUND", 1);
    e.err(&["title", "nope", "x"], "E_NOT_FOUND", 1);
    e.err(&["set", "intro", "--attr", "novalue"], "E_USAGE", 2);
    e.err(&["frobnicate"], "E_USAGE", 2);
    e.err(&["move", "intro"], "E_USAGE", 2);
    e.err(&["undo"], "E_NO_UNDO", 1);
    fs::write(e.path(), include_str!("corpus/21-unterminated.qmd")).unwrap();
    e.err(&["rm", "broken-slide"], "E_UNSAFE_EDIT", 1);
}

#[test]
fn undo_redo() {
    let e = Env::new();
    e.ok(&["title", "intro", "Changed"]);
    let changed = e.text();
    let v = e.ok(&["undo"]);
    assert_eq!(
        (v["ok"].clone(), v["undone"].clone()),
        (Value::Bool(true), Value::from("title"))
    );
    assert_eq!(e.text(), BASIC);
    e.ok(&["redo"]);
    assert_eq!(e.text(), changed);
    e.err(&["redo"], "E_NO_UNDO", 1);
}

#[test]
fn deck_discovery() {
    let e = Env::new();
    fs::rename(e.path(), e.dir.path().join("talk.qmd")).unwrap();
    let v = e.ok(&["slides"]);
    assert_eq!(v["slides"].as_array().unwrap().len(), 4);
    fs::write(e.dir.path().join("other.qmd"), "## X\n").unwrap();
    let out = e.cmd().arg("slides").output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("E_NO_DECK"));
    let v = e.ok(&["--deck", "other.qmd", "slides"]);
    assert_eq!(v["slides"][0]["id"], "x");
    let out = e
        .cmd()
        .env("QD_DECK", "other.qmd")
        .arg("slides")
        .output()
        .unwrap();
    assert!(out.status.success());
}

#[test]
fn new_deck() {
    let e = Env::new();
    let v = e.ok(&["new", "fresh/demo.qmd", "--title", "Demo"]);
    assert_eq!(v["ok"], true);
    let text = fs::read_to_string(e.dir.path().join("fresh/demo.qmd")).unwrap();
    assert!(text.contains("title: \"Demo\"") && text.contains("format: revealjs"));
    e.err(&["new", "deck.qmd"], "E_INVALID_ARG", 1);
}

#[test]
fn human_output() {
    let e = Env::new();
    let out = e.cmd().args(["--human", "slides"]).output().unwrap();
    let s = String::from_utf8(out.stdout).unwrap();
    assert!(
        s.contains("intro") && s.contains("Results") && serde_json::from_str::<Value>(&s).is_err()
    );
}

fn schema_commands(e: &Env) -> Vec<String> {
    let v = e.ok(&["schema"]);
    let mut names = Vec::new();
    fn walk(v: &Value, prefix: &str, out: &mut Vec<String>) {
        for c in v["commands"].as_array().into_iter().flatten() {
            let name = format!("{prefix}{}", c["name"].as_str().unwrap());
            out.push(name.clone());
            walk(c, &format!("{name} "), out);
        }
    }
    walk(&v, "", &mut names);
    assert!(
        v["errors"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c == "E_CONFLICT")
    );
    names
}

#[test]
fn schema_lists_every_command() {
    let e = Env::new();
    let names = schema_commands(&e);
    for c in [
        "slides",
        "show",
        "meta",
        "meta set",
        "meta unset",
        "check",
        "schema",
        "skill",
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
        "new",
        "box add",
        "box move",
        "box resize",
        "box rm",
    ] {
        assert!(
            names.contains(&c.to_string()),
            "schema missing '{c}': {names:?}"
        );
    }
}

/// FR-014: every `qd <cmd>` named in the Skill exists in the schema.
#[test]
fn skill_matches_cli() {
    let e = Env::new();
    let names = schema_commands(&e);
    let skill = e.ok(&["skill"])["skill"].as_str().unwrap().to_string();
    let mut mentioned = 0;
    for (i, _) in skill.match_indices("qd ") {
        let rest = &skill[i + 3..];
        let word: String = rest
            .chars()
            .take_while(|c| c.is_ascii_lowercase())
            .collect();
        if word.is_empty() {
            continue;
        }
        let two: String = rest
            .split_whitespace()
            .take(2)
            .collect::<Vec<_>>()
            .join(" ");
        mentioned += 1;
        assert!(
            names.contains(&word) || names.contains(&two),
            "Skill mentions unknown command 'qd {word}'"
        );
    }
    assert!(mentioned >= 10, "Skill should demonstrate the commands");
    let dir = e.dir.path().join("skills/quartz-deck");
    let v = e.ok(&["skill", "--install", dir.to_str().unwrap()]);
    assert_eq!(v["ok"], true);
    assert_eq!(fs::read_to_string(dir.join("SKILL.md")).unwrap(), skill);
}

/// SC-004: < 200 ms on a 100-slide deck. Run with `cargo test --release -- --ignored`.
#[test]
#[ignore]
fn perf_100_slides() {
    let e = Env::new();
    let mut text = String::from("---\ntitle: Big\nformat: revealjs\n---\n\n");
    for i in 0..100 {
        text += &format!("## Slide {i}\n\nParagraph {i} with some text.\n\n- a\n- b\n\n");
    }
    fs::write(e.path(), &text).unwrap();
    for args in [&["slides"][..], &["move", "slide-0", "--after", "last"][..]] {
        let t = std::time::Instant::now();
        e.ok(args);
        let ms = t.elapsed().as_millis();
        assert!(ms < 200, "qd {args:?} took {ms} ms");
    }
}

/// FR-010 / T033: `run_op` (behind POST /api/op) gives byte-identical JSON and
/// file results to the CLI for every editing op.
#[test]
fn run_op_matches_cli() {
    use quartz_deck::cli::run_op;
    use serde_json::json;
    let cases: Vec<(&[&str], &str, Value)> = vec![
        (
            &[
                "add", "--after", "intro", "--title", "Mid", "--md", "- a", "--attr", "x=1",
                "--class", "c",
            ],
            "add",
            json!({"after": "intro", "title": "Mid", "md": "- a", "attr": ["x=1"], "class": ["c"]}),
        ),
        (&["dup", "results"], "dup", json!({"slide": "results"})),
        (
            &["move", "intro", "--after", "last"],
            "move",
            json!({"slide": "intro", "after": "last"}),
        ),
        (&["rm", "@3"], "rm", json!({"slide": "@3"})),
        (
            &["title", "intro", "-New"],
            "title",
            json!({"slide": "intro", "text": "-New"}),
        ),
        (
            &["body", "intro", "--md", "Body."],
            "body",
            json!({"slide": "intro", "md": "Body."}),
        ),
        (
            &["set", "results", "--attr", "a=b", "--unclass", "smaller"],
            "set",
            json!({"slide": "results", "attr": ["a=b"], "unclass": ["smaller"]}),
        ),
        (
            &["edit", "results/0", "--md", "- z"],
            "edit",
            json!({"element": "results/0", "md": "- z"}),
        ),
        (
            &["meta", "set", "theme", "dark"],
            "meta set",
            json!({"key": "theme", "value": "dark"}),
        ),
        (&["show", "results"], "show", json!({"slide": "results"})),
        (
            &[
                "box", "add", "intro", "--text", "- n", "--at", "10,20", "--size", "30,40",
            ],
            "box add",
            json!({"slide": "intro", "text": "- n", "at": "10,20", "size": "30,40"}),
        ),
        (
            &["box", "move", "#b-plot", "--by", "-5,5"],
            "box move",
            json!({"box": "#b-plot", "by": "-5,5"}),
        ),
        (
            &["box", "resize", "#b-plot", "--size", "1,2", "--at", "3,4"],
            "box resize",
            json!({"box": "#b-plot", "size": "1,2", "at": "3,4"}),
        ),
        (
            &["box", "rm", "#b-plot"],
            "box rm",
            json!({"box": "#b-plot"}),
        ),
        (&["slides"], "slides", json!({})),
    ];
    for (argv, op, args) in cases {
        let a = Env::new();
        let b = Env::new();
        let cli = a.ok(argv);
        let api = run_op(&b.store(), op, &args, None).unwrap_or_else(|e| panic!("{op}: {e}"));
        assert_eq!(cli, api, "{op}: JSON differs");
        assert_eq!(a.text(), b.text(), "{op}: file differs");
    }
    // undo through the API uses the same journal as the CLI
    let e = Env::new();
    e.ok(&["title", "intro", "Changed"]);
    let v = run_op(&e.store(), "undo", &json!({}), None).unwrap();
    assert_eq!(v["undone"], "title");
    assert_eq!(e.text(), BASIC);
}

#[test]
fn run_op_rejects_unsafe_or_unknown() {
    use quartz_deck::cli::run_op;
    use serde_json::json;
    let e = Env::new();
    let p = e.store();
    let code = |op: &str, args: Value| run_op(&p, op, &args, None).unwrap_err().code();
    assert_eq!(code("serve", json!({})), "E_INVALID_ARG");
    assert_eq!(code("new", json!({"path": "x.qmd"})), "E_INVALID_ARG");
    assert_eq!(code("skill", json!({"install": "/tmp/x"})), "E_INVALID_ARG");
    assert_eq!(code("frob", json!({})), "E_INVALID_ARG");
    assert_eq!(
        code("body", json!({"slide": "intro", "file": "/etc/passwd"})),
        "E_INVALID_ARG"
    );
    assert_eq!(
        code("body", json!({"slide": "intro", "md": "-"})),
        "E_INVALID_ARG"
    );
    assert_eq!(
        code("title", json!({"slide": "intro", "bogus": "x"})),
        "E_INVALID_ARG"
    );
    assert_eq!(code("title", json!({"slide": "intro"})), "E_USAGE");
    assert_eq!(
        code("title", json!({"slide": "nope", "text": "x"})),
        "E_NOT_FOUND"
    );
    assert_eq!(
        run_op(
            &p,
            "title",
            &json!({"slide": "intro", "text": "x"}),
            Some("0000000000000000")
        )
        .unwrap_err()
        .code(),
        "E_CONFLICT"
    );
    assert_eq!(e.text(), BASIC);
}

/// US4 CLI: box commands, geometry in `show`, unit conversion flag, errors.
#[test]
fn box_commands() {
    let e = Env::new();
    let v = e.ok(&[
        "box", "add", "intro", "--image", "fig.png", "--at", "-10,20", "--size", "300,200",
    ]);
    assert_edit(&v, "box add");
    let id = v["target"].as_str().unwrap().to_string();
    let show = e.ok(&["show", "intro"]);
    let b = show["slide"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["id"] == id.as_str())
        .unwrap()
        .clone();
    assert_eq!(b["kind"], "box");
    assert_eq!(
        b["geom"],
        serde_json::json!({"left": -10.0, "top": 20.0, "width": 300.0, "height": 200.0, "right": null, "bottom": null})
    );
    assert_edit(
        &e.ok(&["box", "move", &format!("#{id}"), "--by", "15,5"]),
        "box move",
    );
    assert_edit(
        &e.ok(&["box", "resize", &format!("#{id}"), "--size", "100,50"]),
        "box resize",
    );
    assert!(
        e.text().contains(&format!(
            "![](fig.png){{#{id} .absolute left=5 top=25 width=100 height=50}}"
        )),
        "{}",
        e.text()
    );
    assert_edit(&e.ok(&["box", "rm", &format!("#{id}")]), "box rm");
    assert!(!e.text().contains("fig.png"));
    assert_eq!(e.ok(&["slides"])["width"], 1050);

    e.err(&["box", "move", "#b-plot"], "E_USAGE", 2);
    e.err(&["box", "add", "intro"], "E_USAGE", 2);
    e.err(
        &["box", "add", "intro", "--text", "x", "--at", "1"],
        "E_USAGE",
        2,
    );
    e.err(
        &["box", "move", "intro/0", "--at", "1,1"],
        "E_INVALID_ARG",
        1,
    );

    std::fs::write(e.path(), include_str!("corpus/11-absolute.qmd")).unwrap();
    let v = e.ok(&["box", "move", "boxes/2", "--at", "1,2"]);
    assert_eq!(v["converted_units"], true);
}

// ---------- spec 003: element hashes, minimal edits, element-level conflicts ----------

const DECK: &str = "---\ntitle: T\nformat: revealjs\n---\n\n## Methods\n\nWe use **bold** words and a [link](https://example.org)\nacross two lines.\n\n- one\n- two\n\n## Results {#results}\n\nSome \"quoted\" text -- here.\n";

fn hash(s: &str) -> String {
    quartz_deck::deck::version_of(s.as_bytes())
}

#[test]
fn show_reports_block_and_title_hashes() {
    let e = Env::with(DECK);
    let v = e.ok(&["show", "methods"]);
    for b in v["slide"]["blocks"].as_array().unwrap() {
        assert_eq!(b["hash"], hash(b["text"].as_str().unwrap()), "{b}");
    }
    assert_eq!(v["slide"]["title_hash"], hash("Methods"));
}

#[test]
fn edit_changes_only_the_element_bytes() {
    let e = Env::with(DECK);
    let new = "We use **bold** words and a [link](https://example.org)\nacross three lines.";
    e.ok(&["edit", "methods/0", "--md", new]);
    // the slide heading has an implicit id: an element edit must not pin it
    assert_eq!(
        e.text(),
        DECK.replace("across two lines.", "across three lines.")
    );
}

#[test]
fn expect_text_is_per_element() {
    let e = Env::with(DECK);
    let para = e.ok(&["show", "methods"])["slide"]["blocks"][0]["hash"]
        .as_str()
        .unwrap()
        .to_string();
    // an unrelated edit elsewhere changes the file version, not this element
    e.ok(&["edit", "results/0", "--md", "Other text."]);
    e.ok(&[
        "edit",
        "methods/0",
        "--md",
        "Changed.",
        "--expect-text",
        &para,
    ]);
    assert!(e.text().contains("## Methods\n\nChanged.\n\n- one"));
    // now the element differs from what the caller read: refused
    let err = e.err(
        &[
            "edit",
            "methods/0",
            "--md",
            "Again.",
            "--expect-text",
            &para,
        ],
        "E_CONFLICT",
        1,
    );
    assert_eq!(err["error"]["details"]["current_source"], "Changed.");
    assert_eq!(err["error"]["details"]["current_hash"], hash("Changed."));
}

#[test]
fn title_expect_text_uses_the_title_source() {
    let e = Env::with(DECK);
    e.ok(&[
        "title",
        "results",
        "Findings",
        "--expect-text",
        &hash("Results"),
    ]);
    e.err(
        &[
            "title",
            "results",
            "Again",
            "--expect-text",
            &hash("Results"),
        ],
        "E_CONFLICT",
        1,
    );
}

#[test]
fn api_maps_expect_text() {
    let e = Env::with(DECK);
    let args = serde_json::json!({"element": "methods/0", "md": "Via API.", "expect-text": "0000000000000000"});
    let r = quartz_deck::cli::run_op(&e.store(), "edit", &args, None);
    assert_eq!(
        r.unwrap_err().code(),
        "E_CONFLICT",
        "wrong hash must be refused"
    );
}

#[test]
fn body_of_an_empty_slide_keeps_the_next_slide_separate() {
    // regression: the new body ran into the next heading, which Pandoc then
    // swallowed into the body's last list item
    let e = Env::with("## A {#a}\n\n## B {#b}\n\ntext\n");
    e.ok(&["body", "a", "--md", "- one\n- two\n"]);
    assert_eq!(e.text(), "## A {#a}\n\n- one\n- two\n\n## B {#b}\n\ntext\n");
}

#[test]
fn source_that_breaks_slide_structure_is_refused() {
    let e = Env::with(DECK);
    for args in [
        ["edit", "methods/0", "--md", "Text.\n\n## Sneaky\n\nMore."], // new slide heading
        ["edit", "methods/0", "--md", "::: {.callout-note}\nopen"],   // unclosed div
        ["edit", "methods/0", "--md", "```python\nprint(1)"],         // unclosed code fence
        ["body", "methods", "--md", "a\n\n---\n\nb"],                 // `---` starts a slide
    ] {
        e.err(&args, "E_INVALID_ARG", 1);
    }
    // balanced constructs and sub-headings are fine
    e.ok(&[
        "edit",
        "methods/0",
        "--md",
        "::: {.callout-note}\nNote.\n:::",
    ]);
    e.ok(&["edit", "results/0", "--md", "### Sub\n\nText."]);
}

#[test]
fn deck_discovery_ignores_qds_render_copies() {
    // `qd serve` renders through `<stem>.qd-<tag>.qmd` next to the deck; an agent
    // running qd in that folder meanwhile must still find the one real deck
    let e = Env::with(DECK);
    fs::write(e.dir.path().join("deck.qd-preview.qmd"), "## copy\n").unwrap();
    assert_eq!(e.ok(&["slides"])["slides"][0]["id"], "methods");
}

// ---------- spec 003 US4: qd block add/move/rm/dup (T025) ----------

const BLOCKS: &str = "---\ntitle: T\n---\n\n## S\n\nFirst.\n\n- a\n- b\n\n![](x.png){#b-1 .absolute top=0 left=0 width=10 height=10}\n\n## Empty {#empty}\n\n## Last {#last}\n\nEnd.\n";

#[test]
fn block_add_places_new_blocks() {
    let e = Env::with(BLOCKS);
    let v = e.ok(&["block", "add", "s", "--after", "s/0", "--md", "Second."]);
    assert_eq!(v["target"], "s/1");
    assert!(e.text().contains("First.\n\nSecond.\n\n- a"));
    e.ok(&["block", "add", "s", "--start", "--md", "Zeroth."]);
    assert!(e.text().contains("## S\n\nZeroth.\n\nFirst."));
    e.ok(&["block", "add", "s", "--end", "--md", "Final."]);
    assert!(e.text().contains("height=10}\n\nFinal.\n\n## Empty"));
    e.ok(&[
        "block",
        "add",
        "s",
        "--before",
        "s/0",
        "--md",
        "Before all.",
    ]);
    assert!(e.text().contains("## S\n\nBefore all.\n\nZeroth."));
    // an empty slide gets its first block, still separate from the next slide
    let v = e.ok(&["block", "add", "empty", "--end", "--md", "Now here."]);
    assert_eq!(v["target"], "empty/0");
    assert!(
        e.text()
            .contains("## Empty {#empty}\n\nNow here.\n\n## Last")
    );
    // the slide heading is never rewritten (no id pinned on `## S`)
    assert!(e.text().contains("\n## S\n"));
    e.err(
        &["block", "add", "s", "--end", "--md", "## New slide"],
        "E_INVALID_ARG",
        1,
    );
    e.err(
        &["block", "add", "s", "--after", "last/0", "--md", "x"],
        "E_INVALID_ARG",
        1,
    );
}

#[test]
fn block_move_rm_dup_keep_bytes() {
    let e = Env::with(BLOCKS);
    let v = e.ok(&["block", "move", "s/1", "--before", "s/0"]);
    assert_eq!(v["target"], "s/0");
    assert!(e.text().contains("## S\n\n- a\n- b\n\nFirst.\n\n![]"));
    e.ok(&["block", "move", "s/0", "--after", "s/2"]);
    assert!(e.text().contains("## S\n\nFirst.\n\n![](x.png){#b-1 .absolute top=0 left=0 width=10 height=10}\n\n- a\n- b\n\n## Empty"));
    e.err(
        &["block", "move", "s/0", "--after", "last/0"],
        "E_INVALID_ARG",
        1,
    );

    // duplicate: same text, a fresh id for an element that has one
    let v = e.ok(&["block", "dup", "#b-1"]);
    assert_eq!(v["target"], "s/2");
    assert_eq!(v["assigned_ids"].as_array().unwrap().len(), 1);
    let t = e.text();
    assert_eq!(t.matches("![](x.png){#b-").count(), 2);
    assert_eq!(t.matches("#b-1 ").count(), 1);

    // remove, then undo restores the bytes
    let before = e.text();
    e.ok(&["block", "rm", "s/0"]);
    assert!(e.text().contains("## S\n\n![](x.png)"));
    e.ok(&["undo"]);
    assert_eq!(e.text(), before);
    // removing the only block of a slide leaves it empty and separate
    e.ok(&["block", "rm", "last/0"]);
    assert!(
        e.text().trim_end().ends_with("## Last {#last}"),
        "{:?}",
        e.text()
    );
}

#[test]
fn block_ops_through_the_api() {
    let e = Env::with(BLOCKS);
    let args = serde_json::json!({"slide": "s", "end": true, "md": "Via API."});
    let v = quartz_deck::cli::run_op(&e.store(), "block add", &args, None).unwrap();
    assert_eq!(v["op"], "block add");
    assert!(e.text().contains("Via API."));
    for op in ["block move", "block rm", "block dup"] {
        assert!(quartz_deck::cli::run_op(&e.store(), op, &serde_json::json!({}), None).is_err());
    }
}

#[test]
fn block_class_toggles_round_trip() {
    let deck = "## S {#s}\n\nPlain para.\n\n::: {.callout-note title=\"Keep 'me'\"}\nNote.\n:::\n\n#### Sub {.c}\n\n![](x.png){width=\"50%\"}\n";
    let e = Env::with(deck);
    // a paragraph has no attributes: adding wraps it, removing unwraps it again
    e.ok(&["block", "class", "s/0", "--add", "fragment"]);
    assert!(
        e.text().contains("::: {.fragment}\nPlain para.\n:::"),
        "{}",
        e.text()
    );
    e.ok(&["block", "class", "s/0", "--remove", "fragment"]);
    assert_eq!(e.text(), deck);
    // elements with attributes: the class goes in place, untouched attributes keep their quoting
    e.ok(&["block", "class", "s/1", "--add", "fragment"]);
    e.ok(&[
        "block", "class", "s/2", "--add", "fragment", "--remove", "c",
    ]);
    e.ok(&["block", "class", "s/3", "--add", "fragment"]);
    let t = e.text();
    assert!(t.contains("title=\"Keep 'me'\""), "{t}");
    assert!(t.contains(".fragment"), "{t}");
    assert!(t.contains("#### Sub {.fragment}"), "{t}");
    for el in ["s/1", "s/3"] {
        e.ok(&["block", "class", el, "--remove", "fragment"]);
    }
    e.ok(&[
        "block", "class", "s/2", "--remove", "fragment", "--add", "c",
    ]);
    assert_eq!(e.text(), deck);
    e.err(
        &["block", "class", "s/0", "--remove", "nope"],
        "E_INVALID_ARG",
        1,
    );
}

// ---------- spec 004: line ranges, code cells, qd replace (T004, T006) ----------

const RUN: &str = include_str!("fixtures/run/basic.qmd");

#[test]
fn show_reports_line_ranges_and_code_cells() {
    let e = Env::with(RUN);
    let lines: Vec<&str> = RUN.lines().collect();
    let at = |v: &Value| lines[v.as_u64().unwrap() as usize - 1];
    let v = e.ok(&["show", "plot"]);
    let s = &v["slide"];
    assert_eq!(at(&s["lines"][0]), "## Plot {#plot}");
    assert!(
        matches!(at(&s["lines"][1]), "" | "```"),
        "{}",
        at(&s["lines"][1])
    );
    let cell = &s["blocks"][0];
    assert_eq!(at(&cell["lines"][0]), "```{python}");
    assert_eq!(at(&cell["lines"][1]), "```");
    assert_eq!(cell["lang"], "python");
    assert_eq!(cell["kind"], "cell"); // executable; its `hash` is the cell's fingerprint
    // a plain ```python block is code but not a cell; an R cell is a cell in another language
    assert_eq!(
        e.ok(&["show", "plain"])["slide"]["blocks"][0]["kind"],
        "code"
    );
    assert_eq!(e.ok(&["show", "r"])["slide"]["blocks"][0]["lang"], "r");
    // slide listings carry lines too (for source ↔ slide sync)
    assert_eq!(
        at(&e.ok(&["slides"])["slides"][0]["lines"][0]),
        "## Print {#print}"
    );
}

#[test]
fn line_ranges_count_crlf_lines() {
    let e = Env::with("## A {#a}\r\n\r\nText.\r\n\r\n## B {#b}\r\n\r\nMore.\r\n");
    let v = e.ok(&["show", "b"]);
    assert_eq!(v["slide"]["lines"][0], 5);
    assert_eq!(v["slide"]["blocks"][0]["lines"], json!([7, 7]));
}

#[test]
fn replace_swaps_one_unique_occurrence() {
    use quartz_deck::cli::run_op;
    let e = Env::with(DECK);
    let v = e.ok(&[
        "replace",
        "--old",
        "across two lines.\n\n- one",
        "--new",
        "across 2 lines.\n\n- one",
    ]);
    assert_eq!(v["op"], "replace");
    assert_eq!(
        e.text(),
        DECK.replace("across two lines.", "across 2 lines.")
    );
    // not found → the text changed elsewhere; found twice → add context
    e.err(
        &["replace", "--old", "across two lines.", "--new", "x"],
        "E_CONFLICT",
        1,
    );
    let twice = Env::with("## A {#a}\n\nSame.\n\n## B {#b}\n\nSame.\n");
    twice.err(
        &["replace", "--old", "Same.", "--new", "x"],
        "E_AMBIGUOUS",
        1,
    );
    // the source pane may add slides; undo restores the bytes
    let before = e.text();
    e.ok(&[
        "replace",
        "--old",
        "- two\n",
        "--new",
        "- two\n\n## Added {#added}\n\nNew slide.\n",
    ]);
    e.ok(&["show", "added"]);
    e.ok(&["undo"]);
    assert_eq!(e.text(), before);
    // the API never reads files or the server's stdin
    for bad in [
        json!({"old": "-", "new": "x"}),
        json!({"old-file": "/etc/hosts", "new": "x"}),
    ] {
        assert_eq!(
            run_op(&e.store(), "replace", &bad, None)
                .unwrap_err()
                .code(),
            "E_INVALID_ARG"
        );
    }
    // through the API too
    let args = json!({"old": "Some \"quoted\"", "new": "Some 'quoted'"});
    run_op(&e.store(), "replace", &args, None).unwrap();
    assert!(e.text().contains("Some 'quoted' text"));
}

/// Spec 007 US2: `qd settings` reads, sets (checked) and resets the person's
/// settings; no deck needed.
#[test]
fn settings_read_set_and_reset() {
    let e = Env::new();
    let v = e.ok(&["settings"]);
    assert_eq!(v["values"]["source.side"], "left");
    assert!(
        v["defs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["key"] == "theme")
    );
    let v = e.ok(&["settings", "set", "theme", "dark"]);
    assert_eq!(
        (&v["ok"], &v["values"]["theme"]),
        (&json!(true), &json!("dark"))
    );
    let v = e.ok(&["settings", "set", "confirm.slide_delete", "false"]);
    assert_eq!(
        v["values"]["confirm.slide_delete"], false,
        "JSON values parse"
    );
    let err = e.err(
        &["settings", "set", "theme", "purple"],
        "E_INVALID_SETTING",
        1,
    );
    assert!(
        err["error"]["message"]
            .as_str()
            .unwrap()
            .contains("system, light, dark, hc")
    );
    assert_eq!(
        e.ok(&["settings"])["values"]["theme"],
        "dark",
        "a refused value changes nothing"
    );
    let v = e.ok(&["settings", "reset"]);
    assert_eq!(
        (&v["values"]["theme"], &v["values"]["confirm.slide_delete"]),
        (&json!("system"), &json!(true))
    );
}

/// Spec 007 US3: `qd actions` lists what the editor can do, with keys and the
/// matching qd command (the same list the editor's menus and keys use).
#[test]
fn actions_lists_the_editors_actions() {
    let e = Env::new();
    let v = e.ok(&["actions"]);
    let list = v.as_array().unwrap();
    let dup = list.iter().find(|a| a["id"] == "slide-dup").unwrap();
    assert_eq!(
        (&dup["label"], &dup["key"], &dup["qd"], &dup["menu"]),
        (
            &json!("Duplicate slide"),
            &json!("Mod+Shift+D"),
            &json!("qd dup <slide>"),
            &json!(["Slide"])
        )
    );
    assert!(list.iter().all(|a| a["id"].is_string()), "no separators");
    let listed: Value = serde_json::from_str(include_str!("../src/ui/actions.json")).unwrap();
    let ids = |l: &[Value]| {
        l.iter()
            .filter_map(|a| a["id"].as_str().map(String::from))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        ids(list),
        ids(listed.as_array().unwrap()),
        "the editor's own list"
    );
}

#[test]
fn paste_slides_from_stdin() {
    let e = Env::new();
    let copy = "## Results {#results}\n\n- pasted\n";
    let out = e
        .cmd()
        .args(["paste", "--after", "intro", "--md", "-"])
        .write_stdin(copy)
        .output()
        .unwrap();
    assert!(out.status.success(), "{out:?}");
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_edit(&v, "paste");
    assert_ne!(v["target"], "results", "the clashing id is renewed");
    let t = e.text();
    assert!(
        t.contains(&format!(
            "{{#{}}}\n\n- pasted",
            v["target"].as_str().unwrap()
        )),
        "{t}"
    );
    e.err(&["paste", "--md", "no heading here"], "E_PARSE", 1);
    assert_eq!(e.ok(&["undo"])["undone"], "paste", "one undo step");
    assert!(!e.text().contains("- pasted"));
}

/// Spec 008 US5: images beside the deck, listed and added (copied under a free
/// name; with --slide also inserted, as one undo step).
#[test]
fn images_list_and_add() {
    let e = Env::new();
    let png = e.dir.path().join("chart.png");
    fs::write(&png, b"\x89PNG\r\n\x1a\nbody").unwrap();
    let v = e.ok(&["images"]);
    assert_eq!(v["images"][0]["path"], "chart.png");

    let before = e.text();
    let v = e.ok(&["image", "add", "chart.png"]);
    assert_eq!(v["path"], "images/chart.png");
    assert_eq!(e.text(), before, "path only: the deck is untouched");

    let v = e.ok(&["image", "add", "chart.png", "--slide", "results", "--end"]);
    assert_eq!(v["path"], "images/chart-1.png");
    assert!(e.text().contains("![](images/chart-1.png)"), "{}", e.text());
    e.ok(&["undo"]);
    assert_eq!(e.text(), before);
    assert!(
        e.dir.path().join("images/chart-1.png").exists(),
        "the file stays"
    );

    fs::write(e.dir.path().join("fake.png"), b"GIF89a").unwrap();
    e.err(&["image", "add", "fake.png"], "E_INVALID_ARG", 1);
}

/// Spec 008 US9: `qd move` and `qd rm` with several slides, each one undo step.
#[test]
fn move_and_rm_several_slides() {
    let e = Env::new();
    let before = e.text();
    let titles = |e: &Env| {
        e.ok(&["slides"])["slides"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["title"].as_str().unwrap_or("").to_string())
            .collect::<Vec<_>>()
    };
    e.ok(&["move", "intro", "results", "--after", "conclusion"]);
    assert_eq!(titles(&e), ["", "Conclusion", "Intro", "Results"]);
    e.ok(&["undo"]);
    assert_eq!(e.text(), before);
    e.ok(&["rm", "intro", "conclusion"]);
    assert_eq!(titles(&e), ["Results", ""]);
    e.ok(&["undo"]);
    assert_eq!(e.text(), before);
}

/// Spec 009 US1: `qd layouts` and `qd add --layout`, filled with `qd edit`.
#[test]
fn layouts_list_and_add() {
    let e = Env::new();
    let before = e.text();
    let v = e.ok(&["layouts"]);
    let names: Vec<&str> = v["layouts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["name"].as_str().unwrap())
        .collect();
    assert!(
        names.contains(&"two-columns") && names.len() >= 12,
        "{names:?}"
    );
    let v = e.ok(&[
        "add",
        "--layout",
        "image-text",
        "--after",
        "intro",
        "--title",
        "Chart",
    ]);
    let ph = v["placeholders"].as_array().unwrap();
    assert_eq!(
        ph.iter()
            .map(|p| p["kind"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["image", "text"]
    );
    let text_ref = ph[1]["ref"].as_str().unwrap();
    e.ok(&["edit", text_ref, "--md", "Users doubled."]);
    assert!(e.text().contains("Users doubled.\n:::"));
    e.ok(&["undo"]);
    e.ok(&["undo"]);
    assert_eq!(e.text(), before, "the add is one undo step");
    e.err(&["add", "--layout", "nope"], "E_NOT_FOUND", 1);
}

/// Spec 009 US2: place, move, unplace from the CLI; one undo step each.
#[test]
fn place_and_unplace() {
    let e = Env::new();
    let before = e.text();
    let v = e.ok(&["block", "place", "results/0", "--at", "600,400"]);
    let id = v["target"].as_str().unwrap().to_string();
    assert!(e.text().contains(&format!(
        "::: {{#{id} .absolute left=600 top=400}}\n- one\n- two\n:::"
    )));
    e.ok(&["box", "move", &id, "--by", "10,-5"]);
    assert!(e.text().contains("left=610 top=395"));
    e.ok(&["block", "unplace", &id]);
    assert_eq!(
        e.text(),
        before,
        "unplace takes off exactly what place wrote"
    );
    for _ in 0..3 {
        e.ok(&["undo"]);
    }
    assert_eq!(e.text(), before);
    e.ok(&[
        "block",
        "place",
        "results/0",
        "--at",
        "1,2",
        "--size",
        "300,200",
    ]);
    assert!(e.text().contains("left=1 top=2 width=300 height=200}"));
    e.ok(&["undo"]);
    e.err(
        &["block", "place", "conclusion/1", "--at", "1,2"],
        "E_INVALID_ARG",
        1,
    );
    e.err(&["block", "unplace", "intro/0"], "E_INVALID_ARG", 1);
}
