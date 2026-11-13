//! `qd env` and rendering through the deck's environment (spec 004 T015/T017).
//! Needs uv and network for the first lock (Quarto too for rendering); pixi
//! variants need pixi. Each skips with a message when its tool is missing.

mod common;

use common::have;
use quartz_deck::env::BASE;
use serde_json::Value;
use std::path::Path;

/// `qd <args>` on the deck in `dir`; (exit ok, JSON).
fn qd(dir: &Path, args: &[&str]) -> (bool, Value) {
    let out = common::qd(dir).args(args).output().unwrap();
    let ok = out.status.success();
    let text = if ok { &out.stdout } else { &out.stderr };
    (ok, serde_json::from_slice(text).unwrap_or(Value::Null))
}

/// A deck with one Python cell that prints 42.
fn deck() -> tempfile::TempDir {
    let d = tempfile::tempdir().unwrap();
    let text =
        "---\ntitle: T\nformat: revealjs\n---\n\n## Code\n\n```{python}\nprint(6 * 7)\n```\n";
    std::fs::write(d.path().join("deck.qmd"), text).unwrap();
    d
}

#[test]
fn uv_environment_lifecycle() {
    if !have("uv") {
        return;
    }
    let d = deck();
    let (ok, e) = qd(d.path(), &["env"]);
    assert!(!ok && e["error"]["code"] == "E_NO_ENV", "{e}");

    let (ok, r) = qd(d.path(), &["env", "init"]);
    assert!(ok, "{r}");
    assert!(r["tool"] == "uv" && r["lock_current"] == true, "{r}");
    let manifest = std::fs::read_to_string(d.path().join("pyproject.toml")).unwrap();
    for pkg in BASE {
        assert!(manifest.contains(pkg), "{pkg} missing from:\n{manifest}");
    }
    let (ok, e) = qd(d.path(), &["env", "init"]);
    assert!(!ok && e["error"]["code"] == "E_INVALID_ARG", "{e}");

    // add relocks
    let (ok, r) = qd(d.path(), &["env", "add", "six"]);
    assert!(ok, "{r}");
    let lock = std::fs::read_to_string(d.path().join("uv.lock")).unwrap();
    assert!(lock.contains("name = \"six\""));
    let (ok, e) = qd(d.path(), &["env", "add", "--index-url=x"]);
    assert!(!ok, "{e}");

    // a fresh copy reproduces exactly the locked versions (SC-007)
    let copy = tempfile::tempdir().unwrap();
    for f in ["deck.qmd", "pyproject.toml", "uv.lock"] {
        std::fs::copy(d.path().join(f), copy.path().join(f)).unwrap();
    }
    let (ok, r) = qd(copy.path(), &["env", "sync"]);
    assert!(ok, "{r}");
    assert_eq!(
        std::fs::read_to_string(copy.path().join("uv.lock")).unwrap(),
        lock
    );

    // a hand-edited manifest makes the lock stale: reported, and code won't run
    let edited = manifest.replace("\"pyyaml", "\"tomli\",\n    \"pyyaml");
    assert_ne!(edited, manifest);
    std::fs::write(d.path().join("pyproject.toml"), edited).unwrap();
    assert_eq!(qd(d.path(), &["env"]).1["lock_current"], false);
    if have("quarto") {
        let (ok, e) = qd(d.path(), &["render"]);
        assert!(!ok && e["error"]["code"] == "E_LOCK_STALE", "{e}");
    }
    let (ok, r) = qd(d.path(), &["env", "sync"]);
    assert!(ok && r["lock_current"] == true, "{r}");
}

#[test]
fn code_cells_render_through_the_environment() {
    if !have("uv") || !have("quarto") {
        return;
    }
    let d = deck();
    assert!(qd(d.path(), &["env", "init"]).0);
    let (ok, r) = qd(d.path(), &["render"]);
    assert!(ok, "{r}");
    let html = std::fs::read_to_string(r["output"].as_str().unwrap()).unwrap();
    assert!(
        html.contains("42"),
        "the cell's printed value is in the slides"
    );
}

#[test]
fn pixi_environment_has_pypi_packages_only() {
    if !have("pixi") {
        return;
    }
    let d = deck();
    let (ok, r) = qd(d.path(), &["env", "init", "--tool", "pixi"]);
    assert!(ok, "{r}");
    assert!(r["tool"] == "pixi" && r["lock_current"] == true, "{r}");
    let manifest = std::fs::read_to_string(d.path().join("pixi.toml")).unwrap();
    let pypi = manifest
        .split("[pypi-dependencies]")
        .nth(1)
        .expect("PyPI section");
    for pkg in BASE {
        assert!(
            pypi.contains(pkg),
            "{pkg} not a PyPI dependency:\n{manifest}"
        );
    }
    let conda = manifest.split("[dependencies]").nth(1).unwrap_or("");
    let conda = conda.split("\n[").next().unwrap();
    assert!(
        conda
            .lines()
            .filter(|l| l.contains('='))
            .all(|l| l.starts_with("python")),
        "only Python itself comes from conda-forge:\n{manifest}"
    );
}
