# Research: History (012)

Checked on this machine with git 2.x in a scratch repository with a deck
in a subfolder (2026-11-12). The probe is recorded under each decision.

## R1. Talking to git: the `git` command, not a library

Decision: qd runs `git` (found on PATH, like `quarto`), always with `-C
<deck dir>` and the deck's path relative to it. It never runs git with flags
that change the person's configuration (signing, hooks, author: FR-011).

Rationale: using the person's own git binary means their config, signing
program (1Password's SSH agent here), hooks and credential helpers behave
exactly as for their own commits. The spec asks for that. A library (git2 /
gitoxide) would re-implement signing and hooks, or skip them, and adds a
large dependency (constitution VII).

Alternatives considered: `git2` (libgit2). Rejected because it doesn't run
hooks, has its own signing story, and adds a C dependency.

## R2. Committing only the deck

Decision: a qd commit is

```text
git -C <dir> add -- <deck>                       # only if git doesn't track it yet
git -C <dir> commit --only -m <message> -- <deck>
```

and it runs only when `git diff --quiet HEAD -- <deck>` says the deck
differs from HEAD (so a deck the person already committed gives no empty
commit, and no error).

Probe: with `other.txt` staged and then changed again (`MM`), and the deck
changed, `commit --only -- deck` committed the deck alone, and `other.txt`
was still `MM` afterwards. An untracked deck makes `commit --only` fail
("pathspec did not match"), hence the `add` first; it stages only the deck.
`commit --only` with nothing to commit exits 1, hence the `diff --quiet`
check.

`--only` also writes the deck's committed content to the index for that
path. Any partial staging of the deck itself is replaced by the committed
version, which is what a commit of that file means.

## R3. When not to commit

Decision: before committing, qd checks the repository state and skips (and
reports once) when any of these exist in the git dir: `MERGE_HEAD`,
`CHERRY_PICK_HEAD`, `REVERT_HEAD`, `rebase-merge/`, `rebase-apply/`, `BISECT_LOG`,
or when `git symbolic-ref -q HEAD` fails (detached head). The git dir comes
from `git rev-parse --git-dir`, so worktrees work.

## R4. When to commit: the pause, and who commits

Decision:

- **With `qd serve` running** (the editor, or the CLI talking to the same
  deck), the server commits. It already sees every change to the file (live
  reload). On each change it (re)starts a timer of `history.pause` seconds
  (default 30). When the timer fires, it commits if the deck differs from
  HEAD. On shutdown it commits what is pending.
- **Without a server**, a CLI edit commits at once, after the write, when the
  setting is on. Each agent command is then its own commit; the server is
  how bursts become one.
- **The message** lists the edits since the last commit, read from the undo
  journal (entries newer than the last commit's time: their `op` names, with
  repeats folded as `edit ×3`). It ends with who made them: ` (person)`,
  ` (agent)` or ` (person, agent)`. The journal gains a `by` field for this.
  Edits made through the server's `/api/op` are `person`, and CLI processes
  are `agent`. The person's typing in the source pane is not journaled (by
  design, 004), so a change with no journal entry reads `edit source
  (person)`.

Example: `qd: edit results/1 ×3, title methods (person, agent)`.

## R5. Reading history

Decision:

- **List**: `git log --follow --format=%H%x1f%at%x1f%an%x1f%ae%x1f%s
  --name-only -- <deck>`, which also gives the deck's path at each commit
  (renames). `--follow` handles one path, which is all we need. Paging uses
  `--max-count`/`--skip`. "Made by qd" means the subject starts with `qd:`.
- **Text at a version**: `git show <rev>:<path at that rev>`.
- **Uncommitted**: `git diff --quiet HEAD -- <deck>` (exit 1 means changed).
- **Per slide** (`--slide REF`): walk the list, parse each version
  (`Deck::parse`), and keep the commits where that slide's text differs from
  the previous version's. The slide is matched as in R7. This costs a parse
  per commit, which is fine for hundreds of commits; for thousands, `--limit`
  applies first.

Probe: `git mv deck.qmd talk.qmd` then `log --follow -- talk.qmd` listed all
three commits, with `talks/deck.qmd` as the path for the two older ones.

## R6. Rendering an old version

Decision: render it in a snapshot of the deck's folder at that revision, in
qd's cache:

```text
git -C <dir> archive --format=zip -o <cache>/history/<rev>.zip <rev> -- .
```

then unzip into `<cache>/history/<rev>/` with the `zip` crate (already a
dependency). That gives the folder as it was, with its images, data and
`_quarto.yml`. Probe: run from the deck's subfolder, the archive's paths are
relative to that folder (`deck.qmd` at the top), so the snapshot is just
the deck's folder. Then the
existing preview render runs on `<path at rev>` there, with `--no-execute`
(FR-005), and the result is cached by revision. The server serves it under
`/history/<rev>/`, and the History panel shows it in a second preview.

Rationale: nothing is written beside the deck (the current preview copy
goes beside it, but an old version would bring old images). It is also what
"as it was" means.

Limits: files the person never committed are missing in the snapshot. The
preview's existing check marks them. A `_quarto.yml` above the deck's folder
is not in the snapshot; the plan accepts that (rare for talks) and the
preview notes it. Planning-level choice, recorded in plan.md.

## R7. Matching slides across versions

Decision: a slide in A matches a slide in B by explicit id first, then by
title (if unique in both), then by position among the remaining ones. Diff
kinds: `added`, `removed`, `moved` (same text, different position),
`changed` (text differs; the position may also differ). The line diff uses
`similar` (already a dependency, `ops::diff`).

## R8. Restore

Decision: one patch through the existing edit path (`ctx.edit`, a single
journal entry, `--expect`):

- **deck**: the whole file becomes that version's text;
- **slide**: the matched current slide's span is replaced by the old
  slide's text. If it no longer exists, it is inserted after the nearest
  earlier slide (in the old order) that still exists, or first;
- **element**: in the matched current slide, the element with the same
  explicit id (or the same `slide/i`) is replaced by the old one's text.
  Without a current slide: refused with a hint to restore the slide.

Undo and conflict checks come for free from the edit path.

## R9. Outside a repository

Decision: `git rev-parse --show-toplevel` failing means no repository.
History shows "Start a history here", and `qd history --init` runs `git -C
<deck dir> init` and then a first commit of the deck alone (R2). It is never
run automatically (Clarifications). Creating a repository writes outside the
deck file, so `--init` over HTTP needs the session token.

## R10. Tests and signing

Every test repository sets `commit.gpgsign false` and a fixed user in its
local config. Without that, the person's global signing (1Password here)
would prompt during `cargo test`. A test also checks that qd adds no
`-c commit.gpgsign=…` or `--no-verify` of its own: a test repository with a
`pre-commit` hook that writes a marker file must get the marker from a qd
commit (FR-011).
