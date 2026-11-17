//! Spec 016: `qd arrow` reads and edits arrows on a slide (quarto-arrows
//! shortcodes with `position="absolute"`, in slide pixels).

mod common;

use common::json;
use serde_json::json;
use std::fs;

const DECK: &str = "---\ntitle: T\nformat: revealjs\n---\n\n## A {#a}\n\nSome text.\n\n{{< arrow from=\"100,200\" to='300, 50' position=\"absolute\" color=red >}}\n\n{{< arrow from=\"0,0\" to=\"10,10\" >}}\n\n{{< arrow from=\"0,0\" to=\"10,10\" position=\"absolute\" curvy >}}\n\n::: {.notes}\nNotes.\n:::\n";

fn deck(text: &str) -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    fs::write(t.path().join("talk.qmd"), text).unwrap();
    t
}

#[test]
fn show_reads_an_arrow() {
    let t = deck(DECK);
    let (ok, v) = json(t.path(), &["arrow", "show", "a/1"]);
    assert!(ok, "{v}");
    assert_eq!(
        v,
        json!({"ok": true, "ref": "a/1", "from": [100.0, 200.0], "to": [300.0, 50.0], "control": null,
               "options": {"from": "100,200", "to": "300, 50", "position": "absolute", "color": "red"},
               "editable": true})
    );
}

#[test]
fn show_explains_why_not() {
    let t = deck(DECK);
    for (r, why, line) in [("a/2", "flow", 12), ("a/3", "name", 14)] {
        let (ok, v) = json(t.path(), &["arrow", "show", r]);
        assert!(ok, "{v}");
        assert!(
            v["editable"]["reason"].as_str().unwrap().contains(why),
            "{v}"
        );
        assert_eq!(v["editable"]["at"]["line"], line, "{v}");
    }
}

#[test]
fn show_refuses_non_arrows() {
    let t = deck(DECK);
    common::err(t.path(), &["arrow", "show", "a/0"], "E_NOT_FOUND");
}

#[test]
fn list_arrows() {
    let t = deck(DECK);
    let (ok, v) = json(t.path(), &["arrow", "list", "a"]);
    assert!(ok, "{v}");
    let refs: Vec<_> = v["arrows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| (a["ref"].clone(), a["editable"] == true))
        .collect();
    assert_eq!(
        refs,
        [
            (json!("a/1"), true),
            (json!("a/2"), false),
            (json!("a/3"), false)
        ]
    );
}
