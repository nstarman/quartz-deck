//! The person's settings (spec 007 US2): one JSON file per user
//! (`<config>/settings.json`), the same for every deck, every `qd serve` and
//! VS Code's panel. Changed by the editor's Settings window or `qd settings`.
//! Unknown keys (from a newer qd) are kept; a bad value reads as its default.

use crate::error::{QdError, Result};
use serde_json::{Map, Value, json};
use std::fs;
use std::path::{Path, PathBuf};

enum Kind {
    Choice(&'static [&'static str]),
    Bool,
    Number,
    /// A layout's name (spec 009)
    Name,
}

struct Def {
    key: &'static str,
    label: &'static str,
    description: &'static str,
    kind: Kind,
    default: fn() -> Value,
}

const fn def(
    key: &'static str,
    label: &'static str,
    description: &'static str,
    kind: Kind,
    default: fn() -> Value,
) -> Def {
    Def {
        key,
        label,
        description,
        kind,
        default,
    }
}

/// Layout memory (`ui.*`): remembered, not shown in the window.
const fn ui(key: &'static str, kind: Kind, default: fn() -> Value) -> Def {
    def(key, "", "", kind, default)
}

// Theme first: the one most people look for (spec 008 US9)
const DEFS: &[Def] = &[
    def(
        "theme",
        "Theme",
        "Follow the system's light or dark mode, or choose one. In VS Code the panel follows VS Code.",
        Kind::Choice(&["system", "light", "dark", "hc"]),
        || json!("system"),
    ),
    def(
        "source.side",
        "Source pane side",
        "Which side of the slides the source pane sits on.",
        Kind::Choice(&["left", "right"]),
        || json!("left"),
    ),
    def(
        "source.open",
        "Source pane when the editor opens",
        "Open, closed, or as it was last time.",
        Kind::Choice(&["last", "open", "closed"]),
        || json!("last"),
    ),
    def(
        "source.save",
        "Source pane saves",
        "As you pause typing (the slides follow), or only on ⌘S and when you leave the pane.",
        Kind::Choice(&["pause", "explicit"]),
        || json!("pause"),
    ),
    def(
        "confirm.slide_delete",
        "Confirm before deleting a slide",
        "Ask first (undo can always restore it).",
        Kind::Bool,
        || json!(true),
    ),
    def(
        "hints.shortcuts",
        "Show shortcuts in tooltips",
        "Buttons' tooltips include their keyboard shortcut.",
        Kind::Bool,
        || json!(true),
    ),
    def(
        "placement.snap",
        "Snap to guides",
        "While dragging a box, it snaps to the slide's edges and centre and to other elements (hold ⌘/Ctrl to place it freely).",
        Kind::Bool,
        || json!(true),
    ),
    ui("ui.source_width", Kind::Number, || Value::Null),
    ui("ui.source_was_open", Kind::Bool, || json!(false)),
    ui("ui.source_full", Kind::Bool, || json!(false)),
    ui("ui.form_panel", Kind::Bool, || json!(false)),
    ui("ui.last_layout", Kind::Name, || json!("title-content")),
];

fn find(key: &str) -> Option<&'static Def> {
    DEFS.iter().find(|d| d.key == key)
}

fn valid(d: &Def, v: &Value) -> bool {
    match d.kind {
        Kind::Choice(xs) => v.as_str().is_some_and(|s| xs.contains(&s)),
        Kind::Bool => v.is_boolean(),
        Kind::Number => v.is_number() || v.is_null(),
        Kind::Name => v.as_str().is_some_and(crate::layouts::valid_name),
    }
}

fn allowed(d: &Def) -> String {
    match d.kind {
        Kind::Choice(xs) => xs.join(", "),
        Kind::Bool => "true, false".into(),
        Kind::Number => "a number".into(),
        Kind::Name => "a layout name".into(),
    }
}

/// `<config>/settings.json`.
pub fn path() -> PathBuf {
    crate::store::config_root().join("settings.json")
}

/// The file as stored (unknown keys included), plus a notice if it was unreadable.
fn raw(path: &Path) -> (Map<String, Value>, Vec<String>) {
    match fs::read_to_string(path) {
        Err(_) => (Map::new(), vec![]), // none yet: all defaults
        Ok(text) => match serde_json::from_str::<Map<String, Value>>(&text) {
            Ok(m) => (m, vec![]),
            Err(e) => (
                Map::new(),
                vec![format!(
                    "{} could not be read ({e}); using the defaults",
                    path.display()
                )],
            ),
        },
    }
}

/// Every setting's value now (defaults where unset or invalid), and notices for the person.
pub fn load(path: &Path) -> (Map<String, Value>, Vec<String>) {
    let (raw, mut notices) = raw(path);
    let mut values = Map::new();
    for d in DEFS {
        let v = match raw.get(d.key) {
            Some(v) if valid(d, v) => v.clone(),
            Some(v) => {
                notices.push(format!(
                    "setting {} = {v} is not one of {}; using the default",
                    d.key,
                    allowed(d)
                ));
                (d.default)()
            }
            None => (d.default)(),
        };
        values.insert(d.key.into(), v);
    }
    (values, notices)
}

/// Change some settings (all checked before any is written); returns the values now.
pub fn set(path: &Path, changes: &Map<String, Value>) -> Result<Map<String, Value>> {
    for (k, v) in changes {
        let d = find(k).ok_or_else(|| QdError::InvalidSetting(format!("no setting named {k}")))?;
        if !valid(d, v) {
            return Err(QdError::InvalidSetting(format!(
                "{k} = {v}: allowed values are {}",
                allowed(d)
            )));
        }
    }
    let (mut stored, _) = raw(path);
    stored.extend(changes.clone());
    save(path, &stored)?;
    Ok(load(path).0)
}

/// Every known setting back to its default (unknown keys are kept).
pub fn reset(path: &Path) -> Result<Map<String, Value>> {
    let (mut stored, _) = raw(path);
    stored.retain(|k, _| find(k).is_none());
    save(path, &stored)?;
    Ok(load(path).0)
}

/// Atomic, and readable only by the person (FR-011).
fn save(path: &Path, m: &Map<String, Value>) -> Result<()> {
    fs::create_dir_all(path.parent().unwrap())?;
    let tmp = path.with_extension(format!("json.tmp-{}", std::process::id()));
    fs::write(&tmp, serde_json::to_string_pretty(m).unwrap() + "\n")?;
    #[cfg(unix)]
    fs::set_permissions(&tmp, std::os::unix::fs::PermissionsExt::from_mode(0o600))?;
    fs::rename(&tmp, path)?;
    Ok(())
}

/// The window's settings (not `ui.*`), for the editor and `qd settings`.
pub fn defs() -> Value {
    DEFS.iter()
        .filter(|d| !d.key.starts_with("ui."))
        .map(|d| {
            let (kind, values) = match d.kind {
                Kind::Choice(xs) => ("choice", json!(xs)),
                Kind::Bool => ("bool", Value::Null),
                Kind::Number => ("number", Value::Null),
                Kind::Name => ("name", Value::Null),
            };
            json!({"key": d.key, "label": d.label, "description": d.description, "type": kind, "values": values, "default": (d.default)()})
        })
        .collect()
}

/// `{values, defs, notices, path}`: what the editor and `qd settings` show.
pub fn report(path: &Path) -> Value {
    let (values, notices) = load(path);
    json!({"values": values, "defs": defs(), "notices": notices, "path": path})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file() -> (tempfile::TempDir, PathBuf) {
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("settings.json");
        (d, p)
    }

    #[test]
    fn defaults_without_a_file() {
        let (_d, p) = file();
        let (v, notices) = load(&p);
        assert_eq!(v["source.side"], "left");
        assert_eq!(v["theme"], "system");
        assert_eq!(v["confirm.slide_delete"], true);
        assert_eq!(v["placement.snap"], true);
        assert!(notices.is_empty());
        assert_eq!(defs().as_array().unwrap().len(), 7, "ui.* are not shown");
    }

    #[test]
    fn set_checks_every_value_and_keeps_unknown_keys() {
        let (_d, p) = file();
        fs::write(&p, r#"{"future.thing": 7, "theme": "dark"}"#).unwrap();
        let m = |v: Value| v.as_object().unwrap().clone();

        let e = set(&p, &m(json!({"source.side": "right", "theme": "purple"}))).unwrap_err();
        assert_eq!(e.code(), "E_INVALID_SETTING");
        assert!(e.to_string().contains("system, light, dark, hc"), "{e}");
        assert_eq!(
            load(&p).0["source.side"],
            "left",
            "nothing written when one value is bad"
        );
        assert_eq!(
            set(&p, &m(json!({"nope": 1}))).unwrap_err().code(),
            "E_INVALID_SETTING"
        );

        let v = set(
            &p,
            &m(json!({"source.side": "right", "ui.source_width": 420})),
        )
        .unwrap();
        assert_eq!(
            (&v["source.side"], &v["theme"], &v["ui.source_width"]),
            (&json!("right"), &json!("dark"), &json!(420))
        );
        let stored =
            || -> Value { serde_json::from_str(&fs::read_to_string(&p).unwrap()).unwrap() };
        assert_eq!(stored()["future.thing"], 7, "unknown keys survive a save");
        #[cfg(unix)]
        assert_eq!(
            std::os::unix::fs::PermissionsExt::mode(&fs::metadata(&p).unwrap().permissions())
                & 0o777,
            0o600
        );

        let v = reset(&p).unwrap();
        assert_eq!(
            (&v["source.side"], &v["theme"]),
            (&json!("left"), &json!("system"))
        );
        assert_eq!(
            stored(),
            json!({"future.thing": 7}),
            "reset keeps unknown keys"
        );
    }

    #[test]
    fn a_bad_value_or_file_reads_as_the_default_with_one_notice() {
        let (_d, p) = file();
        fs::write(&p, r#"{"theme": "purple", "source.side": "right"}"#).unwrap();
        let (v, notices) = load(&p);
        assert_eq!(
            (&v["theme"], &v["source.side"]),
            (&json!("system"), &json!("right"))
        );
        assert_eq!(notices.len(), 1, "{notices:?}");
        fs::write(&p, "{not json").unwrap();
        let (v, notices) = load(&p);
        assert_eq!(v["source.side"], "left");
        assert_eq!(notices.len(), 1, "{notices:?}");
    }
}
