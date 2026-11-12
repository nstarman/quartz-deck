//! 012 US1 (T004): `qd history` in temp git repositories (signing off
//! locally; tests/common `git_repo`). Rendering old versions needs Quarto.

mod common;
use common::{git_repo, have, qd};
use serde_json::Value;
use std::path::Path;

fn run(dir: &Path, args: &[&str]) -> (bool, Value) {
    let o = qd(dir)
        .args(["--deck", "talks/deck.qmd"])
        .args(args)
        .output()
        .unwrap();
    let text = if o.status.success() {
        &o.stdout
    } else {
        &o.stderr
    };
    (
        o.status.success(),
        serde_json::from_slice(text)
            .unwrap_or_else(|_| panic!("{args:?}: {}", String::from_utf8_lossy(text))),
    )
}

fn messages(v: &Value) -> Vec<String> {
    v["versions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["message"].as_str().unwrap().to_string())
        .collect()
}

/// talks/deck.qmd with five commits: two by qd, a reorder, and a rename.
fn five_commits(dir: &Path) {
    let git = git_repo(dir);
    std::fs::create_dir_all(dir.join("talks")).unwrap();
    let write = |t: &str| std::fs::write(dir.join("talks/deck.qmd"), t).unwrap();
    write("## One {#one}\n\nA\n\n## Two {#two}\n\nB\n");
    git(&["add", "."]);
    git(&["commit", "-qm", "first"]);
    write("## One {#one}\n\nA2\n\n## Two {#two}\n\nB\n");
    git(&["commit", "-qam", "qd: edit one (person)"]);
    write("## Two {#two}\n\nB\n\n## One {#one}\n\nA2\n");
    git(&["commit", "-qam", "reorder"]);
    git(&["mv", "talks/deck.qmd", "talks/old.qmd"]);
    git(&["commit", "-qm", "rename away"]);
    git(&["mv", "talks/old.qmd", "talks/deck.qmd"]);
    write("## Two {#two}\n\nB2\n\n## One {#one}\n\nA2\n");
    git(&["add", "."]);
    git(&["commit", "-qm", "qd: edit two (agent)"]);
}

#[test]
fn history_lists_versions() {
    let t = tempfile::tempdir().unwrap();
    five_commits(t.path());
    let (ok, v) = run(t.path(), &["history"]);
    assert!(ok, "{v}");
    assert_eq!(v["kind"], "history");
    assert_eq!(
        messages(&v),
        [
            "qd: edit two (agent)",
            "rename away",
            "reorder",
            "qd: edit one (person)",
            "first"
        ],
        "newest first, through the renames"
    );
    let first = &v["versions"][0];
    assert_eq!(first["qd"], true);
    assert_eq!(first["author"], "Tester");
    assert!(first["time"].as_u64().unwrap() > 0);
    assert_eq!(v["versions"][1]["qd"], false);
    assert_eq!(
        v["versions"][1]["path"], "talks/old.qmd",
        "the path it had then"
    );
    assert_eq!(v["more"], false);

    let (_, v) = run(t.path(), &["history", "--limit", "2"]);
    assert_eq!(messages(&v).len(), 2);
    assert_eq!(v["more"], true);
    let (_, v) = run(t.path(), &["history", "--limit", "2", "--skip", "2"]);
    assert_eq!(messages(&v), ["reorder", "qd: edit one (person)"]);

    // an uncommitted change comes first
    std::fs::write(
        t.path().join("talks/deck.qmd"),
        "## Two {#two}\n\nB3\n\n## One {#one}\n\nA2\n",
    )
    .unwrap();
    let (_, v) = run(t.path(), &["history", "--limit", "1"]);
    assert_eq!(v["versions"][0]["short"], "uncommitted");
    assert_eq!(v["versions"][0]["rev"], Value::Null);
}

#[test]
fn history_per_slide() {
    let t = tempfile::tempdir().unwrap();
    five_commits(t.path());
    // slide one changed in "qd: edit one" (and was created in "first");
    // the reorder and renames moved it but didn't change it
    let (ok, v) = run(t.path(), &["history", "--slide", "one"]);
    assert!(ok, "{v}");
    assert_eq!(messages(&v), ["qd: edit one (person)", "first"]);
    let (_, v) = run(t.path(), &["history", "--slide", "two"]);
    assert_eq!(messages(&v), ["qd: edit two (agent)", "first"]);
}

#[test]
fn history_outside_git() {
    let t = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(t.path().join("talks")).unwrap();
    std::fs::write(t.path().join("talks/deck.qmd"), "## One\n\nA\n").unwrap();
    std::fs::write(t.path().join("talks/other.txt"), "x").unwrap();
    let (ok, v) = run(t.path(), &["history"]);
    assert!(!ok);
    assert_eq!(v["error"]["code"], "E_NO_REPO");
    assert!(
        v["error"]["hint"]
            .as_str()
            .unwrap()
            .contains("qd history --init")
    );

    // --init: a repository in the deck's folder, the deck alone committed
    let (ok, v) = run_init(t.path());
    assert!(ok, "{v}");
    assert!(t.path().join("talks/.git").is_dir());
    let files = quartz_deck::history::git(&t.path().join("talks"), &["ls-files"]).unwrap();
    assert_eq!(files, "deck.qmd\n", "the deck alone");
    let (ok, v) = run(t.path(), &["history", "--init"]);
    assert!(!ok);
    assert_eq!(v["error"]["code"], "E_INVALID_ARG");
}

/// `--init` commits with the person's git configuration; here a user and no
/// signing come from the environment (a test machine may have neither).
fn run_init(dir: &Path) -> (bool, Value) {
    let mut c = qd(dir);
    c.args(["--deck", "talks/deck.qmd", "history", "--init"]);
    for (i, (k, v)) in [
        ("user.name", "Tester"),
        ("user.email", "tester@example.com"),
        ("commit.gpgsign", "false"),
    ]
    .iter()
    .enumerate()
    {
        c.env(format!("GIT_CONFIG_KEY_{i}"), k)
            .env(format!("GIT_CONFIG_VALUE_{i}"), v);
    }
    let o = c.env("GIT_CONFIG_COUNT", "3").output().unwrap();
    let text = if o.status.success() {
        &o.stdout
    } else {
        &o.stderr
    };
    (o.status.success(), serde_json::from_slice(text).unwrap())
}

#[test]
fn render_old_version() {
    if !have("quarto") {
        return;
    }
    let t = tempfile::tempdir().unwrap();
    let dir = t.path();
    let git = git_repo(dir);
    std::fs::create_dir_all(dir.join("talks")).unwrap();
    // a cell that would leave a marker if it ran; Quarto must not run it
    let marker = dir.join("talks/ran.txt");
    let deck = format!(
        "---\ntitle: Then\n---\n\n## Old {{#old}}\n\n```{{python}}\nopen({:?}, 'w').write('x')\n```\n",
        marker.to_string_lossy()
    );
    std::fs::write(dir.join("talks/deck.qmd"), &deck).unwrap();
    git(&["add", "."]);
    git(&["commit", "-qm", "then"]);
    std::fs::write(
        dir.join("talks/deck.qmd"),
        "---\ntitle: Now\n---\n\n## New\n",
    )
    .unwrap();
    git(&["commit", "-qam", "now"]);

    let cache = dir.join(".cache");
    let html = quartz_deck::history::render_version(&dir.join("talks/deck.qmd"), &cache, "HEAD~1")
        .unwrap();
    let page = std::fs::read_to_string(&html).unwrap();
    assert!(
        page.contains("Then") && page.contains("id=\"old\""),
        "the old version"
    );
    assert!(!marker.exists(), "no code ran (FR-005)");
    assert!(
        html.starts_with(cache.join("history")),
        "rendered in the cache"
    );
    let again = quartz_deck::history::render_version(&dir.join("talks/deck.qmd"), &cache, "HEAD~1")
        .unwrap();
    assert_eq!(again, html, "reused");
    assert!(
        std::fs::read_dir(dir.join("talks")).unwrap().count() == 1,
        "nothing written beside the deck"
    );
}
