//! Shared focus (spec 005): what the person has in the editor — slide,
//! elements, selected text, source line — as the editor reports it. Lives in
//! the server's memory only (FR-010); refs are the ones every command takes.

use crate::deck::Deck;
use crate::error::{QdError, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Selected text is cut here (FR-005); element refs past this are dropped.
const MAX_TEXT: usize = 2000;
const MAX_ELEMENTS: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Selection {
    pub text: String,
    #[serde(rename = "where")]
    pub at: String,
    #[serde(default)]
    pub truncated: bool,
}

/// One editor tab's focus, as it posts it (data-model.md).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub tab: String,
    pub version: String,
    pub slide: Option<String>,
    #[serde(default)]
    pub elements: Vec<String>,
    pub selection: Option<Selection>,
    pub line: Option<u64>,
    #[serde(default)]
    pub editing: bool,
}

impl Snapshot {
    /// Validate and bound a snapshot from the page.
    pub fn clamp(mut self) -> Result<Snapshot> {
        if self.tab.is_empty() || self.version.is_empty() {
            return Err(QdError::InvalidArg("focus needs tab and version".into()));
        }
        if let Some(s) = &mut self.selection {
            if !matches!(s.at.as_str(), "slide" | "source") {
                return Err(QdError::InvalidArg(format!("selection where: {}", s.at)));
            }
            if let Some((i, _)) = s.text.char_indices().nth(MAX_TEXT) {
                s.text.truncate(i);
                s.truncated = true;
            }
        }
        self.elements.truncate(MAX_ELEMENTS);
        Ok(self)
    }

    /// `qd focus` for this snapshot against the current deck `d`, `age`
    /// seconds after it arrived: refs that no longer resolve are dropped.
    pub fn report(&self, d: &Deck, age: f64) -> Value {
        let slide = self.slide.as_deref().filter(|s| d.slide(s).is_ok());
        let elements: Vec<&String> = self
            .elements
            .iter()
            .filter(|r| d.element(r).is_ok())
            .collect();
        json!({"open": true, "age": (age * 10.0).round() / 10.0, "version": self.version,
            "deck_version": d.version, "slide": slide, "elements": elements,
            "selection": self.selection, "line": self.line, "editing": self.editing})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(v: Value) -> Result<Snapshot> {
        serde_json::from_value::<Snapshot>(v).unwrap().clamp()
    }

    #[test]
    fn clamps_validates_and_drops_stale_refs() {
        let d = Deck::parse("## A {#a}\n\nOne.\n\nTwo.\n".to_string());
        let long = "x".repeat(MAX_TEXT + 10);
        let s = snap(json!({"tab": "t", "version": "v0", "slide": "a",
            "elements": ["a/1", "a/7"], "selection": {"text": long, "where": "source"},
            "line": 5, "editing": true}))
        .unwrap();
        let sel = s.selection.as_ref().unwrap();
        assert_eq!((sel.text.chars().count(), sel.truncated), (MAX_TEXT, true));

        let r = s.report(&d, 1.234);
        assert_eq!(
            r["elements"],
            json!(["a/1"]),
            "a/7 no longer exists: dropped"
        );
        assert_eq!(
            (r["slide"].as_str(), r["age"].as_f64()),
            (Some("a"), Some(1.2))
        );
        assert_eq!(
            (&r["version"], &r["deck_version"]),
            (&json!("v0"), &json!(d.version))
        );
        assert_eq!(r["selection"]["where"], "source");

        // title slide / front matter stays null; a deleted slide becomes null
        let r = snap(json!({"tab": "t", "version": "v", "slide": "gone"}))
            .unwrap()
            .report(&d, 0.0);
        assert!(r["slide"].is_null() && r["elements"] == json!([]) && r["selection"].is_null());

        let many: Vec<String> = (0..80).map(|i| format!("a/{i}")).collect();
        let s = snap(json!({"tab": "t", "version": "v", "slide": null, "elements": many})).unwrap();
        assert_eq!(s.elements.len(), MAX_ELEMENTS);

        for bad in [
            json!({"tab": "", "version": "v", "slide": null}),
            json!({"tab": "t", "version": "", "slide": null}),
            json!({"tab": "t", "version": "v", "slide": null, "selection": {"text": "x", "where": "elsewhere"}}),
        ] {
            assert_eq!(snap(bad).err().unwrap().code(), "E_INVALID_ARG");
        }
    }
}
