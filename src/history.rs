//! The deck's history (spec 012), from the person's own `git` (research R1):
//! the versions that changed the deck, the deck at any of them, an old
//! version rendered from a snapshot of its folder, and `qd history --init`.

use crate::deck::{Deck, Slide};
use crate::error::{QdError, Result};
use crate::store::{Edit, Store};
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

/// The deck's folder (`.` for a bare file name).
fn folder(deck: &Path) -> &Path {
    deck.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."))
}

/// The repository the deck at `deck` is in, or `None` outside git.
pub fn repo(deck: &Path) -> Result<Option<Repo>> {
    let dir = folder(deck).to_path_buf();
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
            &["status", "--porcelain", "--", &format!(":/{}", self.path)],
        )
        .is_ok_and(|o| !o.trim().is_empty())
    }

    /// The deck's path at `rev` (it may have been renamed since).
    pub fn path_at(&self, rev: &str) -> Result<String> {
        Ok(log(self, ALL, 0)?
            .0
            .into_iter()
            .find(|v| v.rev.as_deref() == Some(rev))
            .map_or(self.path.clone(), |v| v.path))
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
    diff(old, current)
        .into_iter()
        .filter(|c| matches!(c.kind, "changed" | "added"))
        .map(|c| c.id)
        .collect()
}

/// `changed_since` against the deck at `rev`.
pub fn changed_at(deck_path: &Path, current: &Deck, rev: &str) -> Result<Vec<String>> {
    let repo = repo(deck_path)?.ok_or(QdError::NoRepo)?;
    Ok(changed_since(current, &deck_at(&repo, rev)?.1))
}

/// The deck at `rev` (resolved), parsed from the path it had then.
fn deck_at(repo: &Repo, rev: &str) -> Result<(String, Deck)> {
    let rev = repo.resolve(rev)?;
    let text = repo.text_at(&rev, &repo.path_at(&rev)?)?;
    Ok((rev, Deck::parse(text)))
}

/// One slide that differs between two versions (data-model "SlideChange").
#[derive(Serialize, Debug)]
pub struct SlideChange {
    pub kind: &'static str,
    /// In the newer version (the older for `removed`).
    pub id: String,
    pub title: Option<String>,
    /// 1-based positions.
    pub from: Option<usize>,
    pub to: Option<usize>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub diff: String,
}

/// The slides that differ from `old` to `new` (research R7), in the new order
/// and then the removed ones. Slides match by explicit id, then by a title
/// unique in both, then by position among the rest (slides without an
/// explicit id). `moved` is same text out of order: the slides off the
/// longest run kept in order, so a swap lists one slide, not both.
pub fn diff(old: &Deck, new: &Deck) -> Vec<SlideChange> {
    let (o, n) = (&old.slides, &new.slides);
    let mut pair: Vec<Option<usize>> = vec![None; n.len()]; // new → old
    let mut taken = vec![false; o.len()];
    let unique = |d: &Deck, t: &Option<String>| {
        t.is_some() && d.slides.iter().filter(|s| &s.title == t).count() == 1
    };
    let by_id = |a: &Slide, b: &Slide| a.id_explicit && b.id_explicit && a.id == b.id;
    let by_title =
        |a: &Slide, b: &Slide| a.title == b.title && unique(old, &a.title) && unique(new, &b.title);
    for rule in [&by_id as &dyn Fn(&Slide, &Slide) -> bool, &by_title] {
        for (j, b) in n.iter().enumerate() {
            if pair[j].is_none()
                && let Some(i) = (0..o.len()).find(|&i| !taken[i] && rule(&o[i], b))
            {
                (pair[j], taken[i]) = (Some(i), true);
            }
        }
    }
    let rest_old: Vec<usize> = (0..o.len())
        .filter(|&i| !taken[i] && !o[i].id_explicit)
        .collect();
    let rest_new: Vec<usize> = (0..n.len())
        .filter(|&j| pair[j].is_none() && !n[j].id_explicit)
        .collect();
    for (j, i) in rest_new.into_iter().zip(rest_old) {
        (pair[j], taken[i]) = (Some(i), true);
    }

    let text = |d: &Deck, s: &Slide| d.slice(s.span).trim_end().to_string();
    let same = |j: usize| pair[j].is_some_and(|i| text(old, &o[i]) == text(new, &n[j]));
    // unchanged slides in new order, as old indices; out of the longest
    // in-order run (Myers) means moved
    let kept: Vec<usize> = (0..n.len()).filter(|&j| same(j)).collect();
    let in_new: Vec<usize> = kept.iter().map(|&j| pair[j].unwrap()).collect();
    let mut in_old = in_new.clone();
    in_old.sort_unstable();
    let mut moved = vec![false; n.len()];
    for op in similar::capture_diff_slices(similar::Algorithm::Myers, &in_old, &in_new) {
        if op.tag() != similar::DiffTag::Equal {
            for k in op.new_range() {
                moved[kept[k]] = true;
            }
        }
    }

    let change = |kind, s: &Slide, from: Option<usize>, to: Option<usize>, diff| SlideChange {
        kind,
        id: s.id.clone(),
        title: s.title.clone(),
        from: from.map(|i| i + 1),
        to: to.map(|j| j + 1),
        diff,
    };
    let mut out: Vec<SlideChange> = n
        .iter()
        .enumerate()
        .filter_map(|(j, s)| match pair[j] {
            None => Some(change("added", s, None, Some(j), String::new())),
            Some(i) if !same(j) => Some(change(
                "changed",
                s,
                Some(i),
                Some(j),
                crate::ops::diff(&text(old, &o[i]), &text(new, s)),
            )),
            Some(i) if moved[j] => Some(change("moved", s, Some(i), Some(j), String::new())),
            _ => None,
        })
        .collect();
    out.extend(
        (0..o.len())
            .filter(|&i| !taken[i])
            .map(|i| change("removed", &o[i], Some(i), None, String::new())),
    );
    out
}

/// `qd diff FROM [TO]`: the slides that changed from one version to another,
/// or to the current file when `to` is `None` (FR-013).
pub fn compare(
    deck_path: &Path,
    current: &Deck,
    from: &str,
    to: Option<&str>,
) -> Result<serde_json::Value> {
    let repo = repo(deck_path)?.ok_or(QdError::NoRepo)?;
    let (from, old) = deck_at(&repo, from)?;
    let (to, slides) = match to {
        Some(t) => {
            let (t, new) = deck_at(&repo, t)?;
            (t, diff(&old, &new))
        }
        None => ("uncommitted".into(), diff(&old, current)),
    };
    Ok(serde_json::json!({"kind": "diff", "from": from, "to": to, "slides": slides}))
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
    let more = match slide {
        None => {
            let (v, more) = log(&repo, limit, skip)?;
            out.extend(v);
            more
        }
        Some(r) => {
            // walk every version, keep the ones where the slide's text changed
            // from the version before it (oldest last; trailing blank lines
            // aside: a slide moved to the end has none); ponytail: a parse per
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
            let more = changed.len() > skip + limit;
            out.extend(changed.into_iter().skip(skip).take(limit));
            more
        }
    };
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
    let dir = folder(deck_path);
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

/// A commit message for these edits (data-model "Commit message"): ops in
/// order, repeats folded, who made them; capped at 72 characters.
pub fn message(edits: &[Edit]) -> String {
    if edits.is_empty() {
        return "qd: edit source (person)".into(); // typing in the source pane
    }
    let mut ops: Vec<(String, usize)> = vec![];
    for e in edits {
        let op = format!("{} {}", e.op, e.target).trim().to_string();
        match ops.iter_mut().find(|(o, _)| *o == op) {
            Some((_, n)) => *n += 1,
            None => ops.push((op, 1)),
        }
    }
    let items: Vec<String> = ops
        .into_iter()
        .map(|(o, n)| if n > 1 { format!("{o} ×{n}") } else { o })
        .collect();
    let who: Vec<&str> = ["person", "agent"]
        .into_iter()
        .filter(|w| edits.iter().any(|e| e.by == *w))
        .collect();
    let tail = format!(" ({})", who.join(", "));
    let mut k = items.len();
    let mut subject = format!("qd: {}{tail}", items.join(", "));
    while subject.chars().count() > 72 && k > 1 {
        k -= 1;
        subject = format!("qd: {}, …{tail}", items[..k].join(", "));
    }
    subject
}

/// Commit the deck alone (research R2): other files' staged and unstaged
/// state is untouched. `None` when the deck equals HEAD. Never during a
/// merge, rebase or detached head (R3). git's own configuration applies:
/// signing and hooks; a refusal comes back with git's message.
pub fn commit_deck(repo: &Repo, message: &str) -> Result<Option<String>> {
    if let Some(why) = repo.busy() {
        return Err(QdError::RepoBusy(why.into()));
    }
    if !repo.uncommitted() {
        return Ok(None);
    }
    let path = format!(":/{}", repo.path);
    git(&repo.dir, &["add", "--", &path])?; // an untracked deck
    git(
        &repo.dir,
        &["commit", "-q", "--only", "-m", message, "--", &path],
    )?;
    repo.resolve("HEAD").map(Some)
}

/// Save as git commit (US2): the deck's edits since its last commit, as one
/// commit. `E_NO_REPO` outside git.
pub fn commit_edits(store: &Store) -> Result<Option<String>> {
    let repo = repo(&store.path)?.ok_or(QdError::NoRepo)?;
    let last = log(&repo, 1, 0)?.0.first().map_or(0, |v| v.time);
    // ponytail: the journal keeps seconds, so an edit in the same second as
    // the last commit is named again; millisecond stamps if that matters
    commit_deck(&repo, &message(&store.edits_since(last.saturating_sub(1))))
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
    let text = repo.text_at(&rev, &repo.path_at(&rev)?)?;
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
    crate::render::render(&snap_deck, &text, "hist", &out, true)
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

    fn e(op: &str, target: &str, by: &str) -> Edit {
        Edit {
            ts: 0,
            op: op.into(),
            target: target.into(),
            by: by.into(),
        }
    }

    #[test]
    fn commit_messages() {
        assert_eq!(message(&[]), "qd: edit source (person)");
        assert_eq!(
            message(&[e("title", "one", "agent")]),
            "qd: title one (agent)"
        );
        let three = [
            e("edit", "one/1", "person"),
            e("title", "two", "agent"),
            e("edit", "one/1", "person"),
            e("edit", "one/1", "person"),
        ];
        assert_eq!(
            message(&three),
            "qd: edit one/1 ×3, title two (person, agent)"
        );
        let many: Vec<Edit> = (0..20)
            .map(|i| e("title", &format!("slide-{i}"), "agent"))
            .collect();
        let m = message(&many);
        assert!(m.chars().count() <= 72, "{m}");
        assert!(m.starts_with("qd: title slide-0, title slide-1"), "{m}");
        assert!(m.ends_with(", … (agent)"), "{m}");
    }

    #[test]
    fn diff_lists_only_changed_slides() {
        let old = Deck::parse(
            "## A {#a}\n\nx\n\n## Bee\n\ny\n\n## C {#c}\n\nz\n\n## D {#d}\n\nw\n\n## Gone {#gone}\n\nq\n\n## Same\n\n1\n\n## Same\n\n2\n".into(),
        );
        let new = Deck::parse(
            "## D {#d}\n\nw\n\n## A {#a}\n\nx\n\n## Bee\n\nY2\n\n## C {#c}\n\nz\n\n## Same\n\n1\n\n## Same\n\n2b\n\n## New {#new}\n\nn\n".into(),
        );
        let got: Vec<(String, String, Option<usize>, Option<usize>)> = diff(&old, &new)
            .iter()
            .map(|c| (c.kind.into(), c.id.clone(), c.from, c.to))
            .collect();
        let want = [
            ("moved", "d", Some(4), Some(1)),
            ("changed", "bee", Some(2), Some(3)),
            // two slides titled Same: matched by position among the rest
            ("changed", new.slides[5].id.as_str(), Some(7), Some(6)),
            ("added", "new", None, Some(7)),
            ("removed", "gone", Some(5), None),
        ];
        let want: Vec<_> = want
            .iter()
            .map(|(k, i, f, t)| (k.to_string(), i.to_string(), *f, *t))
            .collect();
        assert_eq!(got, want);
        let bee = &diff(&old, &new)[1];
        assert!(
            bee.diff.contains("-y\n") && bee.diff.contains("+Y2\n"),
            "{}",
            bee.diff
        );
        assert!(diff(&old, &old).is_empty());
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
