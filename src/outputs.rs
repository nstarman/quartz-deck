//! Outputs of code cells (spec 004 R6): kept in qd's cache, never in the
//! `.qmd` (FR-009). One JSON file per deck, keyed by cell ref, with figures as
//! files beside it. An output is stale once its cell's text (fingerprint =
//! the cell's `hash`) has changed since it ran (FR-011).

use crate::deck::{BlockKind, Deck, version_of};
use crate::error::Result;
use crate::store::Store;
use serde_json::{Map, Value, json};
use std::path::{Path, PathBuf};

/// Text items longer than this are cut; the full text goes to a side file.
const MAX_LINES: usize = 2000;

pub struct Outputs {
    dir: PathBuf,
}

impl Outputs {
    pub fn new(store: &Store) -> Outputs {
        Outputs {
            dir: store.cache_dir().join("outputs"),
        }
    }

    fn load(&self) -> Map<String, Value> {
        std::fs::read(self.dir.join("outputs.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    /// Store a kernel reply (`kernel::Session::execute`) as the output of
    /// `cell`, whose text hashes to `fp`; figure files are moved in. Returns
    /// the record.
    pub fn save(&self, cell: &str, fp: &str, reply: &Value) -> Result<Value> {
        std::fs::create_dir_all(&self.dir)?;
        let mut all = self.load();
        if let Some(old) = all.get(cell) {
            for f in files(old) {
                let _ = std::fs::remove_file(self.dir.join(f));
            }
        }
        let mut truncated = false;
        let mut items = vec![];
        for (i, o) in reply["outputs"]
            .as_array()
            .into_iter()
            .flatten()
            .enumerate()
        {
            let mut o = o.clone();
            if let Some(src) = o["path"].as_str().map(PathBuf::from) {
                let ext = src.extension().unwrap_or_default().to_string_lossy();
                let name = format!("{fp}-{i}.{ext}");
                std::fs::copy(&src, self.dir.join(&name))?;
                let _ = std::fs::remove_file(&src);
                o.as_object_mut().unwrap().remove("path");
                o["file"] = json!(name);
            }
            if let Some(text) = o["text"].as_str()
                && text.lines().count() > MAX_LINES
            {
                let name = format!("{fp}-{i}.txt");
                std::fs::write(self.dir.join(&name), text)?;
                let cut: Vec<&str> = text.lines().take(MAX_LINES).collect();
                o["text"] = json!(cut.join("\n"));
                o["full"] = json!(name);
                truncated = true;
            }
            items.push(o);
        }
        let rec = json!({"fingerprint": fp, "counter": reply["counter"], "status": reply["status"],
            "items": items, "truncated": truncated});
        all.insert(cell.to_string(), rec.clone());
        let tmp = self.dir.join("outputs.json.tmp");
        std::fs::write(&tmp, Value::Object(all).to_string())?;
        std::fs::rename(&tmp, self.dir.join("outputs.json"))?;
        Ok(rec)
    }

    /// `{"cells": {ref: record + "stale"}}` for the cells of `d` with stored outputs.
    pub fn report(&self, d: &Deck) -> Value {
        let mut all = self.load();
        let cells: Map<String, Value> = d
            .blocks()
            .filter(|b| b.kind == BlockKind::Cell)
            .filter_map(|b| {
                let mut rec = all.remove(&b.id)?;
                let fp = version_of(d.slice(b.span).as_bytes());
                rec["stale"] = json!(rec["fingerprint"] != fp.as_str());
                Some((b.id.clone(), rec))
            })
            .collect();
        json!({ "cells": cells })
    }

    /// A stored file by name (as in `"file"` / `"full"`), never outside the store.
    pub fn file(&self, name: &str) -> Option<PathBuf> {
        let ok = !name.is_empty() && Path::new(name).file_name() == Some(name.as_ref());
        Some(self.dir.join(name)).filter(|p| ok && p.is_file())
    }
}

fn files(rec: &Value) -> impl Iterator<Item = &str> {
    rec["items"].as_array().into_iter().flatten().flat_map(|o| {
        [&o["file"], &o["full"]]
            .into_iter()
            .filter_map(Value::as_str)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_figures_truncates_and_marks_stale() {
        let d = tempfile::tempdir().unwrap();
        let deck = d.path().join("deck.qmd");
        let cell = "```{python}\nprint(1)\n```";
        std::fs::write(&deck, format!("## A {{#a}}\n\n{cell}\n")).unwrap();
        let store = Store::open_in(&deck, &d.path().join("cache")).unwrap();
        let out = Outputs::new(&store);
        let deck_now = || Deck::parse(std::fs::read_to_string(&deck).unwrap());
        let id = deck_now()
            .blocks()
            .find(|b| b.kind == BlockKind::Cell)
            .unwrap()
            .id
            .clone();
        let fp = version_of(cell.as_bytes());

        let fig = d.path().join("fig.png");
        std::fs::write(&fig, b"png").unwrap();
        let long = "line\n".repeat(MAX_LINES + 5);
        let reply = json!({"status": "ok", "counter": 3, "outputs": [
            {"kind": "text", "text": long},
            {"kind": "image", "mime": "image/png", "path": fig},
        ]});
        let rec = out.save(&id, &fp, &reply).unwrap();
        assert!(!fig.exists(), "figure moved into the store");
        let img = rec["items"][1]["file"].as_str().unwrap();
        assert_eq!(std::fs::read(out.file(img).unwrap()).unwrap(), b"png");
        assert_eq!(
            rec["items"][0]["text"].as_str().unwrap().lines().count(),
            MAX_LINES
        );
        let full = out.file(rec["items"][0]["full"].as_str().unwrap()).unwrap();
        assert_eq!(std::fs::read_to_string(full).unwrap(), long);
        assert_eq!(rec["truncated"], true);
        assert!(out.file("../outputs.json").is_none() && out.file("").is_none());

        let r = out.report(&deck_now());
        assert_eq!(
            (
                r["cells"][&id]["stale"].as_bool(),
                r["cells"][&id]["counter"].as_i64()
            ),
            (Some(false), Some(3))
        );
        std::fs::write(&deck, "## A {#a}\n\n```{python}\nprint(2)\n```\n").unwrap();
        assert_eq!(out.report(&deck_now())["cells"][&id]["stale"], true);

        // a new run replaces the old record and its files
        out.save(
            &id,
            &fp,
            &json!({"status": "ok", "counter": 4, "outputs": []}),
        )
        .unwrap();
        assert!(out.file(img).is_none());
    }
}
