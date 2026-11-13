//! 013 US3 (T009): cells run with the deck's own engine: a named Jupyter
//! kernel, or R for knitr decks. Needs uv (and R for the knitr tests); skips
//! without them.

mod common;

use common::have;
use serde_json::{Value, json};
use std::path::Path;

/// `qd <args>` in `dir` with extra environment: (exit ok, the result on stdout,
/// else the error on stderr: a cell that fails or stops has both).
fn qd(dir: &Path, env: &[(&str, &Path)], args: &[&str]) -> (bool, Value) {
    let mut c = common::qd(dir);
    for (k, v) in env {
        c.env(k, v);
    }
    let out = c.args(args).output().unwrap();
    let json = serde_json::from_slice(&out.stdout)
        .or_else(|_| serde_json::from_slice(&out.stderr))
        .unwrap_or(Value::Null);
    (out.status.success(), json)
}

fn deck(dir: &Path, text: &str) {
    std::fs::write(dir.join("deck.qmd"), text).unwrap();
}

#[test]
fn named_kernel() {
    if !have("uv") {
        return;
    }
    let t = tempfile::tempdir().unwrap();
    let dir = t.path();
    // a Jupyter data folder with one more kernel: Python, marked by an env variable
    let jp = dir.join("jupyter");
    let spec = jp.join("kernels/qd-test");
    std::fs::create_dir_all(&spec).unwrap();
    let k = json!({"argv": ["python", "-m", "ipykernel_launcher", "-f", "{connection_file}"],
        "display_name": "qd test", "language": "python", "env": {"QD_TEST_KERNEL": "yes"}});
    std::fs::write(spec.join("kernel.json"), k.to_string()).unwrap();
    deck(
        dir,
        "---\ntitle: T\njupyter: qd-test\n---\n\n## A {#a}\n\n```{python}\nimport os\nprint(os.environ.get('QD_TEST_KERNEL'))\n```\n",
    );
    let env = [("JUPYTER_PATH", jp.as_path())];
    assert!(qd(dir, &env, &["env", "init"]).0);
    let (ok, r) = qd(dir, &env, &["run", "a/0"]);
    assert!(ok, "{r}");
    assert_eq!(r["runs"][0]["outputs"][0]["text"], "yes\n", "{r}");
    assert_eq!(r["engine"]["kind"], "jupyter");
    assert_eq!(r["engine"]["kernel"], "qd-test");
}

#[test]
fn missing_kernel() {
    if !have("uv") {
        return;
    }
    let t = tempfile::tempdir().unwrap();
    let dir = t.path();
    deck(
        dir,
        "---\ntitle: T\njupyter: nosuch\n---\n\n## A {#a}\n\n```{python}\n1\n```\n",
    );
    assert!(qd(dir, &[], &["env", "init"]).0);
    let (ok, r) = qd(dir, &[], &["run", "a/0"]);
    assert!(!ok);
    let e = &r["error"];
    assert_eq!(e["code"], "E_ENGINE", "{r}");
    let msg = e["message"].as_str().unwrap();
    assert!(msg.contains("nosuch") && msg.contains("qd-python"), "{msg}");
}

const R_DECK: &str = "---\ntitle: R\nformat: revealjs\n---\n\n## A {#a}\n\n```{r}\nx <- 20\n```\n\n## B {#b}\n\n```{r}\nx + 22\n```\n\n## C {#c}\n\n```{r}\nplot(1:10)\n```\n\n## Slow {#slow}\n\n```{r}\nSys.sleep(30)\n```\n";

#[test]
fn knitr_cells() {
    if !have("Rscript") || !have("quarto") {
        return;
    }
    let t = tempfile::tempdir().unwrap();
    let dir = t.path();
    deck(dir, R_DECK);
    // one session: the second cell sees the first's variable
    let (ok, r) = qd(dir, &[], &["run", "a/0", "b/0", "c/0"]);
    assert!(ok, "{r}");
    assert_eq!(r["engine"]["kind"], "knitr");
    assert!(
        r["runs"][1]["outputs"][0]["text"]
            .as_str()
            .unwrap()
            .contains("42"),
        "{r}"
    );
    let img = &r["runs"][2]["outputs"][0];
    assert_eq!(img["kind"], "image", "{r}");
    // the deck's figure size: 10 × 5 in at 96 dpi
    let png = std::fs::read(img["path"].as_str().unwrap()).unwrap();
    let px = |o: usize| u32::from_be_bytes(png[o..o + 4].try_into().unwrap());
    assert_eq!((px(16), px(20)), (960, 480));
    // --timeout stops a slow cell
    let (_, r) = qd(dir, &[], &["run", "slow/0", "--timeout", "2"]);
    assert_eq!(r["status"], "stopped", "{r}");
}

#[test]
fn no_r() {
    if have("Rscript") || !have("quarto") {
        return;
    }
    let t = tempfile::tempdir().unwrap();
    deck(t.path(), R_DECK);
    let (ok, r) = qd(t.path(), &[], &["run", "a/0"]);
    assert!(!ok);
    assert_eq!(r["error"]["code"], "E_ENGINE", "{r}");
    assert!(r["error"]["message"].as_str().unwrap().contains("R"), "{r}");
}

const JULIA_DECK: &str = "---\ntitle: J\nformat: revealjs\nengine: julia\n---\n\n## A {#a}\n\n```{julia}\nx = 20\n```\n\n## B {#b}\n\n```{julia}\nprintln(\"hi\")\nx + 22\n```\n\n## C {#c}\n\n```{julia}\nerror(\"boom\")\n```\n\n## Slow {#slow}\n\n```{julia}\nsleep(30)\n```\n";

/// 013 US5 (T015): Quarto's native Julia engine: one session (state carries),
/// printed output and the last value, errors, --timeout. Needs Julia.
#[test]
fn julia_cells() {
    if !have("julia") || !have("quarto") {
        return;
    }
    let t = tempfile::tempdir().unwrap();
    let dir = t.path();
    deck(dir, JULIA_DECK);
    let (ok, r) = qd(dir, &[], &["run", "a/0", "b/0"]);
    assert!(ok, "{r}");
    assert_eq!(r["engine"]["kind"], "julia");
    // what it printed, then its last value (as a notebook shows them)
    let texts: Vec<&str> = r["runs"][1]["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|o| o["text"].as_str())
        .collect();
    assert_eq!(texts, ["hi\n", "42\n"], "{r}");
    let (ok, r) = qd(dir, &[], &["run", "c/0"]);
    assert!(!ok);
    assert_eq!(r["runs"][0]["status"], "error", "{r}");
    assert!(
        r["runs"][0]["outputs"][0]["evalue"]
            .as_str()
            .unwrap()
            .contains("boom"),
        "{r}"
    );
    let (_, r) = qd(dir, &[], &["run", "slow/0", "--timeout", "2"]);
    assert_eq!(r["status"], "stopped", "{r}");
}

#[test]
fn no_julia() {
    if have("julia") || !have("quarto") {
        return;
    }
    let t = tempfile::tempdir().unwrap();
    deck(t.path(), JULIA_DECK);
    let (ok, r) = qd(t.path(), &[], &["run", "a/0"]);
    assert!(!ok);
    assert_eq!(r["error"]["code"], "E_ENGINE", "{r}");
    assert!(
        r["error"]["message"].as_str().unwrap().contains("Julia"),
        "{r}"
    );
}
