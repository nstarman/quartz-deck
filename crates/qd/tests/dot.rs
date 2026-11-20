//! Spec 015: `qd dot` reads and edits the diagram in a `{dot}` cell.

mod common;

use common::{deck, json, text};
use serde_json::{Value, json};
use std::fs;

const DECK: &str = "---\ntitle: T\nformat: revealjs\n---\n\n## Flow {#d}\n\n```{dot}\ndigraph {\n  layout=neato; splines=ortho;\n  node [shape=box];\n  a [label=\"Data\", pos=\"0,1!\", fillcolor=white, style=\"rounded,filled\"];\n  b [label=\"Tool\", pos=\"2,1!\", URL=\"https://example.org/tool\", width=1.5];\n  a -> b;\n  a -> b [color=red];\n}\n```\n\n```{python}\nx = 1\n```\n";

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
               "url": null, "image": null, "spacer": null, "pos": [0.0, 1.0], "pinned": true, "size": {}, "locked": false})
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
    common::err(
        t.path(),
        &["--expect", "0000", "dot", "move", "d/0", "b", "--by", "0,1"],
        "E_CONFLICT",
    );
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
    common::err(
        t.path(),
        &["dot", "move", "d/0", "zz", "--at", "1,1"],
        "E_NOT_FOUND",
    );
    let t = deck(&DECK.replace("  a -> b;\n", "  a -> ;\n"));
    common::err(
        t.path(),
        &["dot", "move", "d/0", "a", "--at", "1,1"],
        "E_UNSAFE_EDIT",
    );
}

const UNPINNED: &str = "---\ntitle: T\nformat: revealjs\n---\n\n## Flow {#d}\n\n```{dot}\ndigraph {\n  node [shape=box];\n  a -> b;\n  b -> c;\n}\n```\n";

/// Quarto's `//| file:` diagrams live in another file: qd says so, and
/// refuses to edit, instead of reporting a parse error.
#[test]
fn file_cells_are_named() {
    let t = deck(&UNPINNED.replace(
        "```{dot}\ndigraph {\n  node [shape=box];\n  a -> b;\n  b -> c;\n}\n",
        "```{dot}\n//| file: flow.dot\n",
    ));
    let (_, v) = json(t.path(), &["dot", "show", "d/0"]);
    assert_eq!(
        v["editable"]["reason"], "this diagram is in flow.dot (`//| file:`), which qd doesn't edit",
        "{v}"
    );
    common::err(t.path(), &["dot", "add", "d/0"], "E_UNSAFE_EDIT");
}

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
    common::err(
        t.path(),
        &["dot", "pin", "d/0", "--positions", r#"{"a":[0,0]}"#],
        "E_NOT_FOUND",
    );
}

/// A diagram with a subgraph isn't pinned: its boxes can't be placed, and
/// neato doesn't draw clusters. The file is left as it was.
#[test]
fn pin_refuses_subgraphs() {
    let deck_text = UNPINNED.replace(
        "  a -> b;\n",
        "  subgraph cluster_x { label=Group; a; }\n  a -> b;\n",
    );
    let t = deck(&deck_text);
    common::err(
        t.path(),
        &[
            "dot",
            "pin",
            "d/0",
            "--positions",
            r#"{"a":[0,0],"b":[2,0],"c":[2,1.5]}"#,
        ],
        "E_UNSAFE_EDIT",
    );
    assert_eq!(text(&t), deck_text);
}

/// Pin keeps a diagram's own `splines`, and adds `ortho` only when there is
/// none and no arrow uses a port (Graphviz's ortho routing ignores ports).
#[test]
fn pin_keeps_splines_and_ports() {
    let all = r#"{"a":[0,0],"b":[2,0],"c":[2,1.5]}"#;
    for (from, ortho) in [
        ("  a -> b;\n", true),
        ("  a -> b:w;\n", false),
        ("  splines=curved;\n  a -> b;\n", false),
    ] {
        let t = deck(&UNPINNED.replace("  a -> b;\n", from));
        let (ok, v) = json(t.path(), &["dot", "pin", "d/0", "--positions", all]);
        assert!(ok, "{v}");
        let after = text(&t);
        assert_eq!(after.contains("splines=ortho"), ortho, "{after}");
        assert_eq!(
            after.matches("splines=").count(),
            usize::from(ortho || from.contains("splines")),
            "{after}"
        );
    }
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
    use similar::ChangeTag::*;
    let line = |c: similar::Change<&str>| c.value().trim_end().to_string();
    let diff = similar::TextDiff::from_lines(before, after);
    let (added, removed): (Vec<_>, Vec<_>) = diff
        .iter_all_changes()
        .filter(|c| c.tag() != Equal)
        .partition(|c| c.tag() == Insert);
    (
        added.into_iter().map(line).collect(),
        removed.into_iter().map(line).collect(),
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
    common::err(
        t.path(),
        &["dot", "add", "d/0", "--at", "1,1"],
        "E_INVALID_ARG",
    );
    common::err(
        t.path(),
        &["dot", "add", "d/0", "--look", "fancy"],
        "E_INVALID_ARG",
    );
    // --from / --to name boxes already there, never the new box's own name
    for side in ["--from", "--to"] {
        common::err(t.path(), &["dot", "add", "d/0", side, "n1"], "E_NOT_FOUND");
    }
    assert_eq!(text(&t), UNPINNED);
}

#[test]
fn connect_and_repeat() {
    let t = deck(DECK);
    let (v, after) = edit_ok(&t, &["dot", "connect", "d/0", "b", "a"]);
    assert_eq!(v["edge"], "b->a#1");
    assert_eq!(changed(DECK, &after).0, ["  b -> a;"]);
    let (v, _) = edit_ok(&t, &["dot", "connect", "d/0", "a", "b"]);
    assert_eq!(v["edge"], "a->b#3", "the third a->b");
    common::err(
        t.path(),
        &["dot", "connect", "d/0", "a", "zz"],
        "E_NOT_FOUND",
    );
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
    common::err(
        t.path(),
        &["dot", "set", "d/0", "a", "bad key=1"],
        "E_INVALID_ARG",
    );
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
    common::err(t.path(), &["dot", "flip", "d/0", "c--d"], "E_INVALID_ARG");
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

/// Spec 015 US3 (T019): the first `set image=` writes the spacer label,
/// installs `_extensions/dot-image` and names it in `filters:`; undo puts
/// the deck back. `rename` then changes only the text; `image=` clears it.
#[test]
fn image_installs_dot_image() {
    let t = deck(DECK);
    let (ok, v) = json(t.path(), &["dot", "set", "d/0", "b", "image=face.png"]);
    assert!(ok, "{v}");
    assert_eq!(v["installed"], json!(["dot-image"]), "{v}");
    let after = text(&t);
    assert!(after.contains("filters:\n- dot-image\n"), "{after}");
    assert!(
        after.contains("b [label=<<TABLE BORDER=\"0\" CELLPADDING=\"0\" CELLSPACING=\"4\"><TR><TD WIDTH=\"28\" HEIGHT=\"28\" FIXEDSIZE=\"TRUE\"></TD><TD>Tool</TD></TR></TABLE>>, pos=\"2,1!\", URL=\"https://example.org/tool\", width=1.5, image=\"face.png\"];"),
        "{after}"
    );
    for f in ["_extension.yml", "dot-image.lua"] {
        assert!(
            t.path().join("_extensions/dot-image").join(f).is_file(),
            "{f}"
        );
    }
    let (_, v) = json(t.path(), &["dot", "show", "d/0"]);
    assert_eq!(v["nodes"][1]["label"], "Tool");
    assert_eq!(v["nodes"][1]["image"], "face.png");
    assert!(
        v["nodes"][1]["spacer"].is_u64(),
        "the picture's cell, for the editor: {v}"
    );
    let (ok, _) = json(t.path(), &["dot", "rename", "d/0", "b", "Tools"]);
    assert!(ok);
    assert_eq!(text(&t), after.replace("<TD>Tool</TD>", "<TD>Tools</TD>"));
    let (ok, _) = json(t.path(), &["dot", "set", "d/0", "b", "image="]);
    assert!(ok);
    let now = text(&t);
    assert!(
        now.contains("b [label=Tools, pos=\"2,1!\", URL=\"https://example.org/tool\", width=1.5];"),
        "{now}"
    );
    for _ in 0..3 {
        assert!(json(t.path(), &["undo"]).0);
    }
    assert_eq!(text(&t), DECK, "undo restores the deck");
}

// ---------- spec 017: importing a drawing ----------

const HAND: &str = include_str!("fixtures/dot-import/hand.svg");
const SLIDE: &str = "---\ntitle: T\nformat: revealjs\n---\n\n## Flow {#flow}\n\nWords\n";

/// A deck with `hand.svg` beside it, and the picture it shows.
fn with_drawing(deck_text: &str) -> tempfile::TempDir {
    let t = deck(deck_text);
    fs::write(t.path().join("hand.svg"), HAND).unwrap();
    let face = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../plugins/dot-image/face.png"
    );
    fs::copy(face, t.path().join("face.png")).unwrap();
    t
}

/// Spec 017 US1 (T009): a hand-drawn SVG becomes one pinned, editable cell;
/// what can't be placed is reported with the reason.
#[test]
fn import_hand_drawn() {
    let t = with_drawing(SLIDE);
    let (ok, v) = json(t.path(), &["dot", "import", "hand.svg", "--on", "flow"]);
    assert!(ok, "{v}");
    assert_eq!(v["kind"], "hand");
    assert_eq!(v["boxes"].as_array().unwrap().len(), 4, "{v}");
    assert_eq!(
        v["arrows"],
        json!([{"from": "data", "to": "tool"}, {"from": "tool", "to": "results"}, {"from": "paperdraft", "to": "results"}])
    );
    let left: Vec<&str> = v["left_out"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["reason"].as_str().unwrap())
        .collect();
    assert_eq!(left.len(), 3, "{v}");
    assert!(
        left.iter().any(|r| r.contains("outside every box"))
            && left.iter().any(|r| r.contains("near no box"))
    );
    assert_eq!(v["notes"], json!([]), "face.png is beside the deck");
    assert_eq!(v["installed"], json!(["dot-image"]), "{v}");
    let after = text(&t);
    assert!(after.contains("filters:\n- dot-image\n"), "{after}");
    assert!(
        after.contains("//| fig-width: 8.42\n//| fig-height: 3.46\n"),
        "{after}"
    );
    let cell = v["cell"].as_str().unwrap();
    let (ok, s) = json(t.path(), &["dot", "show", cell]);
    assert!(ok && s["editable"] == true, "{s}");
    assert_eq!(s["pinned"], true);
    let nodes = s["nodes"].as_array().unwrap();
    let node = |n: &str| nodes.iter().find(|x| x["name"] == n).unwrap();
    assert_eq!(node("tool")["url"], "https://example.org/tool");
    assert_eq!(node("tool")["image"], "face.png");
    assert_eq!(node("tool")["label"], "Tool");
    assert_eq!(node("paperdraft")["look"]["shape"], "ellipse");
    assert!(
        s["edges"]
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["attrs"]["dir"] == "both"),
        "{s}"
    );
}

/// Spec 017 US1 (T009): one edit: a dry run writes nothing; one undo puts it back; a stale version is refused.
#[test]
fn import_is_one_edit() {
    let t = with_drawing(SLIDE);
    let (ok, v) = json(
        t.path(),
        &["dot", "import", "hand.svg", "--on", "flow", "--dry-run"],
    );
    assert!(
        ok && v["diff"].as_str().is_some_and(|d| d.contains("+```{dot}")),
        "{v}"
    );
    assert_eq!(text(&t), SLIDE);
    let (ok, v) = json(
        t.path(),
        &[
            "dot", "import", "hand.svg", "--on", "flow", "--expect", "0000000",
        ],
    );
    assert!(!ok && v["error"]["code"] == "E_CONFLICT", "{v}");
    assert!(json(t.path(), &["dot", "import", "hand.svg", "--on", "flow"]).0);
    assert!(json(t.path(), &["undo"]).0);
    assert_eq!(text(&t), SLIDE);
}

/// Spec 017 US1 (T009): `--replace` puts the cell where the drawing's image was.
#[test]
fn import_replaces() {
    let deck_text =
        "---\ntitle: T\nformat: revealjs\n---\n\n## Flow {#flow}\n\n![](hand.svg)\n\nAfter\n";
    let t = with_drawing(deck_text);
    let (ok, v) = json(
        t.path(),
        &["dot", "import", "hand.svg", "--replace", "flow/0"],
    );
    assert!(ok, "{v}");
    let after = text(&t);
    assert!(
        !after.contains("![](hand.svg)") && after.contains("```{dot}"),
        "{after}"
    );
    assert!(after.find("```{dot}") < after.find("After"));
}

/// Spec 017 US1 (T009): nothing written on an error.
#[test]
fn import_errors() {
    let t = with_drawing(SLIDE);
    fs::write(
        t.path().join("lines.svg"),
        r#"<svg xmlns="http://www.w3.org/2000/svg"><line x1="0" y1="0" x2="9" y2="9"/></svg>"#,
    )
    .unwrap();
    common::err(
        t.path(),
        &["dot", "import", "talk.qmd", "--on", "flow"],
        "E_INVALID_ARG",
    );
    common::err(
        t.path(),
        &["dot", "import", "lines.svg", "--on", "flow"],
        "E_INVALID_ARG",
    );
    common::err(
        t.path(),
        &["dot", "import", "missing.svg", "--on", "flow"],
        "E_IO",
    );
    let (ok, _) = json(
        t.path(),
        &[
            "dot",
            "import",
            "hand.svg",
            "--on",
            "flow",
            "--replace",
            "flow/0",
        ],
    );
    assert!(!ok, "two places");
    assert_eq!(text(&t), SLIDE);
}

/// Spec 017 FR-008 (T009): an imported diagram edits like any other.
#[test]
fn imported_diagram_edits() {
    let t = with_drawing(SLIDE);
    let (_, v) = json(t.path(), &["dot", "import", "hand.svg", "--on", "flow"]);
    let cell = v["cell"].as_str().unwrap().to_string();
    let before = text(&t);
    let (ok, m) = json(t.path(), &["dot", "move", &cell, "data", "--by", "0.25,0"]);
    assert!(ok, "{m}");
    let after = text(&t);
    let (added, removed) = changed(&before, &after);
    assert_eq!((added.len(), removed.len()), (1, 1), "{added:?}");
    // (a move lands on the 0.25 in grid)
    assert!(added[0].contains("pos=\"0.25,3.5!\""), "{added:?}");
}

/// Spec 017 SC-002 (T011): rendered by Quarto, the imported boxes sit where
/// the drawing has them, one drawing unit to one slide pixel, within 5 px;
/// the picture is drawn.
#[test]
fn import_renders_where_drawn() {
    if !common::have("quarto") {
        return;
    }
    let t = with_drawing(SLIDE);
    assert!(json(t.path(), &["dot", "import", "hand.svg", "--on", "flow"]).0);
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
    assert!(html.contains("class=\"dot-image\""), "the picture");
    let svg = &html[html.find("<svg").unwrap()..];
    let svg = &svg[..svg.find("</svg>").unwrap()];
    // one unit to one pixel: the figure's width is its viewBox's
    let num = |re: &str| -> Vec<f64> {
        let c = regex::Regex::new(re).unwrap().captures(svg).unwrap();
        c.iter()
            .skip(1)
            .map(|m| m.unwrap().as_str().parse().unwrap())
            .collect()
    };
    let w = num(r#"width="([\d.]+)(?:pt|px)?""#)[0];
    let vb = num(r"(?i)viewBox=.([\d.]+) ([\d.]+) ([\d.]+) ([\d.]+)");
    assert!((w / vb[2] - 1.0).abs() < 0.01, "scale {}", w / vb[2]);
    // each box where hand.svg has it: the editor's own reading, in inches from the lowest, leftmost
    let html = html.replacen("<svg", "<div data-qd=\"x\"></div><svg", 1);
    let pos = quartz_deck::dot::positions_from_svg(&html, "x").expect("boxes, rounded ones too");
    let (dx, dy) = (pos["data"].0, pos["data"].1);
    for (name, drawn) in [
        ("tool", (340.0, 0.0)),
        ("paperdraft", (610.0, 0.0)),
        ("results", (340.0, 244.0)),
    ] {
        let (x, y) = pos[name];
        // inches, y up, against drawing units, y down
        let off = ((x - dx) * 72.0, (dy - y) * 72.0);
        assert!(
            (off.0 - drawn.0).abs() <= 5.0 && (off.1 - drawn.1).abs() <= 5.0,
            "{name}: {off:?} against {drawn:?}"
        );
    }
}

/// Spec 017 US2 (T013): a Graphviz-rendered SVG imports with its box names and
/// arrows (SC-004), each box pinned where it was drawn: the fixture's DOT
/// placed them at (0,1) (2.5,1) (5,1) (5,-0.5) (2.5,-0.5) (0,-0.5).
#[test]
fn import_graphviz() {
    let t = deck(SLIDE);
    fs::write(
        t.path().join("drawn.svg"),
        include_str!("fixtures/dot-import/graphviz.svg"),
    )
    .unwrap();
    let (ok, v) = json(t.path(), &["dot", "import", "drawn.svg", "--on", "flow"]);
    assert!(ok, "{v}");
    assert_eq!(v["kind"], "graphviz");
    assert_eq!(v["left_out"], json!([]), "{v}");
    let (_, s) = json(t.path(), &["dot", "show", v["cell"].as_str().unwrap()]);
    let edges: Vec<(String, String)> = s["edges"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e["from"].as_str().unwrap().into(),
                e["to"].as_str().unwrap().into(),
            )
        })
        .collect();
    assert_eq!(
        edges,
        [
            ("raw-data", "clean_set"),
            ("clean_set", "model"),
            ("model", "fit"),
            ("fit", "plots"),
            ("plots", "paper")
        ]
        .map(|(a, b)| (a.to_string(), b.to_string()))
    );
    for (name, want) in [
        ("raw-data", (0.0, 1.5)),
        ("clean_set", (2.5, 1.5)),
        ("model", (5.0, 1.5)),
        ("fit", (5.0, 0.0)),
        ("plots", (2.5, 0.0)),
        ("paper", (0.0, 0.0)),
    ] {
        let n = s["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["name"] == name)
            .unwrap();
        let (x, y) = (n["pos"][0].as_f64().unwrap(), n["pos"][1].as_f64().unwrap());
        assert!(
            (x - want.0).abs() < 0.03 && (y - want.1).abs() < 0.03,
            "{name}: ({x}, {y}) against {want:?}"
        );
        assert_eq!(n["pinned"], true);
    }
}
