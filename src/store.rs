//! Safe file access (constitution V): version tokens, conflict detection,
//! atomic writes, and an undo/redo journal kept in the cache dir (derived
//! data, never deck state).

use crate::deck::{Deck, version_of};
use crate::error::{QdError, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Clone)]
struct Entry {
    ts: u64,
    op: String,
    version_before: String,
    version_after: String,
    before_text: String,
    after_text: String,
}

#[derive(Clone)]
pub struct Store {
    pub path: PathBuf,
    cache: PathBuf,
}

pub fn cache_root() -> PathBuf {
    if let Some(d) = std::env::var_os("QD_CACHE_DIR") {
        return PathBuf::from(d);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    if cfg!(target_os = "macos") {
        home.join("Library/Caches/quartz-deck")
    } else {
        std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".cache"))
            .join("quartz-deck")
    }
}

impl Store {
    pub fn open(path: &Path) -> Result<Store> {
        Store::open_in(path, &cache_root())
    }

    /// Like `open`, with an explicit cache root instead of `cache_root()`.
    pub fn open_in(path: &Path, root: &Path) -> Result<Store> {
        let path = fs::canonicalize(path)
            .map_err(|e| QdError::NoDeck(format!("{}: {e}", path.display())))?;
        let key: String = Sha256::digest(path.to_string_lossy().as_bytes())
            .iter()
            .take(8)
            .map(|b| format!("{b:02x}"))
            .collect();
        Ok(Store {
            cache: root.join(key),
            path,
        })
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache
    }

    fn read_text(&self) -> Result<String> {
        let bytes = fs::read(&self.path)?;
        String::from_utf8(bytes)
            .map_err(|_| QdError::Parse(format!("{} is not valid UTF-8", self.path.display())))
    }

    pub fn read(&self) -> Result<Deck> {
        Ok(Deck::parse(self.read_text()?))
    }

    /// Replace the file with `text` iff it still has `expected_version`.
    /// ponytail: tiny window between the re-read and the rename; a lock file
    /// would close it if two writers ever race in practice.
    fn replace(&self, expected_version: &str, text: &str) -> Result<String> {
        let current = version_of(&fs::read(&self.path)?);
        if current != expected_version {
            return Err(QdError::Conflict(format!(
                "expected version {expected_version}, file is now {current}"
            )));
        }
        let name = self.path.file_name().unwrap().to_string_lossy();
        let tmp = self
            .path
            .with_file_name(format!(".{name}.qd-tmp-{}", std::process::id()));
        fs::write(&tmp, text)?;
        if let Ok(meta) = fs::metadata(&self.path) {
            fs::set_permissions(&tmp, meta.permissions())?;
        }
        fs::rename(&tmp, &self.path)?;
        Ok(version_of(text.as_bytes()))
    }

    /// Write an edit made against `before`; journals it for undo and clears redo.
    pub fn write(&self, before: &Deck, text: &str, op: &str) -> Result<String> {
        let version = self.replace(&before.version, text)?;
        let entry = Entry {
            ts: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs()),
            op: op.into(),
            version_before: before.version.clone(),
            version_after: version.clone(),
            before_text: before.text.clone(),
            after_text: text.into(),
        };
        self.push("undo", &entry)?;
        let _ = fs::remove_file(self.journal("redo"));
        Ok(version)
    }

    pub fn undo(&self) -> Result<(String, String)> {
        let e = self.pop("undo")?;
        let current = version_of(&fs::read(&self.path)?);
        if current != e.version_after {
            self.push("undo", &e)?;
            return Err(QdError::NoUndo(
                "file changed since the last qd edit".into(),
            ));
        }
        let v = self.replace(&current, &e.before_text)?;
        self.push("redo", &e)?;
        Ok((v, e.op))
    }

    pub fn redo(&self) -> Result<(String, String)> {
        let e = self.pop("redo")?;
        let current = version_of(&fs::read(&self.path)?);
        if current != e.version_before {
            self.push("redo", &e)?;
            return Err(QdError::NoUndo("file changed since the undo".into()));
        }
        let v = self.replace(&current, &e.after_text)?;
        self.push("undo", &e)?;
        Ok((v, e.op))
    }

    fn journal(&self, name: &str) -> PathBuf {
        self.cache.join(format!("{name}.jsonl"))
    }

    fn push(&self, name: &str, e: &Entry) -> Result<()> {
        fs::create_dir_all(&self.cache)?;
        let mut f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.journal(name))?;
        writeln!(f, "{}", serde_json::to_string(e).unwrap())?;
        Ok(())
    }

    fn pop(&self, name: &str) -> Result<Entry> {
        let path = self.journal(name);
        let text = fs::read_to_string(&path).unwrap_or_default();
        let mut lines: Vec<&str> = text.lines().filter(|l| !l.is_empty()).collect();
        let last = lines
            .pop()
            .ok_or_else(|| QdError::NoUndo(format!("{name} journal is empty")))?;
        let e: Entry = serde_json::from_str(last)
            .map_err(|e| QdError::NoUndo(format!("corrupt {name} journal: {e}")))?;
        let rest: String = lines.iter().map(|l| format!("{l}\n")).collect();
        fs::write(&path, rest)?;
        Ok(e)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(text: &str) -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("deck.qmd");
        fs::write(&p, text).unwrap();
        let store = Store {
            path: fs::canonicalize(&p).unwrap(),
            cache: dir.path().join("cache"),
        };
        (dir, store)
    }

    #[test]
    fn version_is_16_hex() {
        let v = version_of(b"abc");
        assert_eq!(v.len(), 16);
        assert_eq!(v, "ba7816bf8f01cfea");
    }

    #[test]
    fn write_then_undo_redo() {
        let (_d, s) = setup("## A\n");
        let d0 = s.read().unwrap();
        let v1 = s.write(&d0, "## B\n", "title").unwrap();
        assert_eq!(fs::read_to_string(&s.path).unwrap(), "## B\n");
        assert_eq!(v1, version_of(b"## B\n"));
        let (v0, op) = s.undo().unwrap();
        assert_eq!((v0, op.as_str()), (d0.version.clone(), "title"));
        assert_eq!(fs::read_to_string(&s.path).unwrap(), "## A\n");
        assert_eq!(s.undo().unwrap_err().code(), "E_NO_UNDO");
        s.redo().unwrap();
        assert_eq!(fs::read_to_string(&s.path).unwrap(), "## B\n");
        assert_eq!(s.redo().unwrap_err().code(), "E_NO_UNDO");
    }

    #[test]
    fn conflict_when_file_changed() {
        let (_d, s) = setup("## A\n");
        let d0 = s.read().unwrap();
        fs::write(&s.path, "## A\nexternal\n").unwrap();
        assert_eq!(
            s.write(&d0, "## B\n", "x").unwrap_err().code(),
            "E_CONFLICT"
        );
        assert_eq!(fs::read_to_string(&s.path).unwrap(), "## A\nexternal\n");
    }

    #[test]
    fn undo_refused_after_external_change_and_kept() {
        let (_d, s) = setup("## A\n");
        let d0 = s.read().unwrap();
        s.write(&d0, "## B\n", "x").unwrap();
        fs::write(&s.path, "## C\n").unwrap();
        assert_eq!(s.undo().unwrap_err().code(), "E_NO_UNDO");
        fs::write(&s.path, "## B\n").unwrap();
        s.undo().unwrap(); // entry was kept
        assert_eq!(fs::read_to_string(&s.path).unwrap(), "## A\n");
    }

    #[test]
    fn new_edit_clears_redo() {
        let (_d, s) = setup("## A\n");
        let d0 = s.read().unwrap();
        s.write(&d0, "## B\n", "x").unwrap();
        s.undo().unwrap();
        let d = s.read().unwrap();
        s.write(&d, "## C\n", "y").unwrap();
        assert_eq!(s.redo().unwrap_err().code(), "E_NO_UNDO");
    }

    #[test]
    fn no_temp_files_left() {
        let (d, s) = setup("## A\n");
        let d0 = s.read().unwrap();
        s.write(&d0, "## B\n", "x").unwrap();
        let names: Vec<_> = fs::read_dir(d.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert!(
            names
                .iter()
                .all(|n| !n.to_string_lossy().contains("qd-tmp"))
        );
    }
}
