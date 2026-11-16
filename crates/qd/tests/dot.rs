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

fn text(t: &tempfile::TempDir) -> String {
    fs::read_to_string(t.path().join("talk.qmd")).unwrap()
}

/// Spec 015 US1 (T008): `move --at` and `--by` give the same file, one line
/// changed; snapping, `--no-snap`, `--dry-run`, `--expect` and undo.
#[test]
fn move_at_and_by() {
    let t = deck(DECK);
    let (ok, v) = json(t.path(), &["dot", "move", "d/0", "a", "--at", "1.1,1"]);
    assert!(ok, "{v}");
    assert_eq!(v["pos"], json!([1.0, 1.0]), "snapped to 0.25 in");
    let at = text(&t);
    assert_eq!(
        at,
        DECK.replace("pos=\"0,1!\"", "pos=\"1,1!\""),
        "one value changed"
    );
    let t2 = deck(DECK);
    let (ok, v) = json(t2.path(), &["dot", "move", "d/0", "a", "--by", "1,0"]);
    assert!(ok, "{v}");
    assert_eq!(text(&t2), at, "--by and --at agree");
    // exact
    let (ok, _) = json(
        t2.path(),
        &["dot", "move", "d/0", "a", "--at", "1.13,-0.5", "--no-snap"],
    );
    assert!(ok);
    assert!(text(&t2).contains("pos=\"1.13,-0.5!\""));
    // dry run, stale --expect, undo
    let (ok, v) = json(
        t.path(),
        &["dot", "move", "d/0", "b", "--by", "0,1", "--dry-run"],
    );
    assert!(ok && v["dry_run"] == true, "{v}");
    assert_eq!(text(&t), at);
    let (ok, v) = json(
        t.path(),
        &["--expect", "0000", "dot", "move", "d/0", "b", "--by", "0,1"],
    );
    assert!(!ok && v["error"]["code"] == "E_CONFLICT", "{v}");
    let (ok, _) = json(t.path(), &["undo"]);
    assert!(ok);
    assert_eq!(text(&t), DECK);
}

#[test]
fn move_refuses_unpinned_and_unknown() {
    let t = deck(&DECK.replace("layout=neato; ", ""));
    let (ok, v) = json(t.path(), &["dot", "move", "d/0", "a", "--at", "1,1"]);
    assert!(!ok && v["error"]["code"] == "E_INVALID_ARG", "{v}");
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("qd dot pin d/0"),
        "{v}"
    );
    let t = deck(DECK);
    let (ok, v) = json(t.path(), &["dot", "move", "d/0", "zz", "--at", "1,1"]);
    assert!(!ok && v["error"]["code"] == "E_NOT_FOUND", "{v}");
    let t = deck(&DECK.replace("  a -> b;\n", "  a -> ;\n"));
    let (ok, v) = json(t.path(), &["dot", "move", "d/0", "a", "--at", "1,1"]);
    assert!(!ok && v["error"]["code"] == "E_UNSAFE_EDIT", "{v}");
}

const UNPINNED: &str = "---\ntitle: T\nformat: revealjs\n---\n\n## Flow {#d}\n\n```{dot}\ndigraph {\n  node [shape=box];\n  a -> b;\n  b -> c;\n}\n```\n";

#[test]
fn pin_with_positions() {
    let t = deck(UNPINNED);
    let (ok, v) = json(
        t.path(),
        &[
            "dot",
            "pin",
            "d/0",
            "--positions",
            r#"{"a":[0,0],"b":[2,0],"c":[2,1.5]}"#,
        ],
    );
    assert!(ok, "{v}");
    assert_eq!(v["pinned"], 3);
    let after = text(&t);
    for want in [
        "layout=neato;",
        "splines=ortho;",
        "a [pos=\"0,0!\"];",
        "b [pos=\"2,0!\"];",
        "c [pos=\"2,1.5!\"];",
    ] {
        assert!(after.contains(want), "{want} in {after}");
    }
    let (_, v) = json(t.path(), &["dot", "show", "d/0"]);
    assert_eq!(v["pinned"], true, "{v}");
    // missing a box
    let t = deck(UNPINNED);
    let (ok, v) = json(
        t.path(),
        &["dot", "pin", "d/0", "--positions", r#"{"a":[0,0]}"#],
    );
    assert!(!ok && v["error"]["code"] == "E_NOT_FOUND", "{v}");
}

#[test]
fn pin_rendered_boxes_land_where_pinned() {
    if !common::have("quarto") {
        return;
    }
    let t = deck(UNPINNED);
    let (ok, v) = json(
        t.path(),
        &[
            "dot",
            "pin",
            "d/0",
            "--positions",
            r#"{"a":[0,0],"b":[2,0],"c":[2,1.5]}"#,
        ],
    );
    assert!(ok, "{v}");
    let out = std::process::Command::new("quarto")
        .args(["render", "talk.qmd", "--to", "revealjs"])
        .current_dir(t.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let html = fs::read_to_string(t.path().join("talk.html")).unwrap();
    // the same reading the editor's pin uses: centres in inches, offset to 0,0
    let html = html.replacen("<svg", "<div data-qd=\"d/0\"></div><svg", 1);
    let pos = quartz_deck::dot::positions_from_svg(&html, "d/0").expect("boxes in the SVG");
    assert_eq!(pos["a"], (0.0, 0.0));
    assert_eq!(pos["b"], (2.0, 0.0));
    assert_eq!(pos["c"], (2.0, 1.5));
}

#[test]
fn pin_without_positions() {
    let t = deck(UNPINNED);
    let out = common::qd(t.path())
        .env("PATH", "/usr/bin:/bin")
        .args(["dot", "pin", "d/0"])
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(v["error"]["code"], "E_NOT_FOUND", "{v}");
    assert!(
        v["error"]["message"].as_str().unwrap().contains("qd serve"),
        "{v}"
    );
}
