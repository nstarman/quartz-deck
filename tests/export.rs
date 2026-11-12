//! US2/US3 (T015, T019): `qd export` and `qd pdf` on the offline fixtures.
//! Needs Quarto, a browser, and MathJax (cached or downloadable); skips
//! otherwise. Browser downloads are disabled; the MathJax cache is shared.

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

const FIX: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/offline");

fn cache() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("qd-export-cache")
}

fn qd(dir: &Path) -> Command {
    let mut c = Command::new(assert_cmd::cargo::cargo_bin!("qd"));
    c.current_dir(dir)
        .env_remove("QD_DECK")
        .env("QD_NO_DOWNLOAD", "1")
        .env("QD_CACHE_DIR", cache());
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
    // MathJax: cached already, or fetchable now
    let mathjax =
        quartz_deck::assets::ensure(&cache(), &quartz_deck::assets::MATHJAX, true).is_ok();
    if !(quarto && browser && mathjax) {
        eprintln!("skipping: quarto={quarto} browser={browser} mathjax={mathjax}");
    }
    quarto && browser && mathjax
}

fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for e in std::fs::read_dir(src).unwrap() {
        let e = e.unwrap();
        let to = dst.join(e.file_name());
        if e.path().is_dir() {
            copy_dir(&e.path(), &to);
        } else {
            std::fs::copy(e.path(), &to).unwrap();
        }
    }
}

fn deck(name: &str) -> (tempfile::TempDir, PathBuf) {
    let d = tempfile::tempdir().unwrap();
    std::fs::copy(Path::new(FIX).join(name), d.path().join("deck.qmd")).unwrap();
    copy_dir(&Path::new(FIX).join("media"), &d.path().join("media"));
    copy_dir(&Path::new(FIX).join("apps"), &d.path().join("apps"));
    copy_dir(&Path::new(FIX).join("figures"), &d.path().join("figures"));
    let p = d.path().to_path_buf();
    (d, p)
}

/// Run qd; (exit code, stdout JSON, stderr JSON).
fn run(dir: &Path, args: &[&str]) -> (i32, Value, Value) {
    let out = qd(dir).args(args).output().unwrap();
    let parse = |b: &[u8]| serde_json::from_slice(b).unwrap_or(Value::Null);
    (
        out.status.code().unwrap(),
        parse(&out.stdout),
        parse(&out.stderr),
    )
}

fn problem_codes(v: &Value) -> Vec<String> {
    v["check"]["problems"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["code"].as_str().unwrap().to_string())
        .collect()
}

fn leftovers(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|n| n.contains(".qd-"))
        .collect()
}

#[test]
fn exports_math_deck_offline() {
    if !ready() {
        return;
    }
    let (_t, dir) = deck("math.qmd");
    let before = std::fs::read(dir.join("deck.qmd")).unwrap();
    let (code, v, err) = run(&dir, &["export"]);
    assert_eq!(code, 0, "{v}\n{err}");
    assert_eq!(v["kind"], "export");
    assert_eq!(v["check"]["ok"], true, "{v}");
    assert!(problem_codes(&v).is_empty(), "{v}");
    let html = PathBuf::from(v["html"].as_str().unwrap());
    assert_eq!(
        html,
        std::fs::canonicalize(&dir)
            .unwrap()
            .join("export/deck.html")
    );
    assert!(html.exists() && dir.join("export/.qd-export").exists());
    assert_eq!(
        std::fs::read(dir.join("deck.qmd")).unwrap(),
        before,
        "deck must not change (FR-012)"
    );
    assert!(leftovers(&dir).is_empty(), "{:?}", leftovers(&dir));
    let assets = v["assets"].as_array().unwrap();
    assert!(
        assets
            .iter()
            .any(|a| a["name"] == "mathjax" && a["version"] == "3.2.2"),
        "{v}"
    );
    // the exported file passes a standalone offline check too
    let (code, report, _) = run(&dir, &["check", "--offline", "--html", "export/deck.html"]);
    assert_eq!(
        (code, report["ok"].clone()),
        (0, Value::Bool(true)),
        "{report}"
    );
}

#[test]
fn ships_files_that_cannot_be_inlined() {
    if !ready() {
        return;
    }
    let (_t, dir) = deck("bgvideo.qmd");
    let (code, v, err) = run(&dir, &["export"]);
    assert_eq!(code, 0, "{v}\n{err}");
    assert_eq!(v["files"], serde_json::json!(["media/clip.mp4"]), "{v}");
    assert!(dir.join("export/media/clip.mp4").exists());
    assert!(!problem_codes(&v).contains(&"P_MISSING_FILE".into()), "{v}");

    // a background iframe is inlined (spec 010), not shipped
    let (_t, dir) = deck("bgiframe.qmd");
    let (code, v, err) = run(&dir, &["export"]);
    assert_eq!(code, 0, "{v}\n{err}");
    assert_eq!(v["files"], serde_json::json!([]), "{v}");
    assert!(!dir.join("export/apps").exists());
    let html = std::fs::read_to_string(dir.join("export/deck.html")).unwrap();
    assert!(html.contains("id=\"inlined-app-0\""));
}

/// Spec 010 US1-US4: the one-file deck. The 2.5 MB app asks the web for an
/// image when it runs, so the blocked request proves it ran from its `blob:`
/// URL (in its own iframe target); that is the only error the export reports.
#[test]
fn one_file_export() {
    if !ready() {
        return;
    }
    let (_t, dir) = deck("onefile.qmd");
    let mut x = 7u32;
    let noise: String = (0..1_300_000)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            format!("{:02x}", x as u8)
        })
        .collect();
    std::fs::write(
        dir.join("apps/big.html"),
        format!("<!doctype html><p>big app</p><img src=\"https://qd.invalid/app-ran.png\"><!-- {noise} -->"),
    )
    .unwrap();
    assert!(std::fs::metadata(dir.join("apps/big.html")).unwrap().len() > 2_500_000);
    // a Quarto project, so Quarto copies the deck's resources into the output folder
    std::fs::write(dir.join("_quarto.yml"), "project:\n  type: default\n").unwrap();

    let (code, v, err) = run(&dir, &["export"]);
    assert_eq!(code, 1, "{v}\n{err}");
    let problems = v["check"]["problems"].as_array().unwrap();
    let errors: Vec<&Value> = problems
        .iter()
        .filter(|p| p["severity"] == "error")
        .collect();
    assert_eq!(errors.len(), 1, "{problems:#?}");
    assert_eq!(errors[0]["code"], "P_NETWORK");
    assert_eq!(errors[0]["slide"], "app");
    assert_eq!(errors[0]["url"], "https://qd.invalid/app-ran.png");
    // the box and its paragraph, as check_overflow.py lists them
    let overflow: Vec<&Value> = problems
        .iter()
        .filter(|p| p["code"] == "P_OVERFLOW")
        .collect();
    assert!(
        !overflow.is_empty() && overflow.iter().all(|p| p["slide"] == "wide"),
        "{problems:#?}"
    );
    let largest = v["check"]["size"]["largest"].as_array().unwrap();
    assert!(
        largest
            .iter()
            .any(|p| p["kind"] == "app apps/big.html (gzip)"),
        "{v}"
    );
    assert_eq!(v["files"], serde_json::json!([]));

    let staging = PathBuf::from(v["staging"].as_str().unwrap());
    let mut listed: Vec<String> = std::fs::read_dir(&staging)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    listed.sort();
    assert_eq!(
        listed,
        vec![".qd-export", "deck.html"],
        "Quarto's copies are pruned"
    );
    let html = std::fs::read_to_string(staging.join("deck.html")).unwrap();
    assert!(html.contains("id=\"inlined-app-0\"") && html.contains("id=\"inlined-app-1\""));
    assert!(html.contains("<object data=\"data:image/svg+xml;base64,"));
    assert!(
        html.contains("href=\"data:image/png;base64,"),
        "SVG <image> inlined"
    );
    assert_eq!(
        html.matches("data-dup-data-src=\"dup-").count(),
        2,
        "photo stored once"
    );
    assert!(html.contains("these are the speaker notes"));

    // the PDF of that file: -notes.pdf, recompressed when ghostscript is here
    let (code, p, err) = run(
        &dir,
        &[
            "pdf",
            "--notes",
            "--html",
            staging.join("deck.html").to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "{p}\n{err}");
    assert!(
        p["path"].as_str().unwrap().ends_with("deck-notes.pdf"),
        "{p}"
    );
    let gs = Command::new("gs")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success());
    assert_eq!(p["compressed"], gs, "{p}");
    assert!(p["pages"].as_u64().unwrap() > 6, "{p}");

    // a copy to share: no notes
    let (_, v, _) = run(&dir, &["export", "--strip-notes", "--out", "share"]);
    let html = std::fs::read_to_string(Path::new(v["staging"].as_str().unwrap()).join("deck.html"))
        .unwrap();
    assert!(!html.contains("these are the speaker notes"));
}

#[test]
fn missing_app_fails_the_export() {
    if !ready() {
        return;
    }
    let (_t, dir) = deck("onefile.qmd"); // apps/big.html is not written
    let (code, v, _) = run(&dir, &["export"]);
    assert_eq!(code, 1, "{v}");
    let p = v["check"]["problems"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["code"] == "P_MISSING_FILE")
        .unwrap_or_else(|| panic!("{v}"));
    assert_eq!(p["url"], "apps/big.html");
    assert!(!dir.join("export").exists());
}

#[test]
fn respects_skill_setup_folder() {
    if !ready() {
        return;
    }
    let d = tempfile::tempdir().unwrap();
    let dir = d.path().join("talk");
    copy_dir(&Path::new(FIX).join("skillsetup"), &dir);
    let mj = quartz_deck::assets::ensure(&cache(), &quartz_deck::assets::MATHJAX, false).unwrap();
    std::fs::copy(&mj.path, dir.join("_vendor/mathjax/tex-svg-full.js")).unwrap();
    let yml = std::fs::read(dir.join("_quarto.yml")).unwrap();
    let (code, v, err) = run(&dir, &["export"]);
    assert_eq!(code, 0, "{v}\n{err}");
    assert_eq!(v["check"]["ok"], true, "{v}");
    assert_eq!(
        std::fs::read(dir.join("_quarto.yml")).unwrap(),
        yml,
        "project config must not change"
    );
    assert!(
        !dir.join("_output").exists(),
        "must not write the project's output dir"
    );
}

#[test]
fn refuses_offline_incompatible_features() {
    if !ready() {
        return;
    }
    let (_t, dir) = deck("chalkboard.qmd");
    let (code, _, err) = run(&dir, &["export"]);
    assert_eq!(code, 1);
    assert_eq!(err["error"]["code"], "E_OFFLINE_INCOMPATIBLE");
    assert!(
        err["error"]["message"]
            .as_str()
            .unwrap()
            .contains("chalkboard")
    );
    assert!(!dir.join("export").exists());
}

#[test]
fn failed_export_keeps_previous_and_never_replaces_foreign_folders() {
    if !ready() {
        return;
    }
    let (_t, dir) = deck("math.qmd");
    assert_eq!(run(&dir, &["export"]).0, 0);
    let good = std::fs::read(dir.join("export/deck.html")).unwrap();
    let mut text = std::fs::read_to_string(dir.join("deck.qmd")).unwrap();
    text.push_str("\n## Gone {#gone background-video=\"media/not-there.mp4\"}\n\nx\n");
    std::fs::write(dir.join("deck.qmd"), text).unwrap();
    let (code, v, err) = run(&dir, &["export"]);
    assert_eq!(code, 1, "{v}");
    assert_eq!(err["error"]["code"], "E_OFFLINE");
    assert_eq!(v["ok"], false);
    assert!(problem_codes(&v).contains(&"P_MISSING_FILE".into()), "{v}");
    assert_eq!(
        std::fs::read(dir.join("export/deck.html")).unwrap(),
        good,
        "previous export must stay (FR-015)"
    );
    assert!(
        v["staging"].as_str().is_some_and(|s| Path::new(s).exists()),
        "staging kept for inspection: {v}"
    );

    // a folder qd did not create is never replaced
    let (_t2, dir2) = deck("math.qmd");
    std::fs::create_dir(dir2.join("export")).unwrap();
    std::fs::write(dir2.join("export/mine.txt"), "keep").unwrap();
    let (code, _, err) = run(&dir2, &["export"]);
    assert_eq!(code, 1);
    assert_eq!(err["error"]["code"], "E_INVALID_ARG");
    assert_eq!(
        std::fs::read_to_string(dir2.join("export/mine.txt")).unwrap(),
        "keep"
    );
    let (code, v, _) = run(&dir2, &["export", "--out", "offline"]);
    assert_eq!(code, 0, "{v}");
    assert!(dir2.join("offline/deck.html").exists());
}

#[test]
fn exports_plotly_without_web_scripts() {
    let py = Command::new("python3")
        .args(["-c", "import plotly, ipykernel"])
        .output()
        .is_ok_and(|o| o.status.success());
    if !py {
        eprintln!("skipping: python plotly/ipykernel not installed");
        return;
    }
    if !ready() {
        return;
    }
    let (_t, dir) = deck("plotly.qmd");
    let (code, v, err) = run(&dir, &["export"]);
    assert_eq!(code, 0, "{v}\n{err}");
    assert_eq!(v["check"]["ok"], true, "{v}");
    let html = std::fs::read_to_string(dir.join("export/deck.html")).unwrap();
    assert!(
        // the bundled plotly.js itself mentions cdn.plot.ly (topojsonURL default), so look for a script tag
        !html.contains("src=\"https://cdn.plot.ly"),
        "plotly.js must not be loaded from the web"
    );
    assert!(
        v["assets"]
            .as_array()
            .unwrap()
            .iter()
            .any(|a| a["name"] == "plotly.js"),
        "{v}"
    );
}

// ---------- US3: PDF (T019) ----------

fn pdf_pages(path: &Path) -> usize {
    quartz_deck::offline::count_pdf_pages(&std::fs::read(path).unwrap())
}

fn pdf_text(path: &Path) -> Option<String> {
    let out = Command::new("pdftotext").arg(path).arg("-").output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

#[test]
fn pdf_pages_follow_fragments_and_notes_are_opt_in() {
    if !ready() {
        return;
    }
    let (_t, dir) = deck("fragments.qmd");
    let (code, v, err) = run(&dir, &["export", "--pdf"]);
    assert_eq!(code, 0, "{v}\n{err}");
    let pdf = PathBuf::from(v["pdf"]["path"].as_str().unwrap());
    assert_eq!(
        pdf,
        std::fs::canonicalize(&dir).unwrap().join("export/deck.pdf")
    );
    let steps = v["check"]["steps"].as_u64().unwrap() as usize;
    assert_eq!(steps, 6);
    assert_eq!(v["pdf"]["pages"], steps, "one page per fragment step: {v}");
    assert_eq!(pdf_pages(&pdf), steps, "page count read back from the file");
    if let Some(text) = pdf_text(&pdf) {
        assert!(
            !text.contains("these are the speaker notes"),
            "notes must not appear unless requested"
        );
        assert!(
            text.contains("PRINT STILL") && !text.contains("LIVE ONLY"),
            "print-only/print-hide: {text}"
        );
    }

    let (code, v, _) = run(&dir, &["pdf", "--per-slide", "--out", "slides.pdf"]);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["pages"], 4, "one page per slide: {v}");
    assert_eq!(pdf_pages(&dir.join("slides.pdf")), 4);

    let (code, v, _) = run(&dir, &["pdf", "--notes", "--out", "notes.pdf"]);
    assert_eq!(code, 0, "{v}");
    assert!(
        v["pages"].as_u64().unwrap() as usize > steps,
        "notes on separate pages: {v}"
    );
    if let Some(text) = pdf_text(&dir.join("notes.pdf")) {
        assert!(text.contains("these are the speaker notes"));
    }
}

#[test]
fn pdf_needs_an_export_and_hides_yaml_show_notes() {
    if !ready() {
        return;
    }
    let (_t, dir) = deck("shownotes.qmd");
    let (code, _, err) = run(&dir, &["pdf"]);
    assert_eq!(code, 1);
    assert_eq!(err["error"]["code"], "E_NOT_FOUND");
    assert!(
        err["error"]["message"]
            .as_str()
            .unwrap()
            .contains("qd export"),
        "{err}"
    );
    let (code, v, _) = run(&dir, &["export", "--pdf"]);
    assert_eq!(code, 0, "{v}");
    if let Some(text) = pdf_text(Path::new(v["pdf"]["path"].as_str().unwrap())) {
        assert!(
            !text.contains("audience would see this"),
            "show-notes in YAML must not leak notes into the PDF"
        );
    }
}

// ---------- US4: API parity (T022) ----------

#[test]
fn run_op_exports_like_the_cli() {
    use quartz_deck::cli::run_op;
    use serde_json::json;
    if !ready() {
        return;
    }
    // the CLI and the API produce the same export and the same check
    let (_a, cli_dir) = deck("fragments.qmd");
    let (code, cli, _) = run(&cli_dir, &["export", "--pdf"]);
    assert_eq!(code, 0, "{cli}");
    let (_b, api_dir) = deck("fragments.qmd");
    let store = quartz_deck::store::Store::open_in(&api_dir.join("deck.qmd"), &cache()).unwrap();
    let api = run_op(&store, "export", &json!({"pdf": true}), None).unwrap();
    for k in ["ok", "files"] {
        assert_eq!(api[k], cli[k], "{k}");
    }
    assert_eq!(api["check"]["problems"], cli["check"]["problems"]);
    assert_eq!(api["check"]["steps"], cli["check"]["steps"]);
    assert_eq!(api["pdf"]["pages"], cli["pdf"]["pages"]);
    let report = run_op(&store, "check", &json!({"offline": true}), None).unwrap();
    assert_eq!(report["kind"], "offline_check");

    // no paths, no browser management through the API
    for (op, args) in [
        ("export", json!({"out": "/tmp/x"})),
        ("pdf", json!({"html": "/etc/hosts"})),
        ("check", json!({"offline": true, "html": "x.html"})),
        ("browser", json!({"remove": true})),
    ] {
        assert_eq!(
            run_op(&store, op, &args, None).unwrap_err().code(),
            "E_INVALID_ARG",
            "{op} {args}"
        );
    }
}

/// SC-004 (T027): 30 slides with math (+ one Plotly figure when Python plotly
/// is installed), export + check + PDF in under 60 s. Opt-in:
/// `cargo test --release --test export -- --ignored --nocapture`.
#[test]
#[ignore]
fn export_30_slides_under_a_minute() {
    if !ready() {
        return;
    }
    let plotly = Command::new("python3")
        .args(["-c", "import plotly, ipykernel"])
        .output()
        .is_ok_and(|o| o.status.success());
    let d = tempfile::tempdir().unwrap();
    let jupyter = if plotly { "jupyter: python3\n" } else { "" };
    let mut text = format!("---\ntitle: Timing\nformat: revealjs\n{jupyter}---\n\n");
    for i in 0..30 {
        text += &format!(
            "## Slide {i}\n\nPoint {i}: $x_{i}^2 + y_{i}^2$\n\n::: {{.fragment}}\nmore\n:::\n\n"
        );
    }
    if plotly {
        text += "## Figure\n\n```{python}\nimport plotly.graph_objects as go\ngo.Figure(go.Scatter(x=[1,2,3], y=[3,1,2]))\n```\n";
    }
    std::fs::write(d.path().join("deck.qmd"), text).unwrap();
    let t = std::time::Instant::now();
    let (code, v, err) = run(d.path(), &["export", "--pdf"]);
    let secs = t.elapsed().as_secs_f64();
    eprintln!(
        "export+check+pdf of 30 slides (plotly: {plotly}): {secs:.1} s, {} PDF pages",
        v["pdf"]["pages"]
    );
    assert_eq!(code, 0, "{v}\n{err}");
    assert!(secs < 60.0, "took {secs:.1} s");
}
