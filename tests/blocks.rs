//! Spec 003 foundations (T004): block hashes, minimal element edits,
//! element-level conflict checks (`--expect-text`).

use assert_cmd::Command;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

const DECK: &str = "---\ntitle: T\nformat: revealjs\n---\n\n## Methods\n\nWe use **bold** words and a [link](https://example.org)\nacross two lines.\n\n- one\n- two\n\n## Results {#results}\n\nSome \"quoted\" text -- here.\n";

struct Env {
    dir: tempfile::TempDir,
}

impl Env {
    fn new(text: &str) -> Env {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("deck.qmd"), text).unwrap();
        Env { dir }
    }
    fn path(&self) -> PathBuf {
        self.dir.path().join("deck.qmd")
    }
    fn text(&self) -> String {
        fs::read_to_string(self.path()).unwrap()
    }
    fn run(&self, args: &[&str]) -> (bool, Value, Value) {
        let out = Command::cargo_bin("qd")
            .unwrap()
            .current_dir(self.dir.path())
            .env("QD_CACHE_DIR", self.dir.path().join(".cache"))
            .env_remove("QD_DECK")
            .args(args)
            .output()
            .unwrap();
        let p = |b: &[u8]| serde_json::from_slice(b).unwrap_or(Value::Null);
        (out.status.success(), p(&out.stdout), p(&out.stderr))
    }
    fn ok(&self, args: &[&str]) -> Value {
        let (ok, v, e) = self.run(args);
        assert!(ok, "qd {args:?}: {e}");
        v
    }
}

fn hash(s: &str) -> String {
    Sha256::digest(s.as_bytes())
        .iter()
        .take(8)
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn show_reports_block_hashes() {
    let e = Env::new(DECK);
    let v = e.ok(&["show", "methods"]);
    for b in v["slide"]["blocks"].as_array().unwrap() {
        assert_eq!(b["hash"], hash(b["text"].as_str().unwrap()), "{b}");
    }
}

#[test]
fn edit_changes_only_the_element_bytes() {
    let e = Env::new(DECK);
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
    let e = Env::new(DECK);
    let v = e.ok(&["show", "methods"]);
    let para = v["slide"]["blocks"][0]["hash"]
        .as_str()
        .unwrap()
        .to_string();
    // an unrelated edit elsewhere changes the file version...
    e.ok(&["edit", "results/0", "--md", "Other text."]);
    // ...but not this element: the save still applies
    e.ok(&[
        "edit",
        "methods/0",
        "--md",
        "Changed.",
        "--expect-text",
        &para,
    ]);
    assert!(e.text().contains("## Methods\n\nChanged.\n\n- one"));
    // now the element differs from what the caller read: refused, file unchanged
    let before = e.text();
    let (ok, _, err) = e.run(&[
        "edit",
        "methods/0",
        "--md",
        "Again.",
        "--expect-text",
        &para,
    ]);
    assert!(!ok);
    assert_eq!(err["error"]["code"], "E_CONFLICT", "{err}");
    assert_eq!(
        err["error"]["details"]["current_source"], "Changed.",
        "{err}"
    );
    assert_eq!(
        err["error"]["details"]["current_hash"],
        hash("Changed."),
        "{err}"
    );
    assert_eq!(e.text(), before);
}

#[test]
fn title_expect_text_uses_the_title_source() {
    let e = Env::new(DECK);
    e.ok(&[
        "title",
        "results",
        "Findings",
        "--expect-text",
        &hash("Results"),
    ]);
    let (ok, _, err) = e.run(&[
        "title",
        "results",
        "Again",
        "--expect-text",
        &hash("Results"),
    ]);
    assert!(!ok);
    assert_eq!(err["error"]["code"], "E_CONFLICT", "{err}");
}

#[test]
fn api_maps_expect_text() {
    let e = Env::new(DECK);
    let store =
        quartz_deck::store::Store::open_in(&e.path(), &e.dir.path().join(".cache")).unwrap();
    let args = serde_json::json!({"element": "methods/0", "md": "Via API.", "expect-text": "0000000000000000"});
    let r = quartz_deck::cli::run_op(&store, "edit", &args, None);
    assert!(r.is_err(), "wrong hash must be refused");
}

#[test]
fn body_of_an_empty_slide_keeps_the_next_slide_separate() {
    // regression: the new body ran into the next heading, which Pandoc then
    // swallowed into the body's last list item
    let e = Env::new("## A {#a}\n\n## B {#b}\n\ntext\n");
    e.ok(&["body", "a", "--md", "- one\n- two\n"]);
    assert_eq!(e.text(), "## A {#a}\n\n- one\n- two\n\n## B {#b}\n\ntext\n");
}

#[test]
fn source_that_breaks_slide_structure_is_refused() {
    let e = Env::new(DECK);
    for (args, why) in [
        (
            vec!["edit", "methods/0", "--md", "Text.\n\n## Sneaky\n\nMore."],
            "new slide heading",
        ),
        (
            vec!["edit", "methods/0", "--md", "::: {.callout-note}\nopen"],
            "unclosed div",
        ),
        (
            vec!["edit", "methods/0", "--md", "```python\nprint(1)"],
            "unclosed code fence",
        ),
        (
            vec!["body", "methods", "--md", "a\n\n---\n\nb"],
            "horizontal rule starts a slide",
        ),
    ] {
        let before = e.text();
        let (ok, _, err) = e.run(&args);
        assert!(!ok, "{why} should be refused");
        assert_eq!(err["error"]["code"], "E_INVALID_ARG", "{why}: {err}");
        assert_eq!(e.text(), before, "{why}: file must be unchanged");
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
