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

/// The folder `place` names: `deck` (`_layouts/` beside the deck) or `mine`.
pub fn folder(place: &str, deck_dir: &Path, config: &Path) -> Result<PathBuf> {
    match place {
        "deck" => Ok(deck_folder(deck_dir)),
        "mine" => Ok(my_folder(config)),
        _ => Err(QdError::InvalidArg(format!(
            "'{place}': a layout lives in 'deck' or 'mine'"
        ))),
    }
}

/// Slide `slide` of `d` as a layout file named `name` (R3): an empty heading
/// with the slide's classes, each element a placeholder where it shows (flow
/// elements at `at`, by ref, in the deck's px; else stacked under the title),
/// notes as an empty notes block. None of the slide's text is kept.
pub fn from_slide(
    d: &Deck,
    slide: &str,
    name: &str,
    at: &std::collections::HashMap<String, [f64; 4]>,
) -> Result<String> {
    let s = d.slide(slide)?;
    let mut places = std::collections::HashMap::new();
    for (r, rect) in at {
        let (es, b) = d.element(r)?;
        if es.index != s.index {
            return Err(QdError::InvalidArg(format!(
                "'{r}' is not on slide '{slide}'"
            )));
        }
        places.insert(b.span.start, *rect);
    }
    let (sx, sy) = (BASE.0 / f64::from(d.width), BASE.1 / f64::from(d.height));
    let mut head = "#".repeat(usize::from(s.level.max(1)));
    let classes: Vec<String> = s
        .attrs
        .iter()
        .flat_map(|a| a.classes())
        .map(|c| format!(".{c}"))
        .collect();
    if !classes.is_empty() {
        head = format!("{head} {{{}}}", classes.join(" "));
    }
    let mut out = vec![format!(
        "---\nname: {name}\ndescription: Saved from slide {}\n---\n\n{head}",
        s.id
    )];
    let mut stacked = 0.0;
    for b in &s.blocks {
        let text = d.slice(b.span);
        let kind = match b.kind {
            BlockKind::Notes => {
                out.push("::: notes\n:::".into());
                continue;
            }
            BlockKind::Pause | BlockKind::Html => continue,
            BlockKind::Box if text.contains("![") => "image",
            BlockKind::Image => "image",
            _ => "text",
        };
        let purpose = if b.kind == BlockKind::List {
            "list"
        } else {
            kind
        };
        let g = b
            .attrs
            .as_ref()
            .filter(|_| b.kind == BlockKind::Box)
            .map(crate::ops::geom);
        let px = |l: &Option<crate::ops::Length>| l.as_ref().and_then(|l| l.px());
        // where it is in the deck's px: a box's own place, else where it shows (`at`)
        let r = g
            .and_then(|g| Some([px(&g.left)?, px(&g.top)?, px(&g.width)?, px(&g.height)?]))
            .or_else(|| places.get(&b.span.start).copied())
            .map(|r| [r[0] * sx, r[1] * sy, r[2] * sx, r[3] * sy])
            .unwrap_or_else(|| {
                stacked += 1.0;
                [60.0, 50.0 + 120.0 * stacked, 930.0, 100.0]
            })
            .map(f64::round);
        out.push(format!(
            "::: {{.absolute .qd-ph ph={kind} left={} top={} width={} height={}}}\n<!-- {purpose} -->\n:::",
            r[0], r[1], r[2], r[3]
        ));
    }
    Ok(out.join("\n\n") + "\n")
}

fn built_in(name: &str) -> bool {
    BUILT_IN
        .iter()
        .any(|t| t.contains(&format!("\nname: {name}\n")))
}

fn file(dir: &Path, name: &str) -> Result<PathBuf> {
    if !valid_name(name) {
        return Err(QdError::InvalidArg(format!(
            "'{name}': a layout name is 1–40 of a-z, 0-9 and -"
        )));
    }
    Ok(dir.join(format!("{name}.md")))
}

/// Write layout `text` as `dir/name.md`; a taken name needs `replace`.
pub fn save(dir: &Path, name: &str, text: &str, replace: bool) -> Result<PathBuf> {
    let p = file(dir, name)?;
    if p.exists() && !replace {
        return Err(QdError::Taken(format!(
            "a layout named '{name}' is already there"
        )));
    }
    std::fs::create_dir_all(dir)?;
    std::fs::write(&p, text)?;
    Ok(p)
}

/// The saved layout `name` in `dir` (built-in ones can't be changed).
fn existing(dir: &Path, name: &str) -> Result<PathBuf> {
    let p = file(dir, name)?;
    if p.exists() {
        Ok(p)
    } else if built_in(name) {
        Err(QdError::InvalidArg(format!(
            "'{name}' is a built-in layout; those can't be changed"
        )))
    } else {
        Err(QdError::NotFound(format!(
            "layout '{name}' in {}",
            dir.display()
        )))
    }
}

pub fn rm(dir: &Path, name: &str) -> Result<PathBuf> {
    let p = existing(dir, name)?;
    std::fs::remove_file(&p)?;
    Ok(p)
}

/// Rename a saved layout: its file and the name in its front matter.
pub fn rename(dir: &Path, from: &str, to: &str) -> Result<PathBuf> {
    let old = existing(dir, from)?;
    let new = file(dir, to)?;
    if new.exists() {
        return Err(QdError::Taken(format!(
            "a layout named '{to}' is already there"
        )));
    }
    let text = std::fs::read_to_string(&old)?.replacen(
        &format!("\nname: {from}\n"),
        &format!("\nname: {to}\n"),
        1,
    );
    std::fs::write(&new, text)?;
    std::fs::remove_file(old)?;
    Ok(new)
}

/// One change to the saved layouts, as `POST /api/layouts` sends it (and `qd
/// layout …`): `{"save": {slide, name, where, replace, at}}`, `{"rm": {name,
/// where}}` or `{"rename": {from, to, where}}`. Returns the file it wrote or removed.
pub fn change(d: &Deck, deck_dir: &Path, config: &Path, req: &Value) -> Result<PathBuf> {
    let bad = |m: String| QdError::InvalidArg(m);
    let (op, v) = req
        .as_object()
        .and_then(|o| o.iter().next())
        .ok_or_else(|| bad("expected {\"save\"|\"rm\"|\"rename\": {…}}".into()))?;
    let arg = |k: &str| {
        v[k].as_str()
            .ok_or_else(|| bad(format!("'{k}' is missing")))
    };
    let dir = folder(v["where"].as_str().unwrap_or("deck"), deck_dir, config)?;
    match op.as_str() {
        "save" => {
            let at: Option<std::collections::HashMap<String, [f64; 4]>> =
                serde_json::from_value(v["at"].clone())
                    .map_err(|e| bad(format!("'at': {{ref: [left, top, width, height]}}: {e}")))?;
            let at = at.unwrap_or_default();
            let name = arg("name")?;
            let text = from_slide(d, arg("slide")?, name, &at)?;
            save(&dir, name, &text, v["replace"] == true)
        }
        "rm" => rm(&dir, arg("name")?),
        "rename" => rename(&dir, arg("from")?, arg("to")?),
        _ => Err(bad(format!("unknown layout change '{op}'"))),
    }
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

    /// Spec 009 US3: a slide becomes a layout: its title an empty heading, each
    /// element a placeholder where it was (flow ones at `at`, else stacked), notes kept empty.
    #[test]
    fn a_slide_becomes_a_layout() {
        let d = Deck::parse(
            "---\nformat: revealjs\n---\n\n## Results {#r .center}\n\nIntro.\n\n- a\n\n![](p.png){#b-1 .absolute left=600 top=100 width=300 height=200}\n\n::: notes\nSay.\n:::\n".into(),
        );
        let at =
            std::collections::HashMap::from([("r/1".to_string(), [60.0, 300.0, 400.0, 150.0])]);
        let text = from_slide(&d, "r", "results", &at).unwrap();
        let l = Layout::parse(&text, "mine").unwrap();
        assert_eq!(l.name, "results");
        let ph: Vec<(String, [f64; 4])> = l
            .placeholders()
            .into_iter()
            .map(|p| (p.kind, p.rect))
            .collect();
        assert_eq!(
            ph,
            [
                ("text".into(), [60.0, 170.0, 930.0, 100.0]),
                ("text".into(), [60.0, 300.0, 400.0, 150.0]),
                ("image".into(), [600.0, 100.0, 300.0, 200.0]),
            ]
        );
        assert!(text.contains("\n## {.center}\n"), "{text}");
        assert!(text.contains("::: notes\n:::"), "{text}");
        for gone in ["Intro.", "- a", "Say.", "Results", "p.png", "#b-1"] {
            assert!(!text.contains(gone), "{gone} in {text}");
        }
    }

    /// A placed element keeps its natural height (no `height=`): its place is
    /// where it shows (`at`), not the stacked fallback.
    #[test]
    fn a_placed_box_without_a_height_uses_where_it_shows() {
        let d = Deck::parse(
            "---\nformat: revealjs\n---\n\n## L {#l}\n\n::: {#b-1 .absolute left=560 top=450 width=450}\n- a\n:::\n".into(),
        );
        let at =
            std::collections::HashMap::from([("l/0".to_string(), [560.0, 450.0, 450.0, 200.0])]);
        let l = Layout::parse(&from_slide(&d, "l", "corner", &at).unwrap(), "deck").unwrap();
        assert_eq!(l.placeholders()[0].rect, [560.0, 450.0, 450.0, 200.0]);
    }

    #[test]
    fn save_rm_rename() {
        let d = tempfile::tempdir().unwrap();
        let dir = d.path().join("_layouts");
        let text = |n: &str| format!("---\nname: {n}\n---\n\n##\n");
        let p = save(&dir, "mine-1", &text("mine-1"), false).unwrap();
        assert!(p.ends_with("_layouts/mine-1.md"));
        assert!(matches!(
            save(&dir, "mine-1", &text("mine-1"), false),
            Err(QdError::Taken(_))
        ));
        save(&dir, "mine-1", &text("mine-1"), true).unwrap();
        assert!(matches!(
            save(&dir, "Bad Name", "", false),
            Err(QdError::InvalidArg(_))
        ));
        rename(&dir, "mine-1", "mine-2").unwrap();
        let l = Layout::parse(
            &std::fs::read_to_string(dir.join("mine-2.md")).unwrap(),
            "deck",
        )
        .unwrap();
        assert_eq!(l.name, "mine-2");
        assert!(!dir.join("mine-1.md").exists());
        save(&dir, "other", &text("other"), false).unwrap();
        assert!(matches!(
            rename(&dir, "mine-2", "other"),
            Err(QdError::Taken(_))
        ));
        rm(&dir, "mine-2").unwrap();
        assert!(!dir.join("mine-2.md").exists());
        assert!(matches!(rm(&dir, "nope"), Err(QdError::NotFound(_))));
        for r in [rm(&dir, "two-columns"), rename(&dir, "two-columns", "x")] {
            assert!(matches!(r, Err(QdError::InvalidArg(m)) if m.contains("built-in")));
        }
    }
}
