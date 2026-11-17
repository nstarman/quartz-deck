//! Helpers shared by the integration tests that run `qd` against Quarto.
#![allow(dead_code)] // each test binary uses some of these

use std::path::Path;
use std::process::Command;

/// `qd` in `dir`, with its own cache and no inherited deck.
pub fn qd(dir: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_qd"));
    c.current_dir(dir)
        .env("QD_CACHE_DIR", dir.join(".cache"))
        .env("QD_CONFIG_DIR", dir.join(".config"))
        .env_remove("QD_DECK");
    c
}

/// `tool` (quarto, uv, pixi) is installed; tests that need it skip, with a message, when not.
pub fn have(tool: &str) -> bool {
    let ok = Command::new(tool)
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    if !ok {
        eprintln!("{tool} not found: skipping");
    }
    ok
}

/// qd's temp files (`.qd-` in the name) left in `dir` or `dir/export`;
/// the offline export's `.qd-export` marker is meant to stay.
pub fn leftovers(dir: &Path) -> Vec<String> {
    [dir.to_path_buf(), dir.join("export")]
        .iter()
        .flat_map(|d| std::fs::read_dir(d).into_iter().flatten().flatten())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains(".qd-") && n != ".qd-export")
        .collect()
}

/// A git repository in `dir` with a local user and signing off (so the
/// person's own signing never prompts during tests); returns a runner of
/// `git -C dir args…` that panics on failure and gives stdout.
pub fn git_repo(dir: &Path) -> impl Fn(&[&str]) -> String + '_ {
    let git = move |args: &[&str]| quartz_deck::history::git(dir, args).unwrap();
    git(&["init", "-q", "-b", "main"]);
    for (k, v) in [
        ("user.name", "Tester"),
        ("user.email", "tester@example.com"),
        ("commit.gpgsign", "false"),
    ] {
        git(&["config", k, v]);
    }
    git
}

/// The person's settings for `qd(dir)` (its `QD_CONFIG_DIR`).
pub fn settings(dir: &Path, v: serde_json::Value) {
    std::fs::create_dir_all(dir.join(".config")).unwrap();
    std::fs::write(dir.join(".config/settings.json"), v.to_string()).unwrap();
}

/// An executable shell script at `path` running `body`.
pub fn script(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// A `pre-commit` hook running `body` in the repository at `dir`.
pub fn hook(dir: &Path, body: &str) {
    script(&dir.join(".git/hooks/pre-commit"), body);
}

/// `qd args…` in `dir`: whether it succeeded, and its JSON (stdout, or stderr
/// on failure); panics on output that isn't JSON.
pub fn json(dir: &Path, args: &[&str]) -> (bool, serde_json::Value) {
    json_of(qd(dir), args)
}

/// The command fails with this error code.
pub fn err(dir: &Path, args: &[&str], code: &str) {
    let (good, v) = json(dir, args);
    assert!(!good, "{args:?} should fail: {v}");
    assert_eq!(v["error"]["code"], code, "{args:?}: {v}");
}

/// `json`, with `bin` first on `PATH` (stub plugins).
pub fn with_path(dir: &Path, bin: &Path, args: &[&str]) -> (bool, serde_json::Value) {
    let mut c = qd(dir);
    c.env(
        "PATH",
        format!("{}:{}", bin.display(), std::env::var("PATH").unwrap()),
    );
    json_of(c, args)
}

/// `json`, with a prepared `qd` command (extra environment).
pub fn json_of(mut c: Command, args: &[&str]) -> (bool, serde_json::Value) {
    let out = c.args(args).output().unwrap();
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

/// A deck `talk.qmd` holding `text`, in a new folder.
pub fn deck(text: &str) -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    std::fs::write(t.path().join("talk.qmd"), text).unwrap();
    t
}

/// The deck `talk.qmd` in `t` now.
pub fn text(t: &tempfile::TempDir) -> String {
    std::fs::read_to_string(t.path().join("talk.qmd")).unwrap()
}

/// quarto-arrows (spec 016) from one shared download, in `dir`'s qd cache
/// and, with `in_deck`, in its `_extensions/` too; `None` offline.
pub fn seed_arrows(dir: &Path, in_deck: bool) -> Option<()> {
    use quartz_deck::assets::{QUARTO_ARROWS, ensure};
    let shared = std::path::PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("qd-arrows");
    for f in QUARTO_ARROWS {
        let a = ensure(&shared, f, true)
            .inspect_err(|e| eprintln!("skipping: {e}"))
            .ok()?;
        let mut to = vec![
            dir.join(".cache")
                .join(a.path.strip_prefix(&shared).unwrap()),
        ];
        if in_deck {
            to.push(dir.join("_extensions/emilhvitfeldt/arrows").join(f.file));
        }
        for to in to {
            std::fs::create_dir_all(to.parent().unwrap()).unwrap();
            std::fs::copy(&a.path, &to).unwrap();
        }
    }
    Some(())
}
