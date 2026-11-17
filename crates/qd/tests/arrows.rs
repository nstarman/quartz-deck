//! Spec 016: `qd arrow` reads and edits arrows on a slide (quarto-arrows
//! shortcodes with `position="absolute"`, in slide pixels).

mod common;

use common::{deck, json, seed_arrows, text};
use serde_json::json;
use std::fs;

const DECK: &str = "---\ntitle: T\nformat: revealjs\n---\n\n## A {#a}\n\nSome text.\n\n{{< arrow from=\"100,200\" to='300, 50' position=\"absolute\" color=red >}}\n\n{{< arrow from=\"0,0\" to=\"10,10\" >}}\n\n{{< arrow from=\"0,0\" to=\"10,10\" position=\"absolute\" curvy >}}\n\n::: {.notes}\nNotes.\n:::\n";

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

// ---------- editing (US1) ----------

const SLIDE: &str = "---\ntitle: T\nformat:\n  revealjs:\n    width: 1600\n    height: 900\n---\n\n## A {#a}\n\nSome text.\n\n::: {.notes}\nNotes.\n:::\n";

const ARROW: &str = "{{< arrow from=\"200,600\" to=\"700,300\" position=\"absolute\" >}}";

#[test]
fn add_installs_and_adds() {
    let t = deck(SLIDE);
    if seed_arrows(t.path(), false).is_none() {
        return;
    }
    let lua = t.path().join("_extensions/emilhvitfeldt/arrows/arrows.lua");
    // a dry run writes nothing
    let (ok, v) = json(
        t.path(),
        &[
            "--dry-run",
            "arrow",
            "add",
            "a",
            "--from",
            "200,600",
            "--to",
            "700,300",
        ],
    );
    assert!(ok, "{v}");
    assert_eq!(text(&t), SLIDE);
    assert!(!lua.exists());
    let (ok, v) = json(
        t.path(),
        &["arrow", "add", "a", "--from", "200,600", "--to", "700,300"],
    );
    assert!(ok, "{v}");
    assert_eq!(v["ref"], "a/1");
    assert_eq!(v["installed"], json!(["emilhvitfeldt/arrows"]));
    assert_eq!(
        text(&t),
        SLIDE.replace("Some text.\n", &format!("Some text.\n\n{ARROW}\n"))
    );
    assert!(lua.exists());
    // the second installs nothing
    let (_, v) = json(
        t.path(),
        &[
            "arrow", "add", "a", "--from", "1,2", "--to", "3,4", "--color", "red", "--label",
            "the peak",
        ],
    );
    assert_eq!(v["installed"], json!([]));
    assert!(text(&t).contains("{{< arrow from=\"1,2\" to=\"3,4\" position=\"absolute\" color=\"red\" label=\"the peak\" >}}\n\n::: {.notes}"));
    assert!(json(t.path(), &["undo"]).0);
    assert!(json(t.path(), &["undo"]).0);
    assert_eq!(text(&t), SLIDE);
}

#[test]
fn add_offline_changes_nothing() {
    let t = deck(SLIDE);
    let mut c = common::qd(t.path());
    c.env("QD_NO_DOWNLOAD", "1");
    let (ok, v) = common::json_of(c, &["arrow", "add", "a", "--from", "1,2", "--to", "3,4"]);
    assert!(!ok);
    assert_eq!(v["error"]["code"], "E_DOWNLOAD", "{v}");
    assert_eq!(text(&t), SLIDE);
    assert!(!t.path().join("_extensions").exists());
}

#[test]
fn move_from_to_by() {
    let t = deck(DECK);
    let one = |args: &[&str]| {
        let (ok, v) = json(t.path(), &[&["arrow", "move", "a/1"], args].concat());
        assert!(ok, "{v}");
        text(&t)
    };
    assert_eq!(
        one(&["--from", "110,210"]),
        DECK.replace("from=\"100,200\"", "from=\"110,210\"")
    );
    assert_eq!(
        one(&["--to", "310,60"]),
        DECK.replace("from=\"100,200\"", "from=\"110,210\"")
            .replace("to='300, 50'", "to='310,60'")
    );
    // --by moves both ends, and control points with them
    let t = deck(&DECK.replace(
        "color=red",
        "color=red control1=\"150,100\" control2=\"250,100\"",
    ));
    let (ok, v) = json(t.path(), &["arrow", "move", "a/1", "--by", "5,-5"]);
    assert!(ok, "{v}");
    assert!(text(&t).contains("{{< arrow from=\"105,195\" to='305,45' position=\"absolute\" color=red control1=\"155,95\" control2=\"255,95\" >}}"), "{}", text(&t));
    // a stale --expect is refused
    common::err(
        t.path(),
        &["arrow", "move", "a/1", "--by", "1,1", "--expect", "stale"],
        "E_CONFLICT",
    );
    common::err(t.path(), &["arrow", "move", "a/1"], "E_INVALID_ARG");
}

#[test]
fn move_refuses_read_only() {
    let t = deck(DECK);
    common::err(
        t.path(),
        &["arrow", "move", "a/2", "--by", "1,1"],
        "E_UNSAFE_EDIT",
    );
    assert_eq!(text(&t), DECK);
}

#[test]
fn rm_removes() {
    let t = deck(DECK);
    let (ok, v) = json(t.path(), &["arrow", "rm", "a/1"]);
    assert!(ok, "{v}");
    assert!(!text(&t).contains("to='300, 50'"));
    common::err(t.path(), &["arrow", "rm", "a/0"], "E_NOT_FOUND");
    assert!(json(t.path(), &["undo"]).0);
    assert_eq!(text(&t), DECK);
}

/// SC-002: the rendered arrow's ends are where they were given, in slide pixels.
#[test]
fn rendered_where_given() {
    if !common::have("quarto") {
        return;
    }
    let t = deck(SLIDE);
    if seed_arrows(t.path(), false).is_none() {
        return;
    }
    assert!(
        json(
            t.path(),
            &["arrow", "add", "a", "--from", "200,600", "--to", "700,300"]
        )
        .0
    );
    let out = std::process::Command::new("quarto")
        .args(["render", "talk.qmd"])
        .current_dir(t.path())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let html = fs::read_to_string(t.path().join("talk.html")).unwrap();
    let re = regex::Regex::new(r#"position: absolute; left: (-?[\d.]+)px; top: (-?[\d.]+)px.*?</defs><path d="M ([\d.]+),([\d.]+) L ([\d.]+),([\d.]+)""#).unwrap();
    let c = re.captures(&html).expect("a placed arrow");
    let n = |i: usize| c[i].parse::<f64>().unwrap();
    let (start, end) = ((n(1) + n(3), n(2) + n(4)), (n(1) + n(5), n(2) + n(6)));
    assert!(
        (start.0 - 200.0).abs() <= 1.0 && (start.1 - 600.0).abs() <= 1.0,
        "{start:?}"
    );
    assert!(
        (end.0 - 700.0).abs() <= 1.0 && (end.1 - 300.0).abs() <= 1.0,
        "{end:?}"
    );
}
