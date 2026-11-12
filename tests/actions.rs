//! Spec 007: the editor's one action list (`src/ui/actions.json`) — valid,
//! complete against the inventory of what the editor had (SC-001), and the
//! only place keys are handled (FR-014).

use serde_json::Value;
use std::collections::{HashMap, HashSet};

const LIST: &str = include_str!("../src/ui/actions.json");
const INVENTORY: &str = include_str!("../specs/007-app-shell/checklists/action-inventory.md");

fn actions() -> Vec<Value> {
    serde_json::from_str(LIST).expect("actions.json is a JSON list")
}

/// The action entries (not separators).
fn entries() -> Vec<Value> {
    actions()
        .into_iter()
        .filter(|a| a.get("id").is_some())
        .collect()
}

const PLACES: [&str; 6] = ["anywhere", "slides", "slide", "element", "text", "source"];
const MODS: [&str; 4] = ["Mod", "Shift", "Alt", "Ctrl"];

/// "Mod+Shift+N" → (sorted modifiers, key); panics on anything else.
fn parse_key(k: &str) -> (Vec<&str>, &str) {
    let parts: Vec<&str> = k.split('+').collect();
    let (key, mods) = parts.split_last().unwrap();
    let mut mods: Vec<&str> = mods.to_vec();
    for m in &mods {
        assert!(MODS.contains(m), "{k}: unknown modifier {m}");
    }
    mods.sort();
    let named = [
        "Enter",
        "Escape",
        "Tab",
        "Backspace",
        "Delete",
        "ArrowUp",
        "ArrowDown",
        "ArrowLeft",
        "ArrowRight",
        "F10",
        "ContextMenu",
        ",",
        "?",
    ];
    assert!(
        key.len() == 1 && key.chars().all(|c| c.is_ascii_alphanumeric()) || named.contains(key),
        "{k}: unknown key {key}"
    );
    (mods, key)
}

/// An action's keys: `key` everywhere (`Mod` is ⌘ on macOS, Ctrl elsewhere),
/// except in VS Code's panel, where `vscode_key` replaces it when present
/// (`null` = menu-only there).
fn keys_of<'a>(a: &'a Value, platform: &str) -> Vec<&'a str> {
    let v = match a.get("vscode_key") {
        Some(v) if platform == "vscode" => v,
        _ => &a["key"],
    };
    match v {
        Value::String(s) => vec![s.as_str()],
        Value::Array(xs) => xs.iter().filter_map(Value::as_str).collect(),
        _ => vec![],
    }
}

#[test]
fn the_list_is_valid() {
    let all = entries();
    let mut ids = HashSet::new();
    for a in &all {
        let id = a["id"].as_str().unwrap();
        assert!(ids.insert(id), "duplicate id {id}");
        assert!(
            a["label"].as_str().is_some_and(|l| !l.is_empty()),
            "{id}: label"
        );
        let wh: Vec<&str> = a["where"]
            .as_array()
            .unwrap_or_else(|| panic!("{id}: where"))
            .iter()
            .map(|w| w.as_str().unwrap())
            .collect();
        for w in &wh {
            assert!(PLACES.contains(w), "{id}: unknown place {w}");
        }
        for p in ["browser", "vscode"] {
            for k in keys_of(a, p) {
                parse_key(k);
            }
        }
        if a["runs_code"] == true {
            assert!(
                !wh.contains(&"text"),
                "{id}: code must not run from a key while typing"
            );
        }
    }
    // separators and menu paths
    for a in actions() {
        if a.get("id").is_none() {
            assert_eq!(
                a["separator"], true,
                "{a}: an entry is an action or a separator"
            );
        }
        if let Some(m) = a.get("menu").and_then(Value::as_array) {
            assert!(!m.is_empty(), "{a}: empty menu path");
        }
    }
}

#[test]
fn no_two_actions_share_a_key_in_one_place() {
    for p in ["browser", "vscode"] {
        let mut seen: HashMap<(String, Vec<&str>, &str), &str> = HashMap::new();
        let all = entries();
        for a in &all {
            let id = a["id"].as_str().unwrap();
            for w in a["where"].as_array().unwrap() {
                let w = w.as_str().unwrap();
                // `anywhere` overlaps every place
                let places: Vec<&str> = if w == "anywhere" {
                    PLACES.to_vec()
                } else {
                    vec![w]
                };
                for k in keys_of(a, p) {
                    let (mods, key) = parse_key(k);
                    for pl in &places {
                        let slot = (pl.to_string(), mods.clone(), key);
                        if let Some(other) = seen.insert(slot, id) {
                            assert_eq!(other, id, "{p}: {k} in {pl} is both {other} and {id}");
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn every_inventoried_action_is_listed() {
    let ids: HashSet<String> = entries()
        .iter()
        .map(|a| a["id"].as_str().unwrap().to_string())
        .collect();
    // the Id column: backticked ids in the last cell of each table row
    let mut missing = vec![];
    for line in INVENTORY.lines().filter(|l| l.starts_with('|')) {
        let last = line.trim_end_matches('|').rsplit('|').next().unwrap_or("");
        for id in last.split('`').skip(1).step_by(2) {
            if !id.contains('*') && !ids.contains(id) {
                missing.push(id.to_string());
            }
        }
    }
    assert!(
        missing.is_empty(),
        "inventoried but not in actions.json: {missing:?}"
    );
}

/// Key checks outside `keys.js` that operate a widget from inside it, per file,
/// with why. Everything else must be an action in the list (FR-014).
const WIDGET_KEYS: &[(&str, usize, &str)] = &[
    (
        "menu.js",
        15,
        "moving inside an open menu: arrows, Enter/Space, Esc/Tab, type-ahead",
    ),
    (
        "menubar.js",
        7,
        "moving along the menu bar: ← → ↓ Enter Space Esc",
    ),
    (
        "source.js",
        3,
        "the source popup's own Esc, ⌘Enter and ⇧Enter Run on a code cell (013)",
    ),
    // form ⌘Enter submit; Esc closing a menu while focus is in the preview; the edit
    // session's commit/leave/indent keys (move with US4, T020); the splitter's arrows
    (
        "list.js",
        5,
        "moving along the slide list (↑ ↓), Enter adding a slide there, Esc cancelling a drag",
    ),
    (
        "editor.js",
        8,
        "form submit, menu close, edit-session mechanics, splitter",
    ),
];

/// The editor's modules: (file name, text).
fn ui_js() -> Vec<(String, String)> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src/ui");
    std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension() == Some("js".as_ref()))
        .map(|p| {
            let name = p.file_name().unwrap().to_str().unwrap().to_string();
            (name, std::fs::read_to_string(&p).unwrap())
        })
        .collect()
}

#[test]
fn keys_are_handled_only_through_the_list() {
    let mut found = vec![];
    for (name, text) in ui_js() {
        if name == "keys.js" {
            continue;
        }
        let n = text.matches("e.key").count();
        let allowed = WIDGET_KEYS
            .iter()
            .find(|(f, ..)| *f == name)
            .map_or(0, |(_, n, _)| *n);
        if n != allowed {
            found.push(format!("{name}: {n} key checks, {allowed} allowed"));
        }
    }
    assert!(
        found.is_empty(),
        "keys handled outside the action list: {found:?}. Add the key to \
         src/ui/actions.json and let keys.dispatch handle it, or, if it operates a \
         widget from inside, update WIDGET_KEYS with the reason"
    );
}

/// Spec 008 US5: the editor asks with its own sheets (sheet.js), never the
/// browser's `prompt`, `confirm` or `alert` boxes.
#[test]
fn no_native_boxes() {
    let mut found = vec![];
    for (name, text) in ui_js() {
        for (i, line) in text.lines().enumerate() {
            for f in ["prompt(", "confirm(", "alert("] {
                let hit = line.match_indices(f).any(|(at, _)| {
                    !line[..at].ends_with(|c: char| c.is_alphanumeric() || c == '_' || c == '.')
                });
                if hit {
                    found.push(format!("{name}:{}: {f}", i + 1));
                }
            }
        }
    }
    assert!(found.is_empty(), "native boxes: {found:?}; use sheet.js");
}
