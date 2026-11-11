//! Images beside the deck (spec 008 US5): list them, and add one under `images/`
//! with a free name. Only the image types below, checked by their first bytes;
//! never over 20 MB; never overwriting a file.

use crate::error::{QdError, Result};
use serde_json::{Value, json};
use std::fs;
use std::io::Write;
use std::path::Path;

const KINDS: [&str; 6] = ["png", "jpg", "jpeg", "gif", "svg", "webp"];
pub const MAX_BYTES: usize = 20 << 20;

/// The images under `dir` (3 folders deep; not hidden folders, exports or
/// Quarto's `_files`), newest first, as `{"images": [{path, kind, size, modified}]}`.
pub fn list(dir: &Path) -> Result<Value> {
    fn walk(dir: &Path, rel: &str, depth: usize, out: &mut Vec<Value>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let path = if rel.is_empty() {
                name.clone()
            } else {
                format!("{rel}/{name}")
            };
            let Ok(meta) = e.metadata() else { continue };
            if meta.is_dir() {
                let skip = name.starts_with('.') || name == "export" || name.ends_with("_files");
                if depth < 3 && !skip {
                    walk(&e.path(), &path, depth + 1, out);
                }
            } else if let Some(kind) = kind_of(&name) {
                let modified = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map_or(0, |d| d.as_secs());
                out.push(
                    json!({"path": path, "kind": kind, "size": meta.len(), "modified": modified}),
                );
            }
        }
    }
    let mut found = Vec::new();
    walk(dir, "", 1, &mut found);
    found.sort_by_key(|v| std::cmp::Reverse(v["modified"].as_u64()));
    Ok(json!({ "images": found }))
}

/// The image type a file name says, if it's one we take.
fn kind_of(name: &str) -> Option<&'static str> {
    let ext = name.rsplit_once('.')?.1.to_ascii_lowercase();
    KINDS.into_iter().find(|k| *k == ext)
}

/// Do the bytes start the way a file of `kind` does?
fn looks_like(kind: &str, b: &[u8]) -> bool {
    match kind {
        "png" => b.starts_with(b"\x89PNG\r\n\x1a\n"),
        "jpg" | "jpeg" => b.starts_with(&[0xFF, 0xD8, 0xFF]),
        "gif" => b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a"),
        "webp" => b.len() >= 12 && &b[..4] == b"RIFF" && &b[8..12] == b"WEBP",
        "svg" => String::from_utf8_lossy(&b[..b.len().min(4096)]).contains("<svg"),
        _ => false,
    }
}

/// The file name reduced to letters, digits, `.`, `_` and `-` (no folders, not hidden).
fn clean(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or("");
    let s: String = base
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || "._-".contains(c) {
                c
            } else {
                '-'
            }
        })
        .collect();
    s.trim_start_matches('.').to_string()
}

/// Write `bytes` as `images/<name>` beside the deck (`x.png`, `x-1.png`, … : the
/// first free one). Returns the path relative to `dir`.
pub fn add(dir: &Path, name: &str, bytes: &[u8]) -> Result<String> {
    let bad = |why: &str| QdError::InvalidArg(format!("image {name:?}: {why}"));
    let name = clean(name);
    let kind = kind_of(&name).ok_or_else(|| bad(&format!("not one of {}", KINDS.join(", "))))?;
    let (stem, ext) = name
        .rsplit_once('.')
        .filter(|(s, _)| !s.is_empty())
        .ok_or_else(|| bad("no name"))?;
    if bytes.len() > MAX_BYTES {
        return Err(bad("larger than 20 MB"));
    }
    if !looks_like(kind, bytes) {
        return Err(bad(&format!("not a {kind} file")));
    }
    let folder = dir.join("images");
    fs::create_dir_all(&folder)?;
    for n in 0.. {
        let file = if n == 0 {
            format!("{stem}.{ext}")
        } else {
            format!("{stem}-{n}.{ext}")
        };
        // create_new: a name someone else takes meanwhile is skipped, never overwritten
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(folder.join(&file))
        {
            Ok(mut f) => {
                f.write_all(bytes)?;
                return Ok(format!("images/{file}"));
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.into()),
        }
    }
    unreachable!()
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\nrest";

    #[test]
    fn names_are_cleaned() {
        assert_eq!(clean("../../etc/my chart.png"), "my-chart.png");
        assert_eq!(clean("C:\\x\\.hidden.png"), "hidden.png");
        assert_eq!(clean("é.png"), "-.png");
    }

    #[test]
    fn free_names_and_no_overwrite() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(add(d.path(), "chart.png", PNG).unwrap(), "images/chart.png");
        assert_eq!(
            add(d.path(), "chart.png", PNG).unwrap(),
            "images/chart-1.png"
        );
        assert_eq!(
            add(d.path(), "dir/chart.png", PNG).unwrap(),
            "images/chart-2.png"
        );
        assert_eq!(fs::read(d.path().join("images/chart.png")).unwrap(), PNG);
    }

    #[test]
    fn refuses_wrong_types_bytes_and_size() {
        let d = tempfile::tempdir().unwrap();
        let code = |r: Result<String>| r.unwrap_err().code();
        assert_eq!(code(add(d.path(), "x.exe", PNG)), "E_INVALID_ARG");
        assert_eq!(code(add(d.path(), "x.png", b"GIF89a")), "E_INVALID_ARG");
        assert_eq!(code(add(d.path(), ".png", PNG)), "E_INVALID_ARG");
        let big = [PNG, &vec![0; MAX_BYTES][..]].concat();
        assert_eq!(code(add(d.path(), "x.png", &big)), "E_INVALID_ARG");
        assert!(add(d.path(), "x.svg", b"<?xml version='1.0'?><svg/>").is_ok());
        assert!(add(d.path(), "x.JPG", &[0xFF, 0xD8, 0xFF, 0]).is_ok());
        assert!(!d.path().join("images/x.png").exists());
    }

    #[test]
    fn lists_newest_first_skipping_hidden_and_exports() {
        let d = tempfile::tempdir().unwrap();
        let p = d.path();
        for f in [
            "a.png",
            "deep/1/b.svg",
            "deep/1/2/too-deep.png",
            ".git/c.png",
            "export/d.png",
            "deck_files/e.png",
            "notes.txt",
        ] {
            fs::create_dir_all(p.join(f).parent().unwrap()).unwrap();
            fs::write(p.join(f), PNG).unwrap();
        }
        let old = fs::File::options()
            .write(true)
            .open(p.join("a.png"))
            .unwrap();
        old.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1000))
            .unwrap();
        let v = list(p).unwrap();
        let paths: Vec<_> = v["images"]
            .as_array()
            .unwrap()
            .iter()
            .map(|i| i["path"].as_str().unwrap())
            .collect();
        assert_eq!(paths, ["deep/1/b.svg", "a.png"]);
        assert_eq!(v["images"][1]["kind"], "png");
        assert_eq!(v["images"][1]["modified"], 1000);
    }
}
