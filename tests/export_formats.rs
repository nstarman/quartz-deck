//! 011 (T005, T009): `qd export --to` on the carry-over fixture, whose every
//! feature carries a marker word. Needs Quarto and a Python with jupyter,
//! matplotlib and plotly (`QUARTO_PYTHON` or `python3`); skips otherwise.
//! Beamer also needs TeX.

mod common;
use common::{have, leftovers, qd};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

const FIX: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/export");

fn ready() -> bool {
    let py = std::env::var("QUARTO_PYTHON").unwrap_or_else(|_| "python3".into());
    let python = Command::new(py)
        .args(["-c", "import jupyter_core, matplotlib, plotly"])
        .output()
        .is_ok_and(|o| o.status.success());
    if !python {
        eprintln!("skipping: no Python with jupyter, matplotlib and plotly");
    }
    have("quarto") && python
}

fn fixture() -> (tempfile::TempDir, PathBuf) {
    let d = tempfile::tempdir().unwrap();
    for f in ["carry.qmd", "bg.png"] {
        std::fs::copy(Path::new(FIX).join(f), d.path().join(f)).unwrap();
    }
    let deck = d.path().join("carry.qmd");
    (d, deck)
}

fn export(dir: &Path, args: &[&str]) -> (bool, Value, String) {
    let o = qd(dir)
        .args(["--deck", "carry.qmd", "export"])
        .args(args)
        .output()
        .unwrap();
    let out = String::from_utf8_lossy(&o.stdout).to_string();
    let err = String::from_utf8_lossy(&o.stderr).to_string();
    let v =
        serde_json::from_str(if o.status.success() { &out } else { &err }).unwrap_or(Value::Null);
    (o.status.success(), v, err)
}

/// Every `.xml` member of an Office file under `prefix`, joined, and how many there are.
fn office_xml(p: &Path, prefix: &str) -> (String, usize) {
    let mut z = zip::ZipArchive::new(std::fs::File::open(p).unwrap()).unwrap();
    let names: Vec<String> = z
        .file_names()
        .flatten()
        .filter(|n| n.starts_with(prefix) && n.ends_with(".xml"))
        .map(|n| n.to_string())
        .collect();
    let text = names
        .iter()
        .map(|n| std::io::read_to_string(z.by_name(n).unwrap()).unwrap())
        .collect();
    (text, names.len())
}

fn features(v: &Value) -> Vec<&str> {
    v["losses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["feature"].as_str().unwrap())
        .collect()
}

#[test]
fn pptx_and_beamer() {
    if !ready() {
        return;
    }
    let (t, deck) = fixture();
    let before = std::fs::read(&deck).unwrap();

    let (ok, v, err) = export(t.path(), &["--to", "pptx"]);
    assert!(ok, "{err}");
    let out = t.path().join("export/carry.pptx");
    assert_eq!(v["kind"], "export-format");
    assert_eq!(v["format"], "pptx");
    assert_eq!(v["route"], "quarto");
    assert_eq!(
        Path::new(v["out"].as_str().unwrap())
            .canonicalize()
            .unwrap(),
        out.canonicalize().unwrap()
    );
    assert_eq!(features(&v), ["placed", "pause", "fragment", "interactive"]);
    let (slides, n) = office_xml(&out, "ppt/slides/slide");
    let (notes, _) = office_xml(&out, "ppt/notesSlides/");
    assert_eq!(n, 9, "title + 8 slides");
    assert!(notes.contains("NOTESCOLS"), "notes in the slide notes");
    assert!(slides.contains("<p:bg>"), "the background image is kept");
    assert!(!slides.contains(". . ."), "no pause dots");
    assert!(
        !slides.contains("plt.plot"),
        "code hidden, as on the slides"
    );
    assert!(slides.contains("PLACEDBOX") && slides.contains("FRAGC"));

    if quartz_deck::formats::tex_found(
        std::env::var_os("PATH"),
        std::env::var_os("HOME").map(PathBuf::from),
    ) {
        let (ok, v, err) = export(t.path(), &["--to", "beamer"]);
        assert!(ok, "{err}");
        let pdf = t.path().join("export/carry.pdf");
        let head = std::fs::read(&pdf).unwrap();
        assert!(head.starts_with(b"%PDF"));
        assert_eq!(
            features(&v),
            [
                "notes",
                "placed",
                "fragment",
                "background-image",
                "interactive"
            ]
        );
        assert_eq!(v["losses"][0]["keep"], "hidden");
    } else {
        eprintln!("beamer skipped: no TeX");
    }

    assert_eq!(
        std::fs::read(&deck).unwrap(),
        before,
        "the deck is unchanged"
    );
    assert!(leftovers(t.path()).is_empty(), "{:?}", leftovers(t.path()));
}

#[test]
fn failed_export_leaves_nothing() {
    if !ready() {
        return;
    }
    let (t, deck) = fixture();
    let text = std::fs::read_to_string(&deck)
        .unwrap()
        .replace("plt.plot([1, 3, 2])", "1 / 0");
    std::fs::write(&deck, &text).unwrap();
    let out = t.path().join("export/carry.pptx");
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    std::fs::write(&out, "OLD").unwrap();
    let (ok, v, _) = export(t.path(), &["--to", "pptx"]);
    assert!(!ok);
    assert_eq!(v["error"]["code"], "E_RENDER");
    assert_eq!(std::fs::read_to_string(&out).unwrap(), "OLD");
    assert_eq!(std::fs::read_to_string(&deck).unwrap(), text);
    assert!(leftovers(t.path()).is_empty(), "{:?}", leftovers(t.path()));
}

#[test]
fn dry_run_lists_losses_without_rendering() {
    let (t, _) = fixture();
    let o = qd(t.path())
        .args(["--deck", "carry.qmd", "--dry-run", "export", "--to", "pptx"])
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    assert_eq!(v["dry_run"], true);
    assert_eq!(v["losses"].as_array().unwrap().len(), 4);
    assert!(!t.path().join("export").exists());
}

#[test]
fn usage_errors() {
    let (t, _) = fixture();
    let (ok, v, _) = export(t.path(), &["--to", "nosuch"]);
    assert!(!ok);
    assert_eq!(v["error"]["code"], "E_INVALID_ARG");
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("qd export --list")
    );
    let (ok, v, _) = export(t.path(), &["--to", "pptx", "--pdf"]);
    assert!(!ok);
    assert_eq!(v["error"]["code"], "E_USAGE");
    let (ok, _, _) = export(t.path(), &["--to", "pptx", "--list"]);
    assert!(!ok, "--list conflicts with --to");
}

#[test]
fn list_and_no_tex() {
    let (t, _) = fixture();
    let (ok, v, err) = export(t.path(), &["--list"]);
    assert!(ok, "{err}");
    let names: Vec<&str> = v["formats"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["group"] != "other")
        .map(|f| f["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "revealjs",
            "revealjs-pdf",
            "pptx",
            "beamer",
            "docx",
            "html",
            "pdf",
            "typst",
            "gfm",
            "commonmark",
            "ipynb",
            "odt",
            "epub"
        ]
    );

    // no TeX anywhere: PATH without it and an empty HOME
    let home = tempfile::tempdir().unwrap();
    let run = |args: &[&str]| {
        qd(t.path())
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", home.path())
            .args(["--deck", "carry.qmd", "export"])
            .args(args)
            .output()
            .unwrap()
    };
    let o = run(&["--list"]);
    let v: Value = serde_json::from_slice(&o.stdout).unwrap();
    let not_ready: Vec<&str> = v["formats"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["ready"] == false)
        .map(|f| f["name"].as_str().unwrap())
        .collect();
    assert_eq!(not_ready, ["beamer", "pdf"], "Typst comes with Quarto");
    assert_eq!(
        v["formats"][3]["hint"],
        "install TeX: quarto install tinytex"
    );
    let start = std::time::Instant::now();
    let o = run(&["--to", "beamer"]);
    assert!(start.elapsed().as_secs() < 2, "SC-004");
    let e: Value = serde_json::from_slice(&o.stderr).unwrap();
    assert_eq!(e["error"]["code"], "E_NO_TEX");
    assert!(!t.path().join("export").exists(), "nothing written");
}

/// The text of a document export, for marker words: Office/EPUB XML, the
/// notebook's JSON, a PDF through `pdftotext` (if installed), or the file.
fn text_of(p: &Path) -> Option<String> {
    match p.extension()?.to_str()? {
        "docx" => Some(office_xml(p, "word/").0),
        "odt" => Some(office_xml(p, "content").0),
        "pdf" => Command::new("pdftotext")
            .arg(p)
            .arg("-")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned()),
        _ => std::fs::read_to_string(p).ok(),
    }
}

#[test]
fn documents() {
    if !ready() {
        return;
    }
    let (t, deck) = fixture();
    let before = std::fs::read(&deck).unwrap();
    for (to, file) in [
        ("docx", "carry.docx"),
        ("typst", "carry.pdf"),
        ("gfm", "carry.md"),
        ("ipynb", "carry.ipynb"),
        ("html", "carry.html"),
        ("odt", "carry.odt"),
    ] {
        let (ok, v, err) = export(t.path(), &["--to", to]);
        assert!(ok, "{to}: {err}");
        assert_eq!(
            features(&v)[..2],
            if to == "html" {
                ["placed", "pause"]
            } else {
                ["columns", "placed"]
            },
            "{to}"
        );
        let Some(text) = text_of(&t.path().join("export").join(file)) else {
            eprintln!("{to}: no pdftotext, contents not checked");
            continue;
        };
        for m in [
            "LEFTCOL",
            "PLACEDBOX",
            "FRAGA",
            "FRAGC",
            "BGSLIDE",
            "CALLOUTTXT",
        ] {
            assert!(text.contains(m), "{to} lost {m}");
        }
        assert!(text.contains("NOTESCOLS"), "{to}: notes kept");
        assert!(text.contains("Speaker notes"), "{to}: notes labelled");
        assert!(!text.contains(". . ."), "{to}: no pause dots");
        assert!(!text.contains("plt.plot"), "{to}: code hidden");
    }
    // Markdown's figure and the page's libraries share carry_files/, beside them
    let files = t.path().join("export/carry_files");
    assert!(files.join("figure-commonmark").is_dir() && files.join("libs").is_dir());
    let md = std::fs::read_to_string(t.path().join("export/carry.md")).unwrap();
    assert!(md.contains("](carry_files/figure-commonmark/"), "{md}");
    let nb: Value = serde_json::from_str(
        &std::fs::read_to_string(t.path().join("export/carry.ipynb")).unwrap(),
    )
    .unwrap();
    assert!(
        nb["cells"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["cell_type"] == "code"),
        "code cells stay code cells"
    );
    assert_eq!(
        std::fs::read(&deck).unwrap(),
        before,
        "the deck is unchanged"
    );
    assert!(leftovers(t.path()).is_empty(), "{:?}", leftovers(t.path()));
}

#[test]
fn other_writers() {
    if !ready() {
        return;
    }
    let (t, deck) = fixture();
    let before = std::fs::read(&deck).unwrap();
    for (to, file) in [("rst", "carry.rst"), ("org", "carry.org")] {
        let (ok, v, err) = export(t.path(), &["--to", to]);
        assert!(ok, "{to}: {err}");
        assert_eq!(v["route"], "pandoc", "{to}");
        assert!(
            v["notes"][0]
                .as_str()
                .unwrap()
                .contains("Quarto-only features may be simplified"),
            "{v}"
        );
        let text = std::fs::read_to_string(t.path().join("export").join(file)).unwrap();
        for title in [
            "Columns",
            "Placed",
            "Fragments",
            "Background",
            "Math",
            "Plot",
            "Callout",
        ] {
            assert!(text.contains(title), "{to} lost the slide {title}");
        }
        assert!(
            text.contains("NOTESCOLS") && !text.contains(". . ."),
            "{to}"
        );
    }
    let (ok, v, _) = export(t.path(), &["--list"]);
    assert!(ok);
    let rst = v["formats"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["name"] == "rst")
        .expect("rst listed");
    assert_eq!(rst["group"], "other");
    assert_eq!(rst["ext"], "rst");
    assert_eq!(
        std::fs::read(&deck).unwrap(),
        before,
        "the deck is unchanged"
    );
    assert!(leftovers(t.path()).is_empty(), "{:?}", leftovers(t.path()));
}
