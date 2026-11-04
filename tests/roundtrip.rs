//! SC-002: parsing any corpus deck and re-assembling its spans is byte-identical,
//! and the segmentation (outline) is pinned by snapshots.

use quartz_deck::deck::Deck;
use std::fs;

fn corpus() -> Vec<(String, String)> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/corpus");
    let mut files: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "qmd"))
        .collect();
    files.sort();
    assert!(files.len() >= 20, "corpus needs >= 20 decks");
    files
        .into_iter()
        .map(|p| {
            (
                p.file_stem().unwrap().to_string_lossy().into_owned(),
                fs::read_to_string(&p).unwrap(),
            )
        })
        .collect()
}

#[test]
fn spans_tile_the_file_exactly() {
    for (name, text) in corpus() {
        let deck = Deck::parse(text.clone());
        let mut rebuilt = text[deck.head.start..deck.head.end].to_string();
        let mut pos = deck.head.end;
        for s in &deck.slides {
            assert_eq!(s.span.start, pos, "{name}: gap before slide {}", s.id);
            rebuilt.push_str(deck.slice(s.span));
            pos = s.span.end;
        }
        assert_eq!(pos, text.len(), "{name}: slides do not reach EOF");
        assert_eq!(rebuilt, text, "{name}: round trip differs");
    }
}

#[test]
fn outlines() {
    for (name, text) in corpus() {
        insta::assert_snapshot!(name, Deck::parse(text).outline());
    }
}
