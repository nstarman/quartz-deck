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

/// Spec 018 SC-003: on every corpus deck, each indent and outdent qd allows
/// changes only the headings of the slides it reports, and leaves the slides,
/// their ids and every other byte as they were.
#[test]
fn indent_and_outdent_touch_only_headings() {
    use quartz_deck::ops::{self, Depth};
    let mut tried = 0;
    for (name, text) in corpus() {
        let d = Deck::parse(text.clone());
        for s in &d.slides {
            for depth in [Depth::In, Depth::Out] {
                let Ok((p, changed)) = ops::relevel(&d, &s.id, depth) else {
                    continue; // refused, with its reason (tests/groups.rs)
                };
                tried += 1;
                let heads: Vec<_> = changed
                    .iter()
                    .map(|id| d.slide(id).unwrap().head.unwrap())
                    .collect();
                for (span, _) in &p.replacements {
                    assert!(
                        heads
                            .iter()
                            .any(|h| h.start <= span.start && span.end <= h.end),
                        "{name}: {} {depth:?} edits outside the changed headings",
                        s.id
                    );
                }
                let after = Deck::parse(ops::apply(&text, &p).unwrap());
                // the same slides; a `---` slide that became a head gets an id from its
                // (empty) heading, so only its position reference `@N` may change
                assert_eq!(
                    after.slides.len(),
                    d.slides.len(),
                    "{name}: {} {depth:?}",
                    s.id
                );
                for (a, b) in after.slides.iter().zip(&d.slides) {
                    assert!(
                        a.id == b.id || b.id.starts_with('@'),
                        "{name}: {} became {}",
                        b.id,
                        a.id
                    );
                }
            }
        }
    }
    assert!(
        tried > 20,
        "the corpus exercised indent and outdent {tried} times"
    );
}
