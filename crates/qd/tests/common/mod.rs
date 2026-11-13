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

/// A `pre-commit` hook running `script` in the repository at `dir`.
pub fn hook(dir: &Path, script: &str) {
    use std::os::unix::fs::PermissionsExt;
    let hook = dir.join(".git/hooks/pre-commit");
    std::fs::write(&hook, format!("#!/bin/sh\n{script}\n")).unwrap();
    std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// `qd args…` in `dir`: whether it succeeded, and its JSON (stdout, or stderr
/// on failure); panics on output that isn't JSON.
pub fn json(dir: &Path, args: &[&str]) -> (bool, serde_json::Value) {
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
