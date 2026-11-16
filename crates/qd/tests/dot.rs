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
    let mut c = common::qd(t.path());
    c.env("PATH", "/usr/bin:/bin");
    let (ok, v) = common::json_of(c, &["dot", "pin", "d/0"]);
    assert!(!ok, "{v}");
    assert_eq!(v["error"]["code"], "E_NOT_FOUND", "{v}");
    assert!(
        v["error"]["message"].as_str().unwrap().contains("qd serve"),
        "{v}"
    );
}

// ---------- spec 015 US2 (T014) ----------

/// Run an edit; it succeeds, changes only lines matching `changed`, and `qd undo` restores the deck.
fn edit_ok(t: &tempfile::TempDir, args: &[&str]) -> (Value, String) {
    let before = text(t);
    let (ok, v) = json(t.path(), args);
    assert!(ok, "{args:?}: {v}");
    let after = text(t);
    let (ok, _) = json(t.path(), &["undo"]);
    assert!(ok);
    assert_eq!(text(t), before, "{args:?}: undo restores");
    (v, after)
}

/// The lines added and removed between two texts.
fn changed(before: &str, after: &str) -> (Vec<String>, Vec<String>) {
    let b: Vec<&str> = before.lines().collect();
    let a: Vec<&str> = after.lines().collect();
    (
        a.iter()
            .filter(|l| !b.contains(l))
            .map(|l| l.to_string())
            .collect(),
        b.iter()
            .filter(|l| !a.contains(l))
            .map(|l| l.to_string())
            .collect(),
    )
}

#[test]
fn add_placed_styled_and_connected() {
    let t = deck(DECK);
    let (v, after) = edit_ok(
        &t,
        &[
            "dot",
            "add",
            "d/0",
            "--label",
            "New",
            "--at",
            "1.1,0",
            "--look-of",
            "a",
            "--from",
            "a",
            "--to",
            "b",
        ],
    );
    assert_eq!(v["node"], "n1", "{v}");
    let (added, removed) = changed(DECK, &after);
    assert!(removed.is_empty(), "{removed:?}");
    assert_eq!(
        added,
        [
            "  n1 [label=New, style=\"rounded,filled\", fillcolor=white, pos=\"1,0!\"];",
            "  a -> n1;",
            "  n1 -> b;"
        ]
    );
    // a starter look; unpinned diagrams take no --at
    let (_, after) = edit_ok(&t, &["dot", "add", "d/0", "--look", "tool"]);
    assert!(after.contains("n1 [shape=box, style=\"rounded,filled\", fillcolor=\"#3d4a5c\", color=\"#3d4a5c\", fontcolor=white, pos=\"4,1!\"];"), "{after}");
    let t = deck(UNPINNED);
    let (ok, v) = json(t.path(), &["dot", "add", "d/0", "--at", "1,1"]);
    assert!(!ok && v["error"]["code"] == "E_INVALID_ARG", "{v}");
    let (ok, v) = json(t.path(), &["dot", "add", "d/0", "--look", "fancy"]);
    assert!(!ok && v["error"]["code"] == "E_INVALID_ARG", "{v}");
}

#[test]
fn connect_and_repeat() {
    let t = deck(DECK);
    let (v, after) = edit_ok(&t, &["dot", "connect", "d/0", "b", "a"]);
    assert_eq!(v["edge"], "b->a#1");
    assert_eq!(changed(DECK, &after).0, ["  b -> a;"]);
    let (v, _) = edit_ok(&t, &["dot", "connect", "d/0", "a", "b"]);
    assert_eq!(v["edge"], "a->b#3", "the third a->b");
    let (ok, v) = json(t.path(), &["dot", "connect", "d/0", "a", "zz"]);
    assert!(!ok && v["error"]["code"] == "E_NOT_FOUND", "{v}");
}

#[test]
fn rename_changes_only_the_label() {
    let t = deck(DECK);
    let (_, after) = edit_ok(&t, &["dot", "rename", "d/0", "a", "Raw data"]);
    assert_eq!(after, DECK.replace("label=\"Data\"", "label=\"Raw data\""));
}

#[test]
fn set_attributes_and_looks() {
    let t = deck(DECK);
    let (_, after) = edit_ok(
        &t,
        &[
            "dot",
            "set",
            "d/0",
            "b",
            "URL=https://example.org/new",
            "height=0.75",
            "width=",
        ],
    );
    assert_eq!(
        after,
        DECK.replace(
            "URL=\"https://example.org/tool\", width=1.5]",
            "URL=\"https://example.org/new\", height=0.75]"
        )
    );
    // b takes a's look: a's keys set, b's other look keys removed
    let (_, after) = edit_ok(&t, &["dot", "set", "d/0", "b", "--look-of", "a"]);
    assert!(
        after.contains("b [label=\"Tool\", pos=\"2,1!\", URL=\"https://example.org/tool\", width=1.5, style=\"rounded,filled\", fillcolor=white];"),
        "{after}"
    );
    // an arrow: the second a->b
    let (_, after) = edit_ok(&t, &["dot", "set", "d/0", "a->b#2", "arrowhead=none"]);
    assert_eq!(
        after,
        DECK.replace("[color=red]", "[color=red, arrowhead=none]")
    );
    let (ok, v) = json(t.path(), &["dot", "set", "d/0", "a", "bad key=1"]);
    assert!(!ok && v["error"]["code"] == "E_INVALID_ARG", "{v}");
}

const CHAIN: &str = "---\ntitle: T\nformat: revealjs\n---\n\n## Flow {#d}\n\n```{dot}\ndigraph {\n  a -> b -> c [color=red];\n  c -> d;\n}\n```\n";

#[test]
fn rm_a_box_takes_its_arrows() {
    let t = deck(CHAIN);
    let (v, after) = edit_ok(&t, &["dot", "rm", "d/0", "c"]);
    assert_eq!(
        v["removed"],
        json!({"nodes": ["c"], "edges": ["b->c#1", "c->d#1"]})
    );
    assert!(
        after.contains("digraph {\n  a -> b [color=red];\n  d;\n}"),
        "d stays as a box: {after}"
    );
}

#[test]
fn rm_an_arrow_from_a_chain() {
    let t = deck(CHAIN);
    let (v, after) = edit_ok(&t, &["dot", "rm", "d/0", "a->b"]);
    assert_eq!(v["removed"], json!({"nodes": [], "edges": ["a->b#1"]}));
    assert!(after.contains("  a; b -> c [color=red];\n"), "{after}");
}

#[test]
fn flip_an_arrow() {
    let t = deck(CHAIN);
    let (_, after) = edit_ok(&t, &["dot", "flip", "d/0", "c->d"]);
    assert_eq!(after, CHAIN.replace("c -> d;", "d -> c;"));
    let (_, after) = edit_ok(&t, &["dot", "flip", "d/0", "b->c"]);
    assert!(
        after.contains("  a -> b [color=red];\n  c -> d;\n  c -> b [color=red];\n"),
        "{after}"
    );
    let t = deck(&CHAIN.replace("digraph", "graph").replace("->", "--"));
    let (ok, v) = json(t.path(), &["dot", "flip", "d/0", "c--d"]);
    assert!(!ok && v["error"]["code"] == "E_INVALID_ARG", "{v}");
}

#[test]
fn new_on_a_slide_and_after_a_block() {
    let t = deck(&format!("{DECK}\n## Empty {{#e}}\n"));
    let (v, after) = edit_ok(&t, &["dot", "new", "--on", "e"]);
    assert_eq!(v["cell"], "e/0", "{v}");
    assert!(
        after.ends_with(&format!(
            "## Empty {{#e}}\n\n{}\n",
            quartz_deck::dot::starter_diagram()
        )),
        "{after}"
    );
    let (v, _) = edit_ok(&t, &["dot", "new", "--after", "d/0"]);
    assert_eq!(v["cell"], "d/1", "{v}");
    let (_, v) = json(t.path(), &["dot", "show", "d/0"]);
    assert_eq!(v["pinned"], true);
}

#[test]
fn built_diagram_renders_with_quarto() {
    if !common::have("quarto") {
        return;
    }
    let t = deck("---\ntitle: T\nformat: revealjs\n---\n\n## Flow {#d}\n");
    for args in [
        &["dot", "new", "--on", "d"][..],
        &[
            "dot", "add", "d/0", "--label", "Paper", "--look", "artifact", "--from", "tool",
        ],
        &["dot", "rename", "d/0", "data", "Inputs"],
        &["dot", "set", "d/0", "tool", "URL=https://example.org"],
        &["dot", "rm", "d/0", "data->tool"],
    ] {
        let (ok, v) = json(t.path(), args);
        assert!(ok, "{args:?}: {v}");
    }
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
    for want in [
        ">Inputs<",
        ">Paper<",
        "https://example.org",
        "<title>tool-&gt;n1</title>",
    ] {
        assert!(html.contains(want), "{want}");
    }
    assert!(!html.contains("<title>data-&gt;tool</title>"));
}
