//! US1 (T010, T011): `qd check --offline` on the fixture decks. Needs Quarto
//! and a browser (installed Chrome, QD_CHROME, or qd's cached download); skips
//! otherwise. Downloads are disabled here so tests never fetch a browser.

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

const FIX: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/offline");

fn qd(dir: &Path) -> Command {
    let mut c = Command::new(assert_cmd::cargo::cargo_bin!("qd"));
    c.current_dir(dir)
        .env_remove("QD_DECK")
        .env("QD_NO_DOWNLOAD", "1");
    c
}

fn ready() -> bool {
    let quarto = Command::new("quarto")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    let tmp = tempfile::tempdir().unwrap();
    let browser = qd(tmp.path())
        .arg("browser")
        .output()
        .is_ok_and(|o| o.status.success());
    if !(quarto && browser) {
        eprintln!("skipping: quarto={quarto} browser={browser}");
    }
    quarto && browser
}

/// Copy a fixture deck (and the files it uses) into a temp dir.
fn deck(name: &str) -> (tempfile::TempDir, PathBuf) {
    let d = tempfile::tempdir().unwrap();
    std::fs::copy(Path::new(FIX).join(name), d.path().join("deck.qmd")).unwrap();
    for sub in ["media", "apps"] {
        let src = Path::new(FIX).join(sub);
        std::fs::create_dir_all(d.path().join(sub)).unwrap();
        for e in std::fs::read_dir(src).unwrap() {
            let e = e.unwrap();
            std::fs::copy(e.path(), d.path().join(sub).join(e.file_name())).unwrap();
        }
    }
    let p = d.path().to_path_buf();
    (d, p)
}

/// Run `qd check --offline`; returns (exit code, stdout JSON, stderr JSON).
fn check(dir: &Path, extra: &[&str]) -> (i32, Value, Value) {
    let out = qd(dir)
        .env("QD_CACHE_DIR", dir.join(".cache"))
        .args(["check", "--offline"])
        .args(extra)
        .output()
        .unwrap();
    let parse = |b: &[u8]| serde_json::from_slice(b).unwrap_or(Value::Null);
    (
        out.status.code().unwrap(),
        parse(&out.stdout),
        parse(&out.stderr),
    )
}

fn codes(report: &Value) -> Vec<(String, Value)> {
    report["problems"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| (p["code"].as_str().unwrap().to_string(), p["slide"].clone()))
        .collect()
}

#[test]
fn default_deck_with_math_fails_offline() {
    if !ready() {
        return;
    }
    let (_t, dir) = deck("math.qmd");
    let (code, report, err) = check(&dir, &[]);
    assert_eq!(code, 1, "{report}");
    assert_eq!(err["error"]["code"], "E_OFFLINE");
    assert_eq!(report["ok"], false);
    assert!(
        report["html"].as_str().unwrap().ends_with("deck.qmd"),
        "names the deck it checked: {report}"
    );
    let leftovers: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains(".qd-"))
        .collect();
    assert!(
        leftovers.is_empty(),
        "temporary render left behind: {leftovers:?}"
    );
    let probs = report["problems"].as_array().unwrap();
    assert!(
        probs
            .iter()
            .any(|p| p["code"] == "P_NETWORK" && p["url"].as_str().unwrap().contains("mathjax")),
        "web MathJax request not reported: {report}"
    );
    assert!(
        codes(&report).contains(&("P_RAW_TEX".into(), Value::from("math"))),
        "raw TeX on 'math' not reported: {report}"
    );
    assert!(
        !codes(&report)
            .iter()
            .any(|(c, s)| c == "P_RAW_TEX" && s == "plain"),
        "no math on 'plain': {report}"
    );
    assert!(
        probs
            .iter()
            .all(|p| !p["url"].as_str().unwrap_or("").starts_with("about:blank")),
        "about:blank is by design (FR-003)"
    );
    for p in probs {
        for k in ["code", "severity", "message", "hint"] {
            assert!(p[k].is_string(), "problem lacks {k}: {p}");
        }
    }
}

#[test]
fn counts_slides_and_steps() {
    if !ready() {
        return;
    }
    let (_t, dir) = deck("fragments.qmd");
    let (_, report, _) = check(&dir, &[]);
    // title slide + 3 slides; steps = slides + 2 fragment steps
    assert_eq!(report["slides"], 4, "{report}");
    assert_eq!(report["steps"], 6, "{report}");
}

#[test]
fn missing_file_page_error_and_visible_notes() {
    if !ready() {
        return;
    }
    let (_t, dir) = deck("missing.qmd");
    let (_, report, _) = check(&dir, &[]);
    let p = report["problems"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["code"] == "P_MISSING_FILE")
        .unwrap_or_else(|| panic!("{report}"));
    assert_eq!(p["slide"], "gone");
    assert!(p["url"].as_str().unwrap().ends_with("media/not-there.mp4"));

    let (_t, dir) = deck("throws.qmd");
    let (_, report, _) = check(&dir, &[]);
    assert!(
        codes(&report).iter().any(|(c, _)| c == "P_PAGE_ERROR"),
        "{report}"
    );

    let (_t, dir) = deck("shownotes.qmd");
    let (code, report, _) = check(&dir, &[]);
    let p = report["problems"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["code"] == "P_NOTES_VISIBLE")
        .unwrap_or_else(|| panic!("{report}"));
    assert_eq!(p["severity"], "warning");
    // a warning alone does not fail the check
    let errors = report["problems"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["severity"] == "error")
        .count();
    assert_eq!(report["ok"], errors == 0);
    assert_eq!(code, if errors == 0 { 0 } else { 1 });
}

#[test]
fn checks_any_html_file() {
    if !ready() {
        return;
    }
    let (_t, dir) = deck("bgvideo.qmd");
    let out = Command::new("quarto")
        .args(["render", "deck.qmd", "--to", "revealjs"])
        .current_dir(&dir)
        .output()
        .unwrap();
    assert!(out.status.success());
    let (_, report, _) = check(&dir, &["--html", "deck.html"]);
    assert_eq!(
        report["html"].as_str().map(|h| h.ends_with("deck.html")),
        Some(true),
        "{report}"
    );
    assert!(
        !codes(&report).iter().any(|(c, _)| c == "P_MISSING_FILE"),
        "clip.mp4 is present: {report}"
    );
}

#[test]
fn no_browser_is_a_clear_error_and_editing_still_works() {
    let (_t, dir) = deck("math.qmd");
    let out = qd(&dir)
        .env("QD_CHROME", "/nonexistent/chrome")
        .args(["check", "--offline"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let err: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(err["error"]["code"], "E_NO_BROWSER");
    let out = qd(&dir)
        .env("QD_CHROME", "/nonexistent/chrome")
        .args(["title", "math", "Still editable"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let out = qd(&dir)
        .env("QD_CHROME", "/nonexistent/chrome")
        .arg("browser")
        .output()
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&out.stderr).unwrap()["error"]["code"],
        "E_NO_BROWSER"
    );
}
