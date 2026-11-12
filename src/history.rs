//! The deck's history (spec 012), from the person's own `git` (research R1):
//! the versions that changed the deck, the deck at any of them, an old
//! version rendered from a snapshot of its folder, and `qd history --init`.

use crate::deck::{Deck, Slide};
use crate::error::{QdError, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::Command;

/// `git -C dir args…`: stdout, or `E_NO_GIT` / `E_GIT` with git's message.
/// The person's configuration applies untouched (signing, hooks: FR-011).
pub fn git(dir: &Path, args: &[&str]) -> Result<String> {
    let o = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => QdError::NoGit,
            _ => QdError::Io(e),
        })?;
    if !o.status.success() {
        return Err(QdError::Git(
            String::from_utf8_lossy(&o.stderr).trim().to_string(),
        ));
    }
    Ok(String::from_utf8_lossy(&o.stdout).into_owned())
}

/// "Every version" for `log` (git takes a 32-bit count).
const ALL: usize = 1_000_000;

/// The repository a deck is in.
pub struct Repo {
    /// The deck's folder: every git call runs here.
    pub dir: PathBuf,
    /// The repository's git dir (for its merge/rebase state files).
    pub git_dir: PathBuf,
    /// The deck's path relative to the repository's top level.
    pub path: String,
}

/// The repository the deck at `deck` is in, or `None` outside git.
pub fn repo(deck: &Path) -> Result<Option<Repo>> {
    let dir = deck
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
        .to_path_buf();
    let out = match git(&dir, &["rev-parse", "--show-prefix", "--absolute-git-dir"]) {
        Ok(o) => o,
        Err(QdError::Git(_)) => return Ok(None),
        Err(e) => return Err(e),
    };
    let mut lines = out.lines();
    let prefix = lines.next().unwrap_or_default();
    let git_dir = PathBuf::from(lines.next().unwrap_or_default());
    let name = deck.file_name().unwrap_or_default().to_string_lossy();
    Ok(Some(Repo {
        dir,
        git_dir,
        path: format!("{prefix}{name}"),
    }))
}

impl Repo {
    /// Why qd must not commit now, if it must not (research R3).
    pub fn busy(&self) -> Option<&'static str> {
        for (f, why) in [
            ("MERGE_HEAD", "a merge is in progress"),
            ("CHERRY_PICK_HEAD", "a cherry-pick is in progress"),
            ("REVERT_HEAD", "a revert is in progress"),
            ("rebase-merge", "a rebase is in progress"),
            ("rebase-apply", "a rebase is in progress"),
            ("BISECT_LOG", "a bisect is in progress"),
        ] {
            if self.git_dir.join(f).exists() {
                return Some(why);
            }
        }
        git(&self.dir, &["symbolic-ref", "-q", "HEAD"])
            .is_err()
            .then_some("HEAD is detached")
    }

    /// Does the deck differ from HEAD (or is it not committed at all)?
    pub fn uncommitted(&self) -> bool {
        git(
            &self.dir,
            &["diff", "--quiet", "HEAD", "--", &format!(":/{}", self.path)],
        )
        .is_err()
            || git(
                &self.dir,
                &[
                    "ls-files",
                    "--error-unmatch",
                    "--",
                    &format!(":/{}", self.path),
                ],
            )
            .is_err()
    }

    /// A revision as a full commit hash (`E_NOT_FOUND` if git doesn't know it).
    pub fn resolve(&self, rev: &str) -> Result<String> {
        git(
            &self.dir,
            &["rev-parse", "--verify", "-q", &format!("{rev}^{{commit}}")],
        )
        .map(|h| h.trim().to_string())
        .map_err(|_| QdError::NotFound(format!("revision '{rev}'")))
    }

    /// The deck's text at `rev`, at the path it had then.
    pub fn text_at(&self, rev: &str, path: &str) -> Result<String> {
        git(&self.dir, &["show", &format!("{rev}:{path}")])
            .map_err(|_| QdError::NotFound(format!("the deck at {rev}")))
    }
}

/// A version of the deck (data-model "Version").
#[derive(Serialize, Clone, Debug)]
pub struct Version {
    pub rev: Option<String>,
    pub short: String,
    pub time: u64,
    pub author: String,
    pub message: String,
    pub qd: bool,
    pub path: String,
}

/// Commits that changed the deck, newest first, following renames, after
/// `skip`, at most `limit` (and whether there are more).
pub fn log(repo: &Repo, limit: usize, skip: usize) -> Result<(Vec<Version>, bool)> {
    if git(&repo.dir, &["rev-parse", "-q", "--verify", "HEAD"]).is_err() {
        return Ok((vec![], false)); // a repository with no commits yet
    }
    let out = git(
        &repo.dir,
        &[
            "log",
            "--follow",
            "--format=%x1e%H%x1f%at%x1f%an%x1f%s",
            "--name-only",
            &format!("--max-count={}", limit + 1),
            &format!("--skip={skip}"),
            "--",
            &format!(":/{}", repo.path),
        ],
    )?;
    let mut v: Vec<Version> = out
        .split('\x1e')
        .filter(|r| !r.trim().is_empty())
        .filter_map(|r| {
            let (head, names) = r.split_once('\n')?;
            let f: Vec<&str> = head.split('\x1f').collect();
            let [hash, time, author, message] = f[..] else {
                return None;
            };
            Some(Version {
                rev: Some(hash.into()),
                short: hash[..7.min(hash.len())].into(),
                time: time.parse().unwrap_or(0),
                author: author.into(),
                message: message.into(),
                qd: message.starts_with("qd:"),
                path: names
                    .lines()
                    .map(str::trim)
                    .rfind(|l| !l.is_empty())
                    .unwrap_or(&repo.path)
                    .into(),
            })
        })
        .collect();
    let more = v.len() > limit;
    v.truncate(limit);
    Ok((v, more))
}

/// The slide in `d` that is `target` (by id, then a title unique in both,
/// then position: research R7).
pub fn match_slide<'a>(d: &'a Deck, target: &Slide, unique_title: bool) -> Option<&'a Slide> {
    if let Some(s) = d.slides.iter().find(|s| s.id == target.id) {
        return Some(s);
    }
    let same_title: Vec<&Slide> = d
        .slides
        .iter()
        .filter(|s| s.title.is_some() && s.title == target.title)
        .collect();
    if unique_title && same_title.len() == 1 {
        return Some(same_title[0]);
    }
    d.slides.get(target.index)
}

/// The current deck's slides that differ from `old` (or aren't in it), by
/// id: what the editor marks while a version is open (US1).
pub fn changed_since(current: &Deck, old: &Deck) -> Vec<String> {
    current
        .slides
        .iter()
        .filter(|s| {
            match_slide(old, s, true)
                .is_none_or(|o| old.slice(o.span).trim_end() != current.slice(s.span).trim_end())
        })
        .map(|s| s.id.clone())
        .collect()
}

/// `changed_since` against the deck at `rev`.
pub fn changed_at(deck_path: &Path, current: &Deck, rev: &str) -> Result<Vec<String>> {
    let repo = repo(deck_path)?.ok_or(QdError::NoRepo)?;
    let rev = repo.resolve(rev)?;
    let (versions, _) = log(&repo, ALL, 0)?;
    let path = versions
        .iter()
        .find(|v| v.rev.as_deref() == Some(rev.as_str()))
        .map_or(repo.path.clone(), |v| v.path.clone());
    Ok(changed_since(
        current,
        &Deck::parse(repo.text_at(&rev, &path)?),
    ))
}

/// `qd history`: the versions, the uncommitted state first; with `slide`,
/// only those in which that slide's text changed (FR-002).
pub fn history(
    deck_path: &Path,
    current: &Deck,
    slide: Option<&str>,
    limit: usize,
    skip: usize,
) -> Result<serde_json::Value> {
    let repo = repo(deck_path)?.ok_or(QdError::NoRepo)?;
    let mut out = vec![];
    let uncommitted = repo.uncommitted();
    if uncommitted && skip == 0 {
        out.push(Version {
            rev: None,
            short: "uncommitted".into(),
            time: std::fs::metadata(deck_path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |d| d.as_secs()),
            author: String::new(),
            message: "Not committed".into(),
            qd: false,
            path: repo.path.clone(),
        });
    }
    let more;
    match slide {
        None => {
            let (v, m) = log(&repo, limit, skip)?;
            out.extend(v);
            more = m;
        }
        Some(r) => {
            // walk every version, keep the ones where the slide's text changed
            // (trailing blank lines aside: a slide moved to the end has none)
            // from the version before it (oldest last); ponytail: a parse per
            // commit, fine for hundreds; --limit bounds what is returned
            let target = current.slide(r)?;
            let (all, _) = log(&repo, ALL, 0)?;
            let text_of = |v: &Version| -> Option<String> {
                let t = repo.text_at(v.rev.as_deref()?, &v.path).ok()?;
                let d = Deck::parse(t);
                match_slide(&d, target, true).map(|s| d.slice(s.span).trim_end().to_string())
            };
            let texts: Vec<Option<String>> = all.iter().map(text_of).collect();
            let now = Some(current.slice(target.span).trim_end().to_string());
            if uncommitted && skip == 0 && texts.first() == Some(&now) {
                out.clear(); // the slide is the same as committed
            }
            let changed: Vec<Version> = all
                .iter()
                .enumerate()
                .filter(|(i, _)| texts.get(i + 1) != Some(&texts[*i]))
                .map(|(_, v)| v.clone())
                .collect();
            more = changed.len() > skip + limit;
            out.extend(changed.into_iter().skip(skip).take(limit));
        }
    }
    Ok(serde_json::json!({"kind": "history", "path": repo.path, "versions": out, "more": more}))
}

/// `qd history --init`: a repository in the deck's folder, with a first
/// commit of the deck alone. Never run on its own (spec Clarifications).
pub fn init(deck_path: &Path) -> Result<serde_json::Value> {
    if repo(deck_path)?.is_some() {
        return Err(QdError::InvalidArg(
            "the deck is already in a git repository".into(),
        ));
    }
    let dir = deck_path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    git(dir, &["init", "-q"])?;
    let repo = repo(deck_path)?.ok_or(QdError::NoRepo)?;
    let name = deck_path.file_name().unwrap_or_default().to_string_lossy();
    git(dir, &["add", "--", &name])?;
    git(
        dir,
        &[
            "commit",
            "-q",
            "--only",
            "-m",
            "qd: start history",
            "--",
            &name,
        ],
    )?;
    let rev = repo.resolve("HEAD")?;
    Ok(serde_json::json!({"ok": true, "repo": dir.canonicalize()?, "rev": rev}))
}

/// Render the deck as it was at `rev`, without running code (FR-005), from a
/// snapshot of its folder at that revision in `cache/history/<rev>/` (research
/// R6); reused once made. Returns the HTML path.
pub fn render_version(deck_path: &Path, cache: &Path, rev: &str) -> Result<PathBuf> {
    let repo = repo(deck_path)?.ok_or(QdError::NoRepo)?;
    let rev = repo.resolve(rev)?;
    let root = cache.join("history").join(&rev);
    let out = root.join("out");
    let name = deck_path.file_stem().unwrap_or_default().to_string_lossy();
    let html = out.join(format!("{name}.qd-hist.html"));
    if html.is_file() {
        return Ok(html);
    }
    // the deck's path then (renames), relative to its folder
    let (versions, _) = log(&repo, ALL, 0)?;
    let path = versions
        .iter()
        .find(|v| v.rev.as_deref() == Some(rev.as_str()))
        .map_or(repo.path.clone(), |v| v.path.clone());
    let text = repo.text_at(&rev, &path)?;
    let snap = root.join("snapshot");
    let _ = std::fs::remove_dir_all(&snap);
    std::fs::create_dir_all(&snap)?;
    let zip = root.join("snapshot.zip");
    git(
        &repo.dir,
        &[
            "archive",
            "--format=zip",
            "-o",
            &zip.to_string_lossy(),
            &rev,
            "--",
            ".",
        ],
    )?;
    zip::ZipArchive::new(std::fs::File::open(&zip)?)
        .and_then(|mut z| z.extract(&snap))
        .map_err(|e| QdError::Io(std::io::Error::other(e)))?;
    let _ = std::fs::remove_file(&zip);
    // rendered under today's name, next to the snapshot's files
    let snap_deck = snap.join(format!("{name}.qmd"));
    let made = crate::render::render(&snap_deck, &text, "hist", &out, true)?;
    Ok(made)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn repo_with_deck() -> (tempfile::TempDir, PathBuf, impl Fn(&[&str]) -> String) {
        let t = tempfile::tempdir().unwrap();
        let top = t.path().to_path_buf();
        let g = move |args: &[&str]| git(&top, args).unwrap();
        g(&["init", "-q", "-b", "main"]);
        for (k, v) in [
            ("user.name", "T"),
            ("user.email", "t@example.com"),
            ("commit.gpgsign", "false"),
        ] {
            g(&["config", k, v]);
        }
        std::fs::create_dir_all(t.path().join("talks")).unwrap();
        let deck = t.path().join("talks/deck.qmd");
        std::fs::write(&deck, "## One {#one}\n\nA\n").unwrap();
        g(&["add", "."]);
        g(&["commit", "-qm", "first"]);
        (t, deck, g)
    }

    #[test]
    fn finds_the_repository_from_a_subfolder() {
        let (t, deck, _g) = repo_with_deck();
        let r = repo(&deck).unwrap().unwrap();
        assert_eq!(r.path, "talks/deck.qmd");
        assert!(r.git_dir.ends_with(".git"));
        assert!(!r.uncommitted());
        std::fs::write(&deck, "## One {#one}\n\nB\n").unwrap();
        assert!(r.uncommitted());
        let outside = tempfile::tempdir().unwrap();
        assert!(repo(&outside.path().join("x.qmd")).unwrap().is_none());
        drop(t);
    }

    #[test]
    fn busy_during_a_merge_or_detached() {
        let (_t, deck, g) = repo_with_deck();
        let r = repo(&deck).unwrap().unwrap();
        assert_eq!(r.busy(), None);
        std::fs::write(r.git_dir.join("MERGE_HEAD"), "x").unwrap();
        assert_eq!(r.busy(), Some("a merge is in progress"));
        std::fs::remove_file(r.git_dir.join("MERGE_HEAD")).unwrap();
        g(&["checkout", "-q", "--detach"]);
        assert_eq!(r.busy(), Some("HEAD is detached"));
    }

    #[test]
    fn git_errors_carry_the_message() {
        let (_t, deck, _g) = repo_with_deck();
        let r = repo(&deck).unwrap().unwrap();
        let e = git(&r.dir, &["show", "nosuch:x"]).err().unwrap();
        assert_eq!(e.code(), "E_GIT");
        assert!(r.resolve("nosuch").is_err());
    }
}
