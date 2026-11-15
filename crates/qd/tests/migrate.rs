//! Spec 014 US5 (T031): `qd bib migrate` converts a `{{< paper >}}` deck to
//! `@key` citations, checks that nothing changed for the audience, and puts
//! everything back when something did (unless `--accept`).

mod common;

use common::{git_repo, have, json_of, qd, script};
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

const FIX: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/migrate");

/// The fixture deck and its `paper` extension in a temp folder, `edit`ed.
fn setup(edit: impl Fn(String) -> String) -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    let ext = t.path().join("_extensions/paper");
    fs::create_dir_all(&ext).unwrap();
    for f in fs::read_dir(Path::new(FIX).join("_extensions/paper")).unwrap() {
        let f = f.unwrap();
        fs::copy(f.path(), ext.join(f.file_name())).unwrap();
    }
    let deck = fs::read_to_string(Path::new(FIX).join("deck.qmd")).unwrap();
    fs::write(t.path().join("deck.qmd"), edit(deck)).unwrap();
    t
}

/// The same, as a git repository with everything committed.
fn in_git(edit: impl Fn(String) -> String) -> tempfile::TempDir {
    let t = setup(edit);
    {
        let git = git_repo(t.path());
        git(&["add", "-A"]);
        git(&["commit", "-qm", "deck"]);
    }
    t
}

/// `qd bib migrate args…`, with `bin` (stub plugins) first on PATH.
fn migrate(dir: &Path, bin: Option<&Path>, args: &[&str]) -> (bool, Value) {
    let mut c = qd(dir);
    if let Some(b) = bin {
        c.env(
            "PATH",
            format!("{}:{}", b.display(), std::env::var("PATH").unwrap()),
        );
    }
    let mut all = vec!["bib", "migrate"];
    all.extend(args);
    json_of(c, &all)
}

/// Every file under `dir`, with its bytes, but qd's cache, git's folder and
/// what any Quarto render writes (`.quarto/`, its `.gitignore`, `*_files/`).
fn snapshot(dir: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut out = vec![];
    let mut todo = vec![dir.to_path_buf()];
    while let Some(d) = todo.pop() {
        for e in fs::read_dir(&d).unwrap() {
            let p = e.unwrap().path();
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            if [".cache", ".config", ".git", ".quarto", ".gitignore"].contains(&name.as_str())
                || name.ends_with("_files")
            {
                continue;
            }
            if p.is_dir() {
                todo.push(p);
            } else {
                out.push((
                    p.strip_prefix(dir).unwrap().to_path_buf(),
                    fs::read(&p).unwrap(),
                ));
            }
        }
    }
    out.sort();
    out
}

#[test]
fn migrate_clean() {
    if !have("quarto") {
        return;
    }
    let t = in_git(|d| d);
    let p = t.path();
    let (good, v) = migrate(p, None, &[]);
    assert!(good, "{v}");
    assert_eq!(v["differences"], json!([]), "{v}");
    assert_eq!(v["citations"], json!({"inline": 1, "corner": 1}));
    assert_eq!(v["installed"], json!(["cite-links", "refs-slide"]));
    let deck = fs::read_to_string(p.join("deck.qmd")).unwrap();
    assert!(
        !deck.contains("{{< paper") && !deck.contains("bibliography:start"),
        "{deck}"
    );
    assert!(
        deck.contains("[[@bell2020]]{.corner}") && deck.contains("found by @arden2019."),
        "{deck}"
    );
    assert!(deck.contains("::: {#refs refs-style=\"compact\" refs-columns=\"3\"}\n:::\n"));
    assert!(deck.contains("bibliography: references.bib") && deck.contains("- refs-slide"));
    assert!(
        deck.contains("nocite: '@*'"),
        "the list keeps every entry, as before: {deck}"
    );
    assert!(!p.join("_extensions/paper").exists(), "paper removed");
    assert!(p.join("references.bib").is_file());
    assert!(Path::new(v["report"].as_str().unwrap()).is_file());
    // plain Quarto renders it
    let out = std::process::Command::new("quarto")
        .args(["render", "deck.qmd"])
        .current_dir(p)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// A hand-written label differs from its entry's: the conversion fails and
/// every file is as it was; with --accept it is kept, and still reported.
#[test]
fn migrate_diff_fails_and_restores_unless_accepted() {
    if !have("quarto") {
        return;
    }
    let hand = |d: String| d.replacen(r#"who="Arden""#, r#"who="Arden, by hand""#, 1);
    let t = in_git(hand);
    let p = t.path();
    let before = snapshot(p);
    let (good, v) = migrate(p, None, &[]);
    assert!(!good, "{v}");
    assert_eq!(v["error"]["code"], "E_MIGRATE_DIFF", "{v}");
    let msg = v["error"]["message"].as_str().unwrap();
    let report = msg.split("Report: ").nth(1).unwrap();
    let r: Value = serde_json::from_str(&fs::read_to_string(report).unwrap()).unwrap();
    assert_eq!(r["differences"][0]["kind"], "text", "{r}");
    assert_eq!(r["differences"][0]["slide"], "bibliography", "{r}");
    let names = |s: &[(PathBuf, Vec<u8>)]| s.iter().map(|(p, _)| p.clone()).collect::<Vec<_>>();
    let after = snapshot(p);
    assert_eq!(names(&after), names(&before), "the same files");
    assert_eq!(after, before, "every file back as it was");

    let (good, v) = migrate(p, None, &["--accept"]);
    assert!(good, "{v}");
    assert_eq!(v["accepted"], true);
    assert!(!v["differences"].as_array().unwrap().is_empty());
    assert!(
        !fs::read_to_string(p.join("deck.qmd"))
            .unwrap()
            .contains("{{< paper")
    );
    // and it is one edit: qd undo puts the deck, Makefile-less folder and paper back
    let (good, v) = json_of(qd(p), &["undo"]);
    assert!(good, "{v}");
    assert!(
        fs::read_to_string(p.join("deck.qmd"))
            .unwrap()
            .contains("{{< paper")
    );
    assert!(p.join("_extensions/paper/paper.lua").is_file());
}

/// References from a source plugin (the Makefile's Zotero key, through a stub
/// `qd-zotero`): matched by URL, else by title; an unmatched shortcode gets an
/// entry built from itself; the Makefile's refs target is rewritten.
#[test]
fn migrate_from_a_source() {
    if !have("quarto") {
        return;
    }
    let t = in_git(|d| d);
    let p = t.path();
    fs::write(p.join("Makefile"), "refs:\n\tqd bib --zotero ABCD1234\n").unwrap();
    let bin = p.join(".cache/bin");
    fs::create_dir_all(&bin).unwrap();
    // Bell & Cho by DOI under the manager's key; Arden by title, under another URL
    script(
        &bin.join("qd-zotero"),
        r#"[ "$1" = "ABCD1234" ] || exit 9
cat <<'JSON'
[{"id": "Bell_TwoAuthors", "type": "article-journal", "author": [{"family": "Bell"}, {"family": "Cho"}], "issued": {"date-parts": [[2020]]}, "title": "Two authors", "DOI": "10.5555/bell"},
 {"id": "arden_one", "type": "article-journal", "author": [{"family": "Arden"}], "issued": {"date-parts": [[2019]]}, "title": "One author", "URL": "https://example.org/arden"}]
JSON"#,
    );
    let (good, v) = migrate(p, Some(&bin), &[]);
    assert!(good, "{v}");
    assert_eq!(v["differences"], json!([]), "{v}");
    assert_eq!(v["unmatched"], json!([]));
    let deck = fs::read_to_string(p.join("deck.qmd")).unwrap();
    assert!(
        deck.contains("[[@Bell_TwoAuthors]]{.corner}") && deck.contains("@arden_one."),
        "{deck}"
    );
    assert_eq!(
        fs::read_to_string(p.join("Makefile")).unwrap(),
        "refs:\n\tqd bib pull --from zotero ABCD1234\n"
    );

    // a shortcode the collection lacks gets its own entry, and is listed
    let t = in_git(|d| {
        d.replace("https://example.org/arden", "https://example.org/elsewhere")
            .replace("One author", "Lost")
    });
    let p = t.path();
    fs::write(p.join("Makefile"), "refs:\n\tqd bib --zotero ABCD1234\n").unwrap();
    // its own entry joins the whole collection on the References slide: a
    // change the report shows, accepted here as the person would
    let (good, v) = migrate(p, Some(&bin), &["--accept"]);
    assert!(good, "{v}");
    assert_eq!(v["unmatched"][0]["key"], "arden2019", "{v}");
    assert!(
        v["differences"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["slide"] == "bibliography")
    );
}

/// A deck with citations but no References list keeps having none: citeproc
/// would otherwise append one to the last slide, which the check would catch.
#[test]
fn migrate_without_a_references_slide() {
    if !have("quarto") {
        return;
    }
    let t = in_git(|d| d.split("## References").next().unwrap().to_string());
    let (good, v) = migrate(t.path(), None, &[]);
    assert!(good, "{v}");
    assert_eq!(v["differences"], json!([]));
    let deck = fs::read_to_string(t.path().join("deck.qmd")).unwrap();
    assert!(deck.contains("suppress-bibliography: true"), "{deck}");
}

#[test]
fn migrate_needs_a_backup_outside_git() {
    let t = setup(|d| d);
    let p = t.path();
    let (good, v) = migrate(p, None, &[]);
    assert!(!good);
    assert_eq!(v["error"]["code"], "E_INVALID_ARG", "{v}");
    assert!(v["error"]["message"].as_str().unwrap().contains("--backup"));
    if !have("quarto") {
        return;
    }
    let backup = tempfile::tempdir().unwrap();
    let (good, v) = migrate(p, None, &["--backup", backup.path().to_str().unwrap()]);
    assert!(good, "{v}");
    let saved = fs::read_to_string(backup.path().join("deck.qmd")).unwrap();
    assert!(saved.contains("{{< paper"), "the deck as it was");
    assert!(backup.path().join("_extensions/paper/paper.lua").is_file());
}

#[test]
fn migrate_refuses_what_it_cannot_read() {
    let t = in_git(|d| d.replace(r#"corner="true""#, r#"size="big""#));
    let (good, v) = migrate(t.path(), None, &[]);
    assert!(!good);
    assert_eq!(v["error"]["code"], "E_INVALID_ARG");
    assert!(
        v["error"]["message"].as_str().unwrap().contains("res"),
        "{v}"
    );
    let none =
        in_git(|d| d.split("## Results").next().unwrap().to_string() + "## A\n\nNo papers.\n");
    let (good, v) = migrate(none.path(), None, &[]);
    assert!(!good && v["error"]["code"] == "E_INVALID_ARG", "{v}");
}
