//! Spec 014 US2: `qd bib` keeps a deck's references in a companion file or
//! embedded, in BibTeX or CSL-JSON, with stable keys (contracts/cli.md).

mod common;

use common::{have, qd};
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

const DECK: &str = "---\ntitle: T\nformat: revealjs\n---\n\n## One\n\nSee @bonaca2020 and [@koposov2010]. Mail me@example.org, see @fig-x.\n\n```python\nx = '@notakey'\n```\n";

const BIB: &str = "@article{bonaca2020,\n  author = {Bonaca, Ana and Hogg, David},\n  title = {Spur},\n  year = {2020},\n  doi = {10.1/x}\n}\n\n@article{koposov2010,\n  author = {Koposov, Sergey},\n  title = {GD-1},\n  year = {2010},\n  url = {https://e.x/k}\n}\n";

fn run(dir: &Path, args: &[&str]) -> (bool, Value) {
    let out = qd(dir).args(args).output().unwrap();
    let text = if out.status.success() {
        &out.stdout
    } else {
        &out.stderr
    };
    let v = serde_json::from_slice(text).unwrap_or_else(|_| {
        panic!(
            "{args:?}: not JSON: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        )
    });
    (out.status.success(), v)
}

fn ok(dir: &Path, args: &[&str]) -> Value {
    let (good, v) = run(dir, args);
    assert!(good, "{args:?}: {v}");
    v
}

fn err(dir: &Path, args: &[&str], code: &str) {
    let (good, v) = run(dir, args);
    assert!(!good, "{args:?} should fail: {v}");
    assert_eq!(v["error"]["code"], code, "{args:?}: {v}");
}

fn deck(dir: &Path) -> String {
    fs::read_to_string(dir.join("talk.qmd")).unwrap()
}

/// What the citations render as, through Pandoc's citeproc.
fn cited(dir: &Path) -> String {
    let out = std::process::Command::new("quarto")
        .args(["pandoc", "talk.qmd", "--citeproc", "-t", "plain"])
        .current_dir(dir)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap()
}

fn setup() -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    fs::write(t.path().join("talk.qmd"), DECK).unwrap();
    fs::write(t.path().join("paper.bib"), BIB).unwrap();
    t
}

#[test]
fn list_none_and_missing() {
    let t = setup();
    let v = ok(t.path(), &["bib"]);
    assert_eq!(v["where"], "none");
    assert_eq!(v["entries"], 0);
    let missing: Vec<&str> = v["missing"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["key"].as_str().unwrap())
        .collect();
    assert_eq!(missing, ["bonaca2020", "koposov2010"], "{v}");
}

#[test]
fn pull_creates_companion() {
    if !have("quarto") {
        return;
    }
    let t = setup();
    let v = ok(t.path(), &["bib", "pull", "paper.bib"]);
    assert_eq!(v["op"], "bib-pull");
    assert_eq!(v["created"], true);
    assert_eq!(v["file"], "references.bib");
    assert_eq!(v["added"], json!(["bonaca2020", "koposov2010"]));
    let bib = fs::read_to_string(t.path().join("references.bib")).unwrap();
    assert!(bib.starts_with("% Written by qd bib"), "{bib}");
    assert!(deck(t.path()).contains("bibliography: references.bib\n"));
    assert_eq!(
        deck(t.path()).split_once("---\n\n").unwrap().1,
        DECK.split_once("---\n\n").unwrap().1,
        "only the front matter changed"
    );
    assert_eq!(ok(t.path(), &["bib"])["missing"], json!([]));
    assert!(cited(t.path()).contains("Bonaca and Hogg"));

    // a _talk/ support folder is the default place
    let t2 = setup();
    fs::create_dir(t2.path().join("_talk")).unwrap();
    let v = ok(t2.path(), &["bib", "pull", "paper.bib"]);
    assert_eq!(v["file"], "_talk/references.bib");
    assert!(t2.path().join("_talk/references.bib").is_file());
}

#[test]
fn pull_merges_and_keeps_keys() {
    if !have("quarto") {
        return;
    }
    let t = setup();
    ok(t.path(), &["bib", "pull", "paper.bib"]);
    // the same papers under other keys, one DOI added, one paper missing
    fs::write(
        t.path().join("new.bib"),
        "@article{Bonaca_Spur,\n  author = {Bonaca, Ana and Hogg, David},\n  title = {Spur},\n  year = {2020},\n  doi = {10.1/x},\n  volume = {3}\n}\n",
    )
    .unwrap();
    let v = ok(t.path(), &["bib", "pull", "new.bib"]);
    assert_eq!(v["created"], false);
    assert_eq!(v["added"], json!([]));
    assert_eq!(
        v["changed"],
        json!([{"id": "bonaca2020", "fields": ["volume"]}])
    );
    assert_eq!(v["gone"], json!(["koposov2010"]));
    assert!(cited(t.path()).contains("Bonaca and Hogg"));
}

#[test]
fn pull_json_and_biblatex() {
    if !have("quarto") {
        return;
    }
    let t = setup();
    fs::write(
        t.path().join("refs.json"),
        r#"[{"id": "koposov2010", "type": "article-journal", "author": [{"family": "Koposov"}], "issued": {"date-parts": [[2010]]}, "title": "GD-1"}]"#,
    )
    .unwrap();
    assert_eq!(
        ok(t.path(), &["bib", "pull", "refs.json"])["added"],
        json!(["koposov2010"])
    );
    fs::write(
        t.path().join("late.bib"),
        "@online{late2024,\n  author = {Late, Lu},\n  title = {Late},\n  date = {2024-03},\n  url = {https://l.x}\n}\n",
    )
    .unwrap();
    assert_eq!(
        ok(t.path(), &["bib", "pull", "late.bib"])["added"],
        json!(["late2024"])
    );
}

#[test]
fn embed_extract_convert_roundtrip() {
    if !have("quarto") {
        return;
    }
    let t = setup();
    let p = t.path();
    ok(p, &["bib", "pull", "paper.bib"]);
    let before = cited(p);

    let v = ok(p, &["bib", "embed"]);
    assert_eq!(v["left"], "references.bib");
    let text = deck(p);
    assert!(
        text.contains("references:\n- ") && !text.contains("bibliography:"),
        "{text}"
    );
    assert_eq!(ok(p, &["bib"])["where"], "embedded");
    assert_eq!(cited(p), before);

    fs::remove_file(p.join("references.bib")).unwrap();
    ok(p, &["bib", "extract", "--format", "json"]);
    let text = deck(p);
    assert!(text.contains("bibliography: references.json") && !text.contains("references:\n"));
    assert_eq!(cited(p), before);

    ok(p, &["bib", "convert", "--format", "bib"]);
    assert!(deck(p).contains("bibliography: references.bib"));
    assert!(!p.join("references.json").exists());
    assert_eq!(cited(p), before);

    // undo puts the old file back, under its old name
    ok(p, &["undo"]);
    assert!(p.join("references.json").is_file() && !p.join("references.bib").exists());
    assert!(deck(p).contains("bibliography: references.json"));
}

#[test]
fn read_only_outside() {
    if !have("quarto") {
        return;
    }
    let t = setup();
    let p = t.path().join("deck");
    fs::create_dir_all(&p).unwrap();
    fs::create_dir_all(t.path().join("shared")).unwrap();
    fs::copy(t.path().join("paper.bib"), t.path().join("shared/refs.bib")).unwrap();
    fs::write(
        p.join("talk.qmd"),
        DECK.replace("title: T\n", "title: T\nbibliography: ../shared/refs.bib\n"),
    )
    .unwrap();
    let v = ok(&p, &["bib"]);
    assert_eq!(v["files"][0]["inside"], false);
    assert_eq!(v["entries"], 2);
    err(&p, &["bib", "pull", "../paper.bib"], "E_READ_ONLY");
    err(&p, &["bib", "convert", "--format", "json"], "E_READ_ONLY");
    // embedding reads it, and writes only the deck
    ok(&p, &["bib", "embed"]);
    assert!(deck(&p).contains("references:"));
    err(
        &p,
        &["bib", "extract", "--to", "../shared/x.bib"],
        "E_READ_ONLY",
    );
    assert_eq!(
        fs::read_to_string(t.path().join("shared/refs.bib")).unwrap(),
        BIB
    );
    assert!(!t.path().join("shared/x.bib").exists());
}

#[test]
fn ambiguous_companions() {
    let t = setup();
    fs::write(t.path().join("a.bib"), BIB).unwrap();
    fs::write(t.path().join("b.bib"), BIB).unwrap();
    fs::write(
        t.path().join("talk.qmd"),
        DECK.replace("title: T\n", "title: T\nbibliography: [a.bib, b.bib]\n"),
    )
    .unwrap();
    err(t.path(), &["bib", "pull", "paper.bib"], "E_AMBIGUOUS");
}

#[test]
fn dry_run_expect_undo() {
    if !have("quarto") {
        return;
    }
    let t = setup();
    let p = t.path();
    let v = ok(p, &["bib", "pull", "paper.bib", "--dry-run"]);
    assert_eq!(v["dry_run"], true);
    assert_eq!(v["added"], json!(["bonaca2020", "koposov2010"]));
    assert!(!p.join("references.bib").exists());
    assert_eq!(deck(p), DECK);

    err(
        p,
        &["bib", "pull", "paper.bib", "--expect", "0000"],
        "E_CONFLICT",
    );
    let version = ok(p, &["bib", "pull", "paper.bib"])["version"].clone();
    assert!(version.is_string());
    ok(p, &["undo"]);
    assert_eq!(deck(p), DECK);
    assert!(
        !p.join("references.bib").exists(),
        "the file the pull created is gone"
    );
}
