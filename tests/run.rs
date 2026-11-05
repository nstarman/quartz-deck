//! The live kernel session (spec 004 T019): the embedded bridge in the deck's
//! environment. Needs uv (and network for the first lock); skips without it.

mod common;

use quartz_deck::{env, kernel::Session};
use serde_json::Value;
use std::time::Duration;

fn kinds(r: &Value) -> Vec<&str> {
    r["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["kind"].as_str().unwrap())
        .collect()
}

#[test]
fn session_runs_cells_keeps_state_interrupts_and_restarts() {
    if !common::have("uv") {
        return;
    }
    let d = tempfile::tempdir().unwrap();
    // SAFETY: the only test in this binary; set before any thread reads it
    unsafe { std::env::set_var("QD_CACHE_DIR", d.path().join(".cache")) };
    let e = env::init(d.path(), env::Tool::Uv).unwrap();
    e.add(&["matplotlib".into()]).unwrap();
    let s = Session::start(&e, d.path()).unwrap();

    let r = s.execute("x = 21 * 2\nprint(x)", None).unwrap();
    assert_eq!(
        (r["status"].as_str(), r["outputs"][0]["text"].as_str()),
        (Some("ok"), Some("42\n")),
        "{r}"
    );
    let r = s
        .execute("import matplotlib.pyplot as plt\nplt.plot([1, 3, 2])", None)
        .unwrap();
    assert!(kinds(&r).contains(&"image"), "{r}");
    let fig = r["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["kind"] == "image")
        .unwrap();
    assert!(
        std::path::Path::new(fig["path"].as_str().unwrap()).is_file(),
        "{r}"
    );
    let r = s.execute("x + 1", None).unwrap();
    assert_eq!(r["outputs"][0]["text"], "43", "state is kept: {r}");
    let r = s.execute("1 / 0", None).unwrap();
    assert_eq!(
        (r["status"].as_str(), r["outputs"][0]["ename"].as_str()),
        (Some("error"), Some("ZeroDivisionError"))
    );
    assert!(
        r["outputs"][0]["traceback"]
            .as_array()
            .unwrap()
            .iter()
            .any(|l| l.as_str().unwrap().contains("1 / 0"))
    );
    assert!(
        !r.to_string().contains('\u{1b}'),
        "no terminal colour codes"
    );

    // Stop: from another thread while the cell runs, and by timeout
    let r = std::thread::scope(|t| {
        t.spawn(|| {
            std::thread::sleep(Duration::from_secs(1));
            s.interrupt().unwrap();
        });
        s.execute("while True:\n    pass", None).unwrap()
    });
    assert_eq!(r["status"], "stopped", "{r}");
    let r = s
        .execute("while True:\n    pass", Some(Duration::from_secs(1)))
        .unwrap();
    assert_eq!(r["status"], "stopped", "{r}");

    s.restart().unwrap();
    let r = s.execute("x", None).unwrap();
    assert_eq!(
        r["outputs"][0]["ename"], "NameError",
        "restart clears state: {r}"
    );
    drop(s);

    // a hand-edited manifest: the session refuses to start on a stale lock
    let m = e.manifest();
    let text = std::fs::read_to_string(&m)
        .unwrap()
        .replace("\"pyyaml", "\"tomli\",\n    \"pyyaml");
    std::fs::write(&m, text).unwrap();
    let err = Session::start(&e, d.path())
        .err()
        .expect("stale lock refused");
    assert_eq!(err.code(), "E_LOCK_STALE", "{err}");
}
