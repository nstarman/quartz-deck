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
    /// Who made the edit (spec 012 commit messages): `person` through the
    /// editor, `agent` from a CLI process; entries written before 012 have none.
    #[serde(default = "agent")]
    by: String,
    /// What the edit changed (`one`, `one/2`), for commit messages.
    #[serde(default)]
    target: String,
}

fn agent() -> String {
    "agent".into()
}

thread_local! {
    static PERSON: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Run `f` with its edits journaled as the person's (the editor's `/api/op`).
pub fn as_person<T>(f: impl FnOnce() -> T) -> T {
    PERSON.with(|p| p.set(true));
    let r = f();
    PERSON.with(|p| p.set(false));
    r
}

/// An edit in the undo journal, as history's commit messages read it.
pub struct Edit {
    pub ts: u64,
    pub op: String,
    pub by: String,
    pub target: String,
}

#[derive(Clone)]
pub struct Store {
    pub path: PathBuf,
    cache: PathBuf,
}

/// Has `path` been replaced by another file since `old` was opened?
#[cfg(unix)]
fn replaced(old: &fs::File, path: &Path) -> bool {
    use std::os::unix::fs::MetadataExt;
    match (old.metadata(), fs::metadata(path)) {
        (Ok(a), Ok(b)) => a.ino() != b.ino() || a.dev() != b.dev(),
        _ => true,
    }
}

#[cfg(not(unix))]
fn replaced(_: &fs::File, _: &Path) -> bool {
    false
}

/// A per-user folder: `$env`, else macOS `~/Library/<mac>/quartz-deck`, else
/// `$xdg_env/quartz-deck` or `~/<dot>/quartz-deck`.
fn user_dir(env: &str, mac: &str, xdg_env: &str, dot: &str) -> PathBuf {
    if let Some(d) = std::env::var_os(env) {
        return PathBuf::from(d);
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    if cfg!(target_os = "macos") {
        home.join("Library").join(mac).join("quartz-deck")
    } else {
        std::env::var_os(xdg_env)
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(dot))
            .join("quartz-deck")
    }
}

/// The person's settings folder (spec 007).
pub fn config_root() -> PathBuf {
    user_dir(
        "QD_CONFIG_DIR",
        "Application Support",
        "XDG_CONFIG_HOME",
        ".config",
    )
}

pub fn cache_root() -> PathBuf {
    user_dir("QD_CACHE_DIR", "Caches", "XDG_CACHE_HOME", ".cache")
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

    /// The VS Code buffer overlay (spec 006): while a VS Code panel is
    /// attached, this copy of the buffer *is* the deck for every qd read and
    /// write; the `.qmd` changes only when VS Code saves.
    pub fn overlay(&self) -> PathBuf {
        self.cache.join("buffer.qmd")
    }

    pub fn attached(&self) -> bool {
        self.overlay().exists()
    }

    /// The text qd reads and writes: the overlay when attached, else the deck.
    fn backing(&self) -> PathBuf {
        let o = self.overlay();
        if o.exists() { o } else { self.path.clone() }
    }

    /// Attach a VS Code buffer: from now on qd works on `text`.
    pub fn attach(&self, text: &str) -> Result<String> {
        fs::create_dir_all(&self.cache)?;
        fs::write(self.overlay(), text)?;
        Ok(version_of(text.as_bytes()))
    }

    pub fn detach(&self) {
        let _ = fs::remove_file(self.overlay());
    }

    /// The person typed in VS Code: replace the overlay iff it is still at
    /// `base` (no qd edit landed meanwhile); not a qd edit, so not journaled.
    pub fn set_buffer(&self, base: &str, text: &str) -> Result<String> {
        let _lock = self.lock()?;
        if !self.attached() {
            return Err(QdError::Conflict("no VS Code buffer attached".into()));
        }
        self.replace(base, text)
    }

    /// qd's cache root this store lives under (shared libraries, browser).
    pub fn root(&self) -> PathBuf {
        self.cache
            .parent()
            .map_or_else(cache_root, Path::to_path_buf)
    }

    fn read_text(&self) -> Result<String> {
        let path = self.backing();
        let bytes = fs::read(&path)?;
        String::from_utf8(bytes)
            .map_err(|_| QdError::Parse(format!("{} is not valid UTF-8", path.display())))
    }

    pub fn read(&self) -> Result<Deck> {
        Ok(Deck::parse(self.read_text()?))
    }

    /// Exclusive lock shared by every qd process editing this deck (CLI, GUI
    /// server), held across check + write + journal so qd writers serialize.
    fn lock(&self) -> Result<fs::File> {
        fs::create_dir_all(&self.cache)?;
        let f = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.cache.join("lock"))?;
        f.lock()?;
        Ok(f)
    }

    /// Replace the file with `text` iff it still has `expected_version`.
    ///
    /// Against writers that ignore qd (editors, scripts) the check-then-rename
    /// has a tiny window, so the old file stays open throughout: if it was
    /// replaced (inode changed) before our rename, or written to after our
    /// check, their version is put back, our edit is withdrawn and the caller
    /// gets `E_CONFLICT`. ponytail: best effort against non-qd writers. One that
    /// opens the file before our rename and writes after our last check (or
    /// writes to the new file during the restore) can still lose data; on a
    /// loaded Linux runner an appender every 25 ms lost ~10% (see
    /// tests/concurrency.rs). Only cooperation (the qd lock) is airtight.
    fn replace(&self, expected_version: &str, text: &str) -> Result<String> {
        use std::io::{Read, Seek};
        let read_all = |f: &mut fs::File| -> Result<Vec<u8>> {
            let mut buf = Vec::new();
            f.rewind()?;
            f.read_to_end(&mut buf)?;
            Ok(buf)
        };
        let conflict = |v: &str| {
            QdError::Conflict(format!(
                "expected version {expected_version}, file is now {v}"
            ))
        };
        let path = self.backing();
        let mut old = fs::File::open(&path)?;
        let name = path.file_name().unwrap().to_string_lossy();
        let tmp = path.with_file_name(format!(".{name}.qd-tmp-{}", std::process::id()));
        fs::write(&tmp, text)?;
        if let Ok(meta) = old.metadata() {
            fs::set_permissions(&tmp, meta.permissions())?;
        }
        let current = version_of(&read_all(&mut old)?);
        if current != expected_version || replaced(&old, &path) {
            let _ = fs::remove_file(&tmp);
            return Err(conflict(&current));
        }
        fs::rename(&tmp, &path)?;
        let late = read_all(&mut old)?;
        if version_of(&late) != expected_version {
            // someone wrote to the file between our check and our rename
            fs::write(&tmp, &late)?;
            fs::rename(&tmp, &path)?;
            return Err(conflict(&version_of(&late)));
        }
        Ok(version_of(text.as_bytes()))
    }

    /// Write an edit made against `before`; journals it for undo and clears redo.
    pub fn write(&self, before: &Deck, text: &str, op: &str, target: &str) -> Result<String> {
        let _lock = self.lock()?;
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
            by: if PERSON.with(|p| p.get()) {
                "person"
            } else {
                "agent"
            }
            .into(),
            target: target.into(),
        };
        self.push("undo", &entry)?;
        let _ = fs::remove_file(self.journal("redo"));
        Ok(version)
    }

    pub fn undo(&self) -> Result<(String, String)> {
        let _lock = self.lock()?;
        let e = self.pop("undo")?;
        let current = version_of(&fs::read(self.backing())?);
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
        let _lock = self.lock()?;
        let e = self.pop("redo")?;
        let current = version_of(&fs::read(self.backing())?);
        if current != e.version_before {
            self.push("redo", &e)?;
            return Err(QdError::NoUndo("file changed since the undo".into()));
        }
        let v = self.replace(&current, &e.after_text)?;
        self.push("undo", &e)?;
        Ok((v, e.op))
    }

    /// The undo journal's edits made after `ts` (Unix seconds), oldest first.
    pub fn edits_since(&self, ts: u64) -> Vec<Edit> {
        fs::read_to_string(self.journal("undo"))
            .unwrap_or_default()
            .lines()
            .filter_map(|l| serde_json::from_str::<Entry>(l).ok())
            .filter(|e| e.ts > ts)
            .map(|e| Edit {
                ts: e.ts,
                op: e.op,
                by: e.by,
                target: e.target,
            })
            .collect()
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

    /// Spec 006 (T003): attached, every read/write/undo goes to the buffer
    /// overlay and the deck file is untouched; detached, back to the file.
    #[test]
    fn buffer_overlay_is_the_deck_while_attached() {
        let (_d, st) = setup("## A\n\nold\n");
        let disk = || fs::read_to_string(&st.path).unwrap();
        assert!(!st.attached());
        let v = st.attach("## A\n\nunsaved\n").unwrap();
        assert!(st.attached());
        let d = st.read().unwrap();
        assert_eq!(
            (d.text.as_str(), d.version.as_str()),
            ("## A\n\nunsaved\n", v.as_str())
        );

        // a qd edit lands in the overlay (journaled), the file is untouched
        let v2 = st.write(&d, "## A\n\nedited\n", "edit", "").unwrap();
        assert_eq!(st.read().unwrap().text, "## A\n\nedited\n");
        assert_eq!(disk(), "## A\n\nold\n");
        assert!(
            st.write(&d, "x", "edit", "").is_err(),
            "CAS on the overlay's version"
        );

        st.undo().unwrap();
        assert_eq!(st.read().unwrap().text, "## A\n\nunsaved\n");
        st.redo().unwrap();
        assert_eq!(st.read().unwrap().version, v2);

        // the person's typing: CAS on the base, not journaled
        assert_eq!(
            st.set_buffer("stale", "y").unwrap_err().code(),
            "E_CONFLICT"
        );
        st.set_buffer(&v2, "## A\n\ntyped\n").unwrap();
        assert_eq!(st.read().unwrap().text, "## A\n\ntyped\n");

        st.detach();
        assert!(!st.attached());
        assert_eq!(st.read().unwrap().text, "## A\n\nold\n", "back to the file");
    }

    #[test]
    fn version_is_16_hex() {
        let v = version_of(b"abc");
        assert_eq!(v.len(), 16);
        assert_eq!(v, "ba7816bf8f01cfea");
    }

    #[test]
    fn edits_record_who_made_them() {
        let (_d, s) = setup("a\n");
        let d = s.read().unwrap();
        s.write(&d, "b\n", "edit", "one").unwrap();
        let d = s.read().unwrap();
        as_person(|| s.write(&d, "c\n", "title", "one")).unwrap();
        let e = s.edits_since(0);
        let got: Vec<_> = e
            .iter()
            .map(|e| (e.op.as_str(), e.target.as_str(), e.by.as_str()))
            .collect();
        assert_eq!(got, [("edit", "one", "agent"), ("title", "one", "person")]);
        assert!(s.edits_since(u64::MAX).is_empty());
        // entries written before 012 have no `by`: an agent's
        let old: Entry = serde_json::from_str(
            r#"{"ts":1,"op":"x","version_before":"a","version_after":"b","before_text":"","after_text":""}"#,
        )
        .unwrap();
        assert_eq!(old.by, "agent");
    }

    #[test]
    fn write_then_undo_redo() {
        let (_d, s) = setup("## A\n");
        let d0 = s.read().unwrap();
        let v1 = s.write(&d0, "## B\n", "title", "").unwrap();
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
            s.write(&d0, "## B\n", "x", "").unwrap_err().code(),
            "E_CONFLICT"
        );
        assert_eq!(fs::read_to_string(&s.path).unwrap(), "## A\nexternal\n");
    }

    #[test]
    fn undo_refused_after_external_change_and_kept() {
        let (_d, s) = setup("## A\n");
        let d0 = s.read().unwrap();
        s.write(&d0, "## B\n", "x", "").unwrap();
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
        s.write(&d0, "## B\n", "x", "").unwrap();
        s.undo().unwrap();
        let d = s.read().unwrap();
        s.write(&d, "## C\n", "y", "").unwrap();
        assert_eq!(s.redo().unwrap_err().code(), "E_NO_UNDO");
    }

    #[test]
    fn no_temp_files_left() {
        let (d, s) = setup("## A\n");
        let d0 = s.read().unwrap();
        s.write(&d0, "## B\n", "x", "").unwrap();
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
