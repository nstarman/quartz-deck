//! Spec 015: `qd dot` reads and edits the diagram in a `{dot}` cell.

mod common;

use common::json;
use serde_json::{Value, json};
use std::fs;

const DECK: &str = "---\ntitle: T\nformat: revealjs\n---\n\n## Flow {#d}\n\n```{dot}\ndigraph {\n  layout=neato; splines=ortho;\n  node [shape=box];\n  a [label=\"Data\", pos=\"0,1!\", fillcolor=white, style=\"rounded,filled\"];\n  b [label=\"Tool\", pos=\"2,1!\", URL=\"https://example.org/tool\", width=1.5];\n  a -> b;\n  a -> b [color=red];\n}\n```\n\n```{python}\nx = 1\n```\n";

fn deck(text: &str) -> tempfile::TempDir {
    let t = tempfile::tempdir().unwrap();
    fs::write(t.path().join("talk.qmd"), text).unwrap();
    t
}

#[test]
fn show_lists_the_diagram() {
    let t = deck(DECK);
    let (ok, v) = json(t.path(), &["dot", "show", "d/0"]);
    assert!(ok, "{v}");
    assert_eq!(
        (v["kind"].clone(), v["cell"].clone()),
        (json!("dot"), json!("d/0"))
    );
    assert_eq!(v["editable"], true);
    assert_eq!(v["pinned"], true);
    assert_eq!(
        v["settings"],
        json!({"layout": "neato", "splines": "ortho"})
    );
    assert_eq!(
        v["nodes"][0],
        json!({"name": "a", "label": "Data", "look": {"fillcolor": "white", "style": "rounded,filled"},
               "url": null, "image": null, "pos": [0.0, 1.0], "pinned": true, "size": {}, "locked": false})
    );
    assert_eq!(v["nodes"][1]["url"], "https://example.org/tool");
    assert_eq!(v["nodes"][1]["size"], json!({"width": "1.5"}));
    let idx: Vec<&Value> = v["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| &e["index"])
        .collect();
    assert_eq!(idx, [&json!(1), &json!(2)]);
    assert_eq!(v["edges"][1]["attrs"], json!({"color": "red"}));
}

#[test]
fn show_explains_why_not() {
    let t = deck(&DECK.replace("  a -> b;\n", "  a -> ;\n"));
    let (ok, v) = json(t.path(), &["dot", "show", "d/0"]);
    assert!(ok, "{v}");
    let why = &v["editable"];
    assert!(why["reason"].as_str().unwrap().contains(';'), "{v}");
    assert_eq!(why["at"]["line"], 14, "the deck's line: {v}");
    assert!(v.get("nodes").is_none());
}

#[test]
fn show_refuses_what_is_not_a_diagram() {
    let t = deck(DECK);
    for r in ["d/1", "d/9"] {
        let (ok, v) = json(t.path(), &["dot", "show", r]);
        assert!(!ok, "{r}: {v}");
        assert_eq!(v["error"]["code"], "E_NOT_FOUND", "{r}: {v}");
    }
}
