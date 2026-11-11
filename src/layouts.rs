//! Slide layouts (spec 009): one slide of plain Quarto markdown to start a new
//! slide from — a heading and positioned boxes (research R1), the empty ones
//! marked as placeholders (`.qd-ph ph=text|image`, R2), in a 1050×700 slide.
//! Built-in ones are in the binary; the person's live in qd's config folder
//! (`layouts/`), the deck's in `_layouts/` beside it (Quarto skips `_` folders).

use crate::deck::{BlockKind, Deck};
use crate::error::{QdError, Result};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// The slide size layouts are drawn for.
pub const BASE: (f64, f64) = (1050.0, 700.0);

const BUILT_IN: &[&str] = &[
    include_str!("layouts/title-content.md"),
    include_str!("layouts/two-columns.md"),
    include_str!("layouts/three-columns.md"),
    include_str!("layouts/image-text.md"),
    include_str!("layouts/text-image.md"),
    include_str!("layouts/full-image.md"),
    include_str!("layouts/big-number.md"),
    include_str!("layouts/quote.md"),
    include_str!("layouts/title-notes.md"),
    include_str!("layouts/title.md"),
    include_str!("layouts/section.md"),
    include_str!("layouts/blank.md"),
];

#[derive(Clone, Debug)]
pub struct Layout {
    pub name: String,
    pub description: String,
    /// `built-in`, `mine` (qd's config folder) or `deck` (`_layouts/` beside it).
    pub from: &'static str,
    /// The layout's file: front matter, then the slide.
    pub deck: Deck,
}

/// A placeholder box of a layout, in its 1050×700 slide.
#[derive(Clone, Debug, PartialEq)]
pub struct Placeholder {
    pub kind: String,
    pub purpose: String,
    pub rect: [f64; 4],
}

pub fn valid_name(name: &str) -> bool {
    (1..=40).contains(&name.len())
        && name
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

impl Layout {
    pub fn parse(text: &str, from: &'static str) -> Result<Layout> {
        let deck = Deck::parse(text.to_string());
        let field = |k: &str| {
            deck.meta
                .get(k)
                .and_then(|v| v.as_str())
                .map(str::to_string)
        };
        let bad = |why: &str| QdError::InvalidArg(format!("layout: {why}"));
        let name = field("name")
            .filter(|n| valid_name(n))
            .ok_or_else(|| bad("needs a name of a-z, 0-9 and -"))?;
        if deck.slides.len() != 1 {
            return Err(bad(&format!("'{name}' must hold exactly one slide")));
        }
        Ok(Layout {
            description: field("description").unwrap_or_default(),
            name,
            from,
            deck,
        })
    }

    /// Its placeholder boxes, in order.
    pub fn placeholders(&self) -> Vec<Placeholder> {
        let s = &self.deck.slides[0];
        s.blocks
            .iter()
            .filter(|b| b.kind == BlockKind::Box)
            .filter_map(|b| {
                let a = b.attrs.as_ref()?;
                if !a.has_class("qd-ph") {
                    return None;
                }
                let g = crate::ops::geom(a);
                let n = |l: Option<crate::ops::Length>| l.and_then(|l| l.px()).unwrap_or(0.0);
                let text = self.deck.slice(b.span);
                let purpose = text
                    .split_once("<!--")
                    .and_then(|(_, r)| r.split_once("-->"))
                    .map_or("", |(p, _)| p.trim());
                Some(Placeholder {
                    kind: a.get("ph").unwrap_or("text").to_string(),
                    purpose: purpose.to_string(),
                    rect: [n(g.left), n(g.top), n(g.width), n(g.height)],
                })
            })
            .collect()
    }

    /// For `qd layouts` / `GET /api/layouts`, with boxes scaled to a `w`×`h` slide.
    pub fn json(&self, w: u32, h: u32) -> Value {
        let ph = self.placeholders();
        json!({
            "name": self.name, "description": self.description, "from": self.from,
            "placeholders": ph.iter().map(|p| json!({"kind": p.kind, "purpose": p.purpose})).collect::<Vec<_>>(),
            "boxes": ph.iter().map(|p| {
                let r = scale(p.rect, w, h);
                json!({"kind": p.kind, "left": r[0], "top": r[1], "width": r[2], "height": r[3]})
            }).collect::<Vec<_>>(),
        })
    }
}

/// A rectangle drawn for 1050×700, for a `w`×`h` slide (whole px).
pub fn scale(r: [f64; 4], w: u32, h: u32) -> [f64; 4] {
    let (sx, sy) = (f64::from(w) / BASE.0, f64::from(h) / BASE.1);
    [r[0] * sx, r[1] * sy, r[2] * sx, r[3] * sy].map(f64::round)
}

/// The folder of the deck's own layouts.
pub fn deck_folder(deck_dir: &Path) -> PathBuf {
    deck_dir.join("_layouts")
}

/// The folder of the person's own layouts.
pub fn my_folder(config: &Path) -> PathBuf {
    config.join("layouts")
}

/// Every layout: the deck's, the person's, then the built-in ones (a name
/// already listed is hidden by the earlier one). Files that don't parse are skipped.
pub fn list(deck_dir: &Path, config: &Path) -> Vec<Layout> {
    let read = |dir: PathBuf, from: &'static str| -> Vec<Layout> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "md"))
            .collect();
        files.sort();
        files
            .iter()
            .filter_map(|p| Layout::parse(&std::fs::read_to_string(p).ok()?, from).ok())
            .collect()
    };
    let mut seen = std::collections::HashSet::new();
    read(deck_folder(deck_dir), "deck")
        .into_iter()
        .chain(read(my_folder(config), "mine"))
        .chain(
            BUILT_IN
                .iter()
                .map(|t| Layout::parse(t, "built-in").expect("built-in layout")),
        )
        .filter(|l| seen.insert(l.name.clone()))
        .collect()
}

/// `{"layouts": […]}` for `qd layouts` and `GET /api/layouts`, boxes scaled to `w`×`h`.
pub fn json_for(deck_dir: &Path, w: u32, h: u32) -> Value {
    let list = list(deck_dir, &crate::store::config_root());
    json!({"layouts": list.iter().map(|l| l.json(w, h)).collect::<Vec<_>>()})
}

pub fn find(name: &str, deck_dir: &Path, config: &Path) -> Result<Layout> {
    list(deck_dir, config)
        .into_iter()
        .find(|l| l.name == name)
        .ok_or_else(|| QdError::NotFound(format!("layout '{name}' (see qd layouts)")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(name: &str) -> Layout {
        let d = tempfile::tempdir().unwrap();
        find(name, d.path(), d.path()).unwrap()
    }

    #[test]
    fn built_ins_parse_with_their_placeholders() {
        let d = tempfile::tempdir().unwrap();
        let all = list(d.path(), d.path());
        assert_eq!(all.len(), 12);
        let kinds = |n: &str| {
            layout(n)
                .placeholders()
                .iter()
                .map(|p| p.kind.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(kinds("two-columns"), ["text", "text"]);
        assert_eq!(kinds("three-columns"), ["text", "text", "text"]);
        assert_eq!(kinds("image-text"), ["image", "text"]);
        assert_eq!(kinds("text-image"), ["text", "image"]);
        assert_eq!(kinds("full-image"), ["image"]);
        assert!(kinds("blank").is_empty() && kinds("title").is_empty());
        assert_eq!(
            layout("two-columns").placeholders()[0].purpose,
            "left column"
        );
        for l in &all {
            assert!(!l.description.is_empty(), "{}", l.name);
            assert_eq!(l.from, "built-in");
        }
    }

    #[test]
    fn names_and_one_slide() {
        assert!(valid_name("two-columns") && valid_name("a1"));
        assert!(
            !valid_name("Two")
                && !valid_name("")
                && !valid_name("a b")
                && !valid_name(&"a".repeat(41))
        );
        let e = |t: &str| Layout::parse(t, "deck").unwrap_err().code();
        assert_eq!(e("---\nname: Bad Name\n---\n\n## A\n"), "E_INVALID_ARG");
        assert_eq!(e("---\nname: two\n---\n\n## A\n\n## B\n"), "E_INVALID_ARG");
    }

    #[test]
    fn scaled_to_the_deck() {
        assert_eq!(
            scale([60.0, 170.0, 450.0, 470.0], 1050, 700),
            [60.0, 170.0, 450.0, 470.0]
        );
        assert_eq!(
            scale([60.0, 170.0, 450.0, 470.0], 1280, 720),
            [73.0, 175.0, 549.0, 483.0]
        );
        let j = layout("two-columns").json(1280, 720);
        assert_eq!(j["boxes"][1]["left"], 658.0);
        assert_eq!(j["placeholders"][1]["purpose"], "right column");
    }

    #[test]
    fn deck_then_mine_then_built_in() {
        let deck = tempfile::tempdir().unwrap();
        let config = tempfile::tempdir().unwrap();
        let write = |dir: PathBuf, desc: &str| {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                dir.join("quote.md"),
                format!("---\nname: quote\ndescription: {desc}\n---\n\n##\n"),
            )
            .unwrap();
        };
        write(my_folder(config.path()), "mine");
        assert_eq!(
            find("quote", deck.path(), config.path()).unwrap().from,
            "mine"
        );
        write(deck_folder(deck.path()), "the deck's");
        let q = find("quote", deck.path(), config.path()).unwrap();
        assert_eq!((q.from, q.description.as_str()), ("deck", "the deck's"));
        assert_eq!(
            list(deck.path(), config.path()).len(),
            12,
            "one 'quote', the deck's"
        );
        std::fs::write(
            deck_folder(deck.path()).join("broken.md"),
            "no front matter",
        )
        .unwrap();
        assert_eq!(
            list(deck.path(), config.path()).len(),
            12,
            "a broken file is skipped"
        );
        assert_eq!(
            find("nope", deck.path(), config.path()).unwrap_err().code(),
            "E_NOT_FOUND"
        );
    }
}
