# Implementation Plan: History

**Branch**: `012-history` | **Date**: 2026-11-12 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/012-history/spec.md`

## Summary

The deck's git history goes into the editor and the CLI. A new module
`src/history.rs` runs the person's own `git`. It lists the versions that
touched the deck (following renames), reads the deck at any version,
compares two versions slide by slide, and restores the deck, a slide or an
element as one ordinary qd edit. With the setting **Save as git commit** on,
`qd serve` commits the deck alone after a pause (a burst of edits becomes one
commit), and a CLI edit with no server commits at once. Commits follow the
person's git configuration: signing, hooks and author. Old versions render
without running code, from a snapshot of the deck's folder in qd's cache. The
editor gets a History panel, a second preview beside the current deck, and a
slide-by-slide comparison.

## Technical Context

**Language/Version**: Rust stable; plain ES modules for the editor
**Primary Dependencies**: existing crates (`similar` for line diffs, `zip` to unpack snapshots, `notify` already watching the deck); the `git` command at run time, like `quarto`
**Storage**: git itself. qd's cache keeps rendered old versions (`<cache>/history/<rev>/`) and nothing else. The undo journal gains a `by` field.
**Testing**: `cargo test`: unit tests for slide matching and restore patches; CLI tests in temp git repositories (signing off locally, a test hook); browser tests for the panel
**Target Platform**: macOS first, Linux
**Project Type**: CLI plus a localhost web editor (single binary)
**Performance Goals**: History lists 1,000 commits in under 2 s (SC-005); `--slide` parses one version per commit, with `--limit` first
**Constraints**: only the deck is ever committed; other files' index state is untouched; no history rewriting; no code runs for old versions; no git flags that change the person's config
**Scale/Scope**: one module (~400 lines), a server timer, three CLI commands, one panel and a second preview

External tools: git (any modern 2.x); Quarto as before. Licences: no new
dependencies.

## Constitution Check

| Principle | Check |
|---|---|
| I. Clean room | Built on git's documented commands and our own probes (research). Pass. |
| II. `.qmd` is the truth | History is git's history of the `.qmd`. qd keeps no second history (Clarifications); restores are minimal patches. Pass. |
| III. CLI first | `qd history`, `qd diff`, `qd restore`, and `qd history --init` come first, with JSON; the editor calls them. Pass. |
| IV. Live preview | The current preview is unchanged; old versions get a second, real Quarto render. Pass. |
| V. Safe concurrent edits | A restore goes through the edit path: `--expect`, atomic write, one undo step. Commits read the file, never write it. Pass. |
| VI. Test first | Matching, diff and restore are unit-tested; commits are tested in real temp repositories; browser tests cover the panel. Pass. |
| VII. Simplicity | The `git` command instead of a library; git as the only store; renders cached by revision. Pass. |
| Constraints: code runs only on an explicit action | Old versions render with `--no-execute`. Running their cells is out of scope here (spec US1-3 offers it; it uses 004's Run, on the snapshot). Pass. |

No Complexity Tracking entries.

## Project Structure

### Documentation (this feature)

```text
specs/012-history/
├── plan.md, research.md, data-model.md, quickstart.md
├── contracts/cli.md, contracts/http.md
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
src/
├── history.rs     # NEW: git(), repo state, log (+follow, per slide), show, diff by slide,
│                  #      restore patches, commit_deck(), snapshot + render of a version
├── store.rs       # journal Entry gains `by` (person|agent); entries since a time
├── cli.rs         # qd history [--slide|--element] [--limit] [--init], qd diff, qd restore;
│                  # after an edit with no server: commit when the setting is on
├── server.rs      # pause timer → commit; GET /api/history, /api/history/diff,
│                  # POST /api/history/render (token), /api/history/init (token);
│                  # /history/<rev>/… serves old renders; /api/op edits journal by=person
├── settings.rs    # history.commit (Bool, false), history.pause (Number, 30)
├── error.rs       # NoRepo (E_NO_REPO), NoGit (E_NO_GIT), RepoBusy (E_REPO_BUSY)
└── ui/
    ├── history.js # NEW: the History panel, second preview, compare view
    ├── actions.json, editor.js, editor.css, index.html
tests/
├── history.rs     # NEW: CLI tests in temp git repos
└── onslide.rs     # panel, preview, restore from the menu
```

**Structure Decision**: single crate as before. History logic goes in one
module. The editor part goes in its own `history.js`, because editor.js is
already large.

## Design

### git access (`history.rs`)

`git(dir, args) -> Result<String>` runs `git -C dir args…` with the
environment untouched. It returns stdout, or `E_NO_GIT` when git isn't
installed, and any other failure as `QdError::Git(stderr)` (code `E_GIT`).
`repo(deck) -> Option<Repo>` holds the top level, the git dir and the
deck's path relative to the top level. Outside a repository it returns
`None`, and the commands report `E_NO_REPO` with the hint
`qd history --init`.

### Versions

`log(repo, limit, skip) -> Vec<Version>` (research R5) plus the
uncommitted entry. `text_at(repo, rev) -> String` uses the path that
version had. `log_slide(repo, ref, limit)` filters by the slide's text,
using the matching in R7.

### Comparing

`diff(a: &Deck, b: &Deck) -> Vec<SlideChange>` matches slides (id, unique
title, position) and classifies each pair (data-model). A `changed` entry
carries `ops::diff` of the two slides' texts. `qd diff REV [REV]` compares
REV with the current file, or REV with REV.

### Restoring

`restore(current, old, what) -> Patch`. `what` is the deck, a slide ref
resolved in the old version, or an element ref resolved in the old version.
The patch follows R8. It goes through `ctx.edit` as op `restore <what> from
<short rev>`, so the edit path supplies `--expect`, the atomic write,
undo/redo and the live preview.

### Committing

`commit_deck(repo, message) -> Result<Option<String>>`:

1. If the repository is mid-merge, mid-rebase or similar, or on a detached
   head, return `E_REPO_BUSY` (R3).
2. If the deck equals HEAD, return `None`.
3. If git doesn't track the deck yet, `git add -- <deck>`.
4. Run `git commit --only -m <message> -- <deck>`. A failure is returned with
   git's stderr (a signing failure, a refusing hook) and the file is left as
   it is.

The message comes from `journal_since(last commit time)` (R4).

**Server**: the file-change handler that already exists (`watch`) also
calls `pending.touch()` when `history.commit` is on and the deck is in a
repository. A task sleeps until `last_touch + pause`, then commits. The
result goes to the editor as an SSE event `history` (`committed`, `failed`
with its reason, or `skipped` with its reason, once per state). On
`/api/shutdown` and on stop, anything pending is committed first.

**CLI**: after a successful edit, if the setting is on and
`server::running` says no server is up, it commits at once and adds
`"commit": {rev | error}` to the edit's JSON.

### Rendering an old version

`render_version(repo, rev) -> PathBuf`: if `<cache>/history/<rev>/` holds a
finished render, use it. Otherwise snapshot the folder (`git archive
--format=zip` and unzip; R6) and run `render::render(snapshot/<deck>, text,
"hist", out, preview: true)`, which already means `--no-execute` and no
embedded resources. Served at `/history/<rev>/<deck>.html` from the cache,
read-only.

### Editor (`ui/history.js`)

- **View ▸ History** (action `history`, ⌥⌘H) opens a panel next to the
  slide list.
  - It shows versions newest first: "Not committed" at the top, then each
    commit's time (relative), author, message and a qd badge. It loads 50
    at a time, more on scroll.
  - Outside a repository it shows **Start a history here**.
- **Selecting a version** splits the canvas: the current deck on one side,
  that version's render on the other.
  - The slide list marks the slides that differ from that version, from
    `/api/history/diff`.
  - Arrow keys step both previews together, through matched slides.
- **Compare** (action `history-compare`) shows the changed slides in a
  list. Each one opens its source diff (lines marked) and the two renders
  side by side.
- **Restore** is offered while a version is open:
  - **Restore this version…** (whole deck), with a confirmation sheet;
  - **Restore slide from this version** on the slide right-click menu and
    the slide bar;
  - **Restore element from this version** on the element right-click menu.

  Each runs `qd restore` through `/api/op` with `expect`.
- **Settings**: **Save as git commit** and **Pause before committing
  (seconds)** in the History section. A failed commit shows a notice with
  git's reason, and once fixed (1Password unlocked) it clears at the next
  pause.

## Phasing (for tasks)

- Foundational: `history.rs` git access, repo detection, errors, the
  journal's `by`, the settings, and the test repository helper.
- US1 (P1): `qd history` (+`--slide`, `--limit`, `--init`), snapshot
  rendering, the panel and the second preview.
- US2 (P1): `commit_deck`, the server timer, CLI commit without a server,
  the messages, the SSE notice.
- US3 (P2): slide matching, `qd diff`, the compare view.
- US4 (P2): restore patches, `qd restore`, the menu entries.
- Polish: README, skill, ROADMAP, action inventory, quickstart run,
  concision audit.
