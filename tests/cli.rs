//! CLI contract tests (contracts/cli.md): JSON on stdout, JSON errors on stderr,
//! stable codes and exit statuses, file untouched on error/dry-run.

use assert_cmd::Command;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

const BASIC: &str = include_str!("fixtures/basic.qmd");

struct Env {
    dir: tempfile::TempDir,
}

impl Env {
    fn new() -> Env {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("deck.qmd"), BASIC).unwrap();
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
