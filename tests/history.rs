//! 012 US1 (T004): `qd history` in temp git repositories (signing off
//! locally; tests/common `git_repo`). Rendering old versions needs Quarto.

mod common;
use common::{git_repo, have, hook, qd, settings};
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

/// 012 US3 (T012): `qd diff REV [REV]` lists the slides that changed.
#[test]
fn diff_versions() {
    let t = tempfile::tempdir().unwrap();
    five_commits(t.path());
    let kinds = |v: &Value| -> Vec<(String, String, Value, Value)> {
        v["slides"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| {
                let s = |k: &str| c[k].as_str().unwrap().to_string();
                (s("kind"), s("id"), c["from"].clone(), c["to"].clone())
            })
            .collect()
    };
    // against the file (equal to HEAD): the reorder's text, then two edited
    let (ok, v) = run(t.path(), &["diff", "HEAD~2"]);
    assert!(ok, "{v}");
    assert_eq!(v["kind"], "diff");
    assert_eq!(v["to"], "uncommitted");
    assert_eq!(
        kinds(&v),
        [("changed".into(), "two".into(), 1.into(), 1.into())]
    );
    assert!(v["slides"][0]["diff"].as_str().unwrap().contains("+B2"));
    // two versions, the older under its old name (renamed since)
    let (ok, v) = run(t.path(), &["diff", "HEAD~4", "HEAD~1"]);
    assert!(ok, "{v}");
    // one changed and moved below two; two kept its text, so it isn't listed
    // (a swap moves one slide, and the changed one is listed anyway)
    assert_eq!(
        kinds(&v),
        [("changed".into(), "one".into(), 1.into(), 2.into())]
    );
    // an uncommitted added slide
    let deck = t.path().join("talks/deck.qmd");
    let text = std::fs::read_to_string(&deck).unwrap();
    std::fs::write(&deck, format!("{text}\n## Three {{#three}}\n\nC\n")).unwrap();
    let (_, v) = run(t.path(), &["diff", "HEAD"]);
    assert_eq!(
        kinds(&v),
        [("added".into(), "three".into(), Value::Null, 3.into())]
    );
    let (ok, v) = run(t.path(), &["diff", "nosuch"]);
    assert!(!ok);
    assert_eq!(v["error"]["code"], "E_NOT_FOUND");
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

// ---------- US2: save as git commit ----------

fn commit(dir: &Path, deck: &str) -> quartz_deck::error::Result<Option<String>> {
    let repo = quartz_deck::history::repo(&dir.join(deck))
        .unwrap()
        .unwrap();
    quartz_deck::history::commit_deck(&repo, "qd: edit (agent)")
}

/// talks/deck.qmd committed once, plus `other.txt`.
fn one_commit(dir: &Path) -> impl Fn(&[&str]) -> String + '_ {
    let git = git_repo(dir);
    std::fs::create_dir_all(dir.join("talks")).unwrap();
    std::fs::write(dir.join("talks/deck.qmd"), "## One {#one}\n\nA\n").unwrap();
    std::fs::write(dir.join("other.txt"), "1\n").unwrap();
    git(&["add", "."]);
    git(&["commit", "-qm", "first"]);
    git
}

#[test]
fn commit_only_the_deck() {
    let t = tempfile::tempdir().unwrap();
    let dir = t.path();
    let git = one_commit(dir);
    assert_eq!(
        commit(dir, "talks/deck.qmd").unwrap(),
        None,
        "the deck equals HEAD"
    );
    std::fs::write(dir.join("other.txt"), "2\n").unwrap();
    git(&["add", "other.txt"]);
    std::fs::write(dir.join("other.txt"), "3\n").unwrap();
    std::fs::write(dir.join("talks/deck.qmd"), "## One {#one}\n\nB\n").unwrap();
    let rev = commit(dir, "talks/deck.qmd").unwrap().unwrap();
    assert_eq!(git(&["rev-parse", "HEAD"]).trim(), rev);
    assert_eq!(
        git(&["show", "--name-only", "--format=", "HEAD"]).trim(),
        "talks/deck.qmd"
    );
    assert_eq!(git(&["status", "--porcelain"]), "MM other.txt\n");
    // an untracked deck is added and committed
    std::fs::write(dir.join("talks/new.qmd"), "## New\n").unwrap();
    commit(dir, "talks/new.qmd").unwrap().unwrap();
    assert_eq!(
        git(&["show", "--name-only", "--format=", "HEAD"]).trim(),
        "talks/new.qmd"
    );
    assert_eq!(git(&["status", "--porcelain"]), "MM other.txt\n");
}

#[test]
fn commit_follows_git_config() {
    let t = tempfile::tempdir().unwrap();
    let dir = t.path();
    let git = one_commit(dir);
    let marker = dir.join("hook-ran");
    hook(dir, &format!("touch {marker:?}"));
    std::fs::write(dir.join("talks/deck.qmd"), "## One {#one}\n\nB\n").unwrap();
    commit(dir, "talks/deck.qmd").unwrap().unwrap();
    assert!(marker.exists(), "the person's hook ran");
    hook(dir, "echo 'no commits today' >&2; exit 1");
    std::fs::write(dir.join("talks/deck.qmd"), "## One {#one}\n\nC\n").unwrap();
    let e = commit(dir, "talks/deck.qmd").unwrap_err();
    assert_eq!(e.code(), "E_GIT");
    assert!(e.to_string().contains("no commits today"), "{e}");
    assert!(
        std::fs::read_to_string(dir.join("talks/deck.qmd"))
            .unwrap()
            .contains("C"),
        "the edit stays"
    );
    assert_eq!(git(&["log", "--format=%s"]).lines().count(), 2);
}

#[test]
fn no_commit_when_busy() {
    let t = tempfile::tempdir().unwrap();
    let dir = t.path();
    let git = one_commit(dir);
    git(&["checkout", "-qb", "side"]);
    std::fs::write(dir.join("other.txt"), "side\n").unwrap();
    git(&["commit", "-qam", "side"]);
    git(&["checkout", "-q", "main"]);
    std::fs::write(dir.join("other.txt"), "main\n").unwrap();
    git(&["commit", "-qam", "main"]);
    assert!(quartz_deck::history::git(dir, &["merge", "-q", "side"]).is_err());
    std::fs::write(dir.join("talks/deck.qmd"), "## One {#one}\n\nB\n").unwrap();
    let e = commit(dir, "talks/deck.qmd").unwrap_err();
    assert_eq!(e.code(), "E_REPO_BUSY");
    assert_eq!(git(&["log", "-1", "--format=%s"]).trim(), "main");
}

#[test]
fn cli_edit_commits_without_server() {
    let t = tempfile::tempdir().unwrap();
    let dir = t.path();
    let git = one_commit(dir);
    let (ok, v) = run(dir, &["title", "one", "X"]);
    assert!(ok && v.get("commit").is_none(), "off by default: {v}");
    settings(dir, serde_json::json!({"history.commit": true}));
    let (ok, v) = run(dir, &["title", "one", "Y"]);
    assert!(ok, "{v}");
    assert_eq!(
        v["commit"]["rev"].as_str().unwrap(),
        git(&["rev-parse", "HEAD"]).trim()
    );
    assert_eq!(
        git(&["log", "-1", "--format=%s"]).trim(),
        "qd: title one ×2 (agent)",
        "both edits since the last commit"
    );
    // outside git: no commit, no error
    let out = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(out.path().join("talks")).unwrap();
    std::fs::write(out.path().join("talks/deck.qmd"), "## One {#one}\n").unwrap();
    settings(out.path(), serde_json::json!({"history.commit": true}));
    let (ok, v) = run(out.path(), &["title", "one", "Z"]);
    assert!(ok && v.get("commit").is_none(), "{v}");
}

#[test]
fn serve_commits_after_a_pause() {
    if !have("quarto") {
        return;
    }
    let t = tempfile::tempdir().unwrap();
    let dir = t.path();
    let git = one_commit(dir);
    settings(
        dir,
        serde_json::json!({"history.commit": true, "history.pause": 5}),
    );
    let mut child = qd(dir)
        .args(["--deck", "talks/deck.qmd", "serve", "--port", "0"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let mut line = String::new();
    std::io::BufRead::read_line(
        &mut std::io::BufReader::new(child.stdout.take().unwrap()),
        &mut line,
    )
    .unwrap();
    let count = || {
        git(&["rev-list", "--count", "HEAD"])
            .trim()
            .parse::<u32>()
            .unwrap()
    };
    for x in ["A", "B", "C"] {
        assert!(run(dir, &["title", "one", x]).0);
    }
    std::thread::sleep(std::time::Duration::from_secs(2));
    assert_eq!(count(), 1, "not before the pause");
    std::thread::sleep(std::time::Duration::from_secs(5));
    assert_eq!(count(), 2, "one commit for the burst");
    assert_eq!(
        git(&["log", "-1", "--format=%s"]).trim(),
        "qd: title one ×3 (agent)"
    );
    assert!(run(dir, &["title", "one", "D"]).0);
    std::thread::sleep(std::time::Duration::from_secs(1));
    let (ok, v) = run(dir, &["stop"]);
    assert!(ok, "{v}");
    child.wait().unwrap();
    assert_eq!(count(), 3, "stopping commits what is pending");
    assert!(git(&["show", "HEAD:talks/deck.qmd"]).contains("## D"));
}
