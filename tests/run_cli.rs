//! `qd run` / `qd outputs` (spec 004 US4, T029): one-shot without a server,
//! the live session with one. Needs uv; skips without it.

mod common;

use common::have;
use serde_json::Value;
use std::path::Path;

/// `qd <args>` in `dir`: (exit ok, stdout JSON, stderr JSON).
fn qd(dir: &Path, args: &[&str]) -> (bool, Value, Value) {
    let out = common::qd(dir).args(args).output().unwrap();
    let json = |b: &[u8]| serde_json::from_slice(b).unwrap_or(Value::Null);
    (out.status.success(), json(&out.stdout), json(&out.stderr))
}

#[test]
fn agents_run_cells_from_the_cli() {
    if !have("uv") {
        return;
    }
    let d = tempfile::tempdir().unwrap();
    let dir = d.path();
    let deck = dir.join("deck.qmd");
    std::fs::write(&deck, include_str!("fixtures/run/basic.qmd")).unwrap();
    assert!(qd(dir, &["env", "init"]).0);
    assert!(qd(dir, &["env", "add", "matplotlib"]).0);
    let before = std::fs::read(&deck).unwrap();

    // no server: a fresh one-shot session runs from the top through the cell
    let (ok, r, e) = qd(dir, &["run", "state/0"]);
    assert!(ok, "{r} {e}");
    assert_eq!(r["mode"], "fresh");
    let cells: Vec<&str> = r["runs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["cell"].as_str().unwrap())
        .collect();
    assert_eq!(cells, ["print/0", "plot/0", "state/0"], "{r}");
    assert_eq!(r["runs"][2]["outputs"][0]["text"], "43", "{r}");
    assert_eq!(r["runs"][2]["status"], "done");
    let img = r["runs"][1]["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["kind"] == "image")
        .unwrap_or_else(|| panic!("no image: {r}"));
    assert!(Path::new(img["path"].as_str().unwrap()).is_file(), "{img}");

    // --all stops at the first error: result on stdout, E_RUN on stderr, exit 1
    let (ok, r, e) = qd(dir, &["run", "--all"]);
    assert!(!ok && e["error"]["code"] == "E_RUN", "{e}");
    assert_eq!(r["ok"], false);
    let last = r["runs"].as_array().unwrap().last().unwrap();
    assert_eq!(
        (last["cell"].as_str(), last["status"].as_str()),
        (Some("fails/0"), Some("error"))
    );
    assert_eq!(last["outputs"][0]["ename"], "ZeroDivisionError");

    for (cell, code) in [("plain/0", "E_NOT_CODE"), ("r/0", "E_ENGINE")] {
        let (ok, _, e) = qd(dir, &["run", cell]);
        assert!(!ok && e["error"]["code"] == code, "{cell}: {e}");
    }

    // stored outputs, marked stale once the cell changes
    let (ok, r, _) = qd(dir, &["outputs", "print/0"]);
    assert!(ok && r["outputs"][0]["stale"] == false, "{r}");
    assert!(qd(dir, &["replace", "--old", "21 * 2", "--new", "21 * 3"]).0);
    let (_, r, _) = qd(dir, &["outputs"]);
    let print = r["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["cell"] == "print/0")
        .unwrap();
    assert_eq!(print["stale"], true, "{r}");
    assert!(qd(dir, &["undo"]).0);

    // with `qd serve` running: its live session, state kept between calls
    let (ok, s, e) = qd(dir, &["serve", "--detach"]);
    assert!(ok, "{s} {e}");
    let (ok, r, e) = qd(dir, &["run", "print/0"]);
    assert!(ok && r["mode"] == "live", "{r} {e}");
    assert_eq!(r["runs"][0]["outputs"][0]["text"], "42\n", "{r}");
    let (ok, r, _) = qd(dir, &["run", "state/0"]);
    assert!(ok && r["runs"].as_array().unwrap().len() == 1, "{r}");
    assert_eq!(r["runs"][0]["outputs"][0]["text"], "43", "live state: {r}");
    let (ok, r, _) = qd(dir, &["run", "state/0", "--fresh"]);
    assert!(ok && r["runs"].as_array().unwrap().len() == 3, "{r}");
    assert!(qd(dir, &["run", "--timeout", "30", "print/0"]).0);
    assert!(qd(dir, &["stop"]).0);

    assert_eq!(
        std::fs::read(&deck).unwrap(),
        before,
        "runs never touch the .qmd"
    );
}
