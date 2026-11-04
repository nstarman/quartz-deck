//! T050 helper (ignored by default): render every corpus deck twice — as is
//! ("final") and as the editor's annotated preview copy — into
//! `$QD_DRIFT_OUT/<deck>/{final,preview}.html`, for a browser-side layout
//! comparison (see specs/001-core-editor/checklists/layout-drift.md).
//!
//!     QD_DRIFT_OUT=/tmp/drift cargo test --test drift -- --ignored --nocapture

use quartz_deck::deck::Deck;
use quartz_deck::render::{preview_copy, render};
use std::fs;
use std::path::PathBuf;

#[test]
#[ignore]
fn render_corpus_for_drift_check() {
    let out = PathBuf::from(std::env::var("QD_DRIFT_OUT").expect("set QD_DRIFT_OUT"));
    let corpus = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/corpus");
    let mut names: Vec<_> = fs::read_dir(corpus)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "qmd"))
        .collect();
    names.sort();
    let mut index = Vec::new();
    for src in names {
        let name = src.file_stem().unwrap().to_string_lossy().into_owned();
        let dir = out.join(&name);
        fs::create_dir_all(&dir).unwrap();
        let deck = dir.join("deck.qmd");
        let text = fs::read_to_string(&src).unwrap();
        fs::write(&deck, &text).unwrap();
        let d = Deck::parse(text.clone());
        let a = render(&deck, &text, "final", &dir, true);
        let b = render(&deck, &preview_copy(&d), "preview", &dir, true);
        match (a, b) {
            (Ok(a), Ok(b)) => {
                fs::rename(a, dir.join("final.html")).unwrap();
                fs::rename(b, dir.join("preview.html")).unwrap();
                index.push(format!("\"{name}\""));
                eprintln!("ok   {name}");
            }
            (a, b) => eprintln!(
                "skip {name}: final {:?} / preview {:?}",
                a.err().map(|e| e.code()),
                b.err().map(|e| e.code())
            ),
        }
    }
    fs::write(out.join("index.json"), format!("[{}]", index.join(","))).unwrap();
}
