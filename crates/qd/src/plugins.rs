//! Render plugins `qd` carries and installs into a deck's `_extensions/`
//! (spec 014, research R9). A deck outside this repository then needs only
//! `qd`; one installed this way still renders with plain `quarto render`.

use crate::error::Result;
use serde_json::{Value, json};
use std::fs;
use std::path::Path;

/// A Quarto extension compiled into `qd` from `plugins/<name>/_extensions/<name>/`.
pub struct Extension {
    pub name: &'static str,
    /// `(path inside the extension's folder, contents)`; `_extension.yml` among them.
    pub files: &'static [(&'static str, &'static str)],
}

/// The extensions `qd bib` installs (filled by cite-links and refs-slide).
pub const EXTENSIONS: &[Extension] = &[];

/// A deck's copy compared with qd's.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum State {
    Absent,
    Older,
    Same,
    /// The same version, with files that differ: the person edited it.
    Edited,
    Newer,
}

fn version(yml: &str) -> Vec<u64> {
    serde_yaml_ng::from_str::<serde_yaml_ng::Value>(yml)
        .ok()
        .and_then(|y| {
            y.get("version").map(|v| match v {
                serde_yaml_ng::Value::String(s) => s.clone(),
                other => serde_yaml_ng::to_string(other).unwrap_or_default(),
            })
        })
        .unwrap_or_default()
        .trim()
        .split('.')
        .map(|p| p.parse().unwrap_or(0))
        .collect()
}

impl Extension {
    fn yml(&self) -> &'static str {
        self.files
            .iter()
            .find(|(p, _)| *p == "_extension.yml")
            .map_or("", |(_, t)| t)
    }

    pub fn state(&self, deck_dir: &Path) -> State {
        let dir = deck_dir.join("_extensions").join(self.name);
        let Ok(theirs) = fs::read_to_string(dir.join("_extension.yml")) else {
            return State::Absent;
        };
        match version(&theirs).cmp(&version(self.yml())) {
            std::cmp::Ordering::Less => State::Older,
            std::cmp::Ordering::Greater => State::Newer,
            std::cmp::Ordering::Equal => {
                let same = self
                    .files
                    .iter()
                    .all(|(p, t)| fs::read(dir.join(p)).is_ok_and(|b| b == t.as_bytes()));
                if same { State::Same } else { State::Edited }
            }
        }
    }

    /// Write qd's files over an absent or older copy. ponytail: an edited
    /// copy of an older version is overwritten (qd keeps no old versions' bytes).
    fn write(&self, deck_dir: &Path) -> Result<()> {
        let dir = deck_dir.join("_extensions").join(self.name);
        for (p, t) in self.files {
            let to = dir.join(p);
            fs::create_dir_all(to.parent().expect("inside the extension's folder"))?;
            fs::write(to, t)?;
        }
        Ok(())
    }
}

/// Install `exts` into the deck's `_extensions/`: `{installed, kept}`.
pub fn install_these(exts: &[Extension], deck_dir: &Path) -> Result<Value> {
    let (mut installed, mut kept) = (vec![], vec![]);
    for e in exts {
        match e.state(deck_dir) {
            State::Absent | State::Older => {
                e.write(deck_dir)?;
                installed.push(json!(e.name));
            }
            State::Same => {}
            State::Edited => kept.push(
                json!({"name": e.name, "state": "edited", "code": "W_EDITED",
                "message": format!("_extensions/{} was edited; qd left it as it is", e.name)}),
            ),
            State::Newer => kept.push(json!({"name": e.name, "state": "newer"})),
        }
    }
    Ok(json!({"installed": installed, "kept": kept}))
}

pub fn install(deck_dir: &Path) -> Result<Value> {
    install_these(EXTENSIONS, deck_dir)
}

/// Each extension's state in the deck, for `qd bib`.
pub fn states(deck_dir: &Path) -> Value {
    EXTENSIONS
        .iter()
        .map(|e| json!({"name": e.name, "qd": version(e.yml()).iter().map(u64::to_string).collect::<Vec<_>>().join("."),
            "state": format!("{:?}", e.state(deck_dir)).to_lowercase()}))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const V1: Extension = Extension {
        name: "demo",
        files: &[
            ("_extension.yml", "title: demo\nversion: 1.0.0\n"),
            ("demo.lua", "-- one\n"),
        ],
    };
    const V2: Extension = Extension {
        name: "demo",
        files: &[
            ("_extension.yml", "title: demo\nversion: 1.2.0\n"),
            ("demo.lua", "-- two\n"),
        ],
    };

    #[test]
    fn installs_by_version_and_keeps_edits() {
        let t = tempfile::tempdir().unwrap();
        let p = t.path();
        let lua = || fs::read_to_string(p.join("_extensions/demo/demo.lua")).unwrap();
        assert_eq!(V1.state(p), State::Absent);
        assert_eq!(
            install_these(&[V1], p).unwrap()["installed"],
            json!(["demo"])
        );
        assert_eq!(V1.state(p), State::Same);
        assert_eq!(install_these(&[V1], p).unwrap()["installed"], json!([]));
        // older copy: replaced
        assert_eq!(V2.state(p), State::Older);
        install_these(&[V2], p).unwrap();
        assert_eq!(lua(), "-- two\n");
        // newer copy: left
        assert_eq!(V1.state(p), State::Newer);
        assert_eq!(
            install_these(&[V1], p).unwrap()["kept"][0]["state"],
            "newer"
        );
        assert_eq!(lua(), "-- two\n");
        // same version, edited: left, with a warning
        fs::write(p.join("_extensions/demo/demo.lua"), "-- mine\n").unwrap();
        assert_eq!(V2.state(p), State::Edited);
        let v = install_these(&[V2], p).unwrap();
        assert_eq!(v["kept"][0]["code"], "W_EDITED");
        assert_eq!(lua(), "-- mine\n");
    }
}
