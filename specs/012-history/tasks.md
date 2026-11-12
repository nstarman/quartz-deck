---
description: "Tasks for 012 history"
---

# Tasks: History — look back, save versions, compare, restore

**Input**: `specs/012-history/` (spec.md, plan.md, research.md,
data-model.md, contracts/, quickstart.md)

**Tests**: required (constitution VI, test-first). Each story's tests are
written first and seen failing. Every test repository sets
`commit.gpgsign false` and a local user (research R10).

**Organization**: by user story, in the spec's priority order. P1: US1 (look
back), US2 (save as git commit). P2: US3 (compare), US4 (restore). Paths are
relative to the repo root.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on unfinished tasks)
- **[Story]**: US1–US4

---

## Phase 1: Setup

- [X] T001 Add `src/history.rs` (`pub mod history;` in `src/lib.rs`), `src/ui/history.js` (empty module, served like the other `/ui/` modules), and a test helper `tests/common/mod.rs::git_repo(dir)` that runs `git init`, sets `user.name`, `user.email` and `commit.gpgsign false` locally, and returns a `git(args)` runner.

---

## Phase 2: Foundational (blocks every story)

- [X] T002 Write failing tests:
  - **`src/history.rs` units** (in a temp repository):
    - `repo()` finds the top level, the git dir and the deck's relative path from a subfolder, and returns `None` outside git;
    - `git()` maps a missing binary to `E_NO_GIT` and a failure to `E_GIT` with stderr;
    - `busy()` reports a merge (`MERGE_HEAD`), a rebase (`rebase-merge/`) and a detached head (research R3).
  - **`src/store.rs` units**: a journal entry records `by` (`person`/`agent`); old entries without it read as `agent`; `entries_since(t)` returns the entries newer than `t`, in order.
  - **`src/settings.rs`**: `history.commit` (Bool, false) and `history.pause` (Number, 30, 5–600) exist; the settings count in `src/settings.rs` and `tests/serve.rs` goes up by 2.
- [X] T003 Implement `git`, `Repo`, `repo`, `busy` in `src/history.rs`; errors `NoGit` (`E_NO_GIT`, hint "install git"), `NoRepo` (`E_NO_REPO`, hint "`qd history --init`"), `Git(String)` (`E_GIT`) in `src/error.rs`; the journal `by` and `entries_since` in `src/store.rs`, with `/api/op` edits journaled as `person` (`src/server.rs`); the two settings in `src/settings.rs`. T002 passes.

**Checkpoint**: git and the repository state are readable; nothing user-facing yet.

---

## Phase 3: User Story 1 — Look back through the deck's versions (P1) 🎯 MVP

**Independent Test**: spec US1; quickstart §1, §5.

- [X] T004 [P] [US1] Write failing tests:
  - **`tests/history.rs` (new) `history_lists_versions`**: a repository with a deck in `talks/` and five commits (two with `qd:` subjects, one a `git mv` rename). `qd history` returns all five, newest first, with author, time, message, `qd` and the path at each commit. An uncommitted change adds `uncommitted` first. `--limit 2` gives two and `"more": true`.
  - **`history_per_slide`**: `--slide one` lists only the commits where slide `one`'s text changed, matched by id across a reorder.
  - **`history_outside_git`**: `E_NO_REPO` with the hint. `qd history --init` creates the repository in the deck's folder and commits only the deck. A second `--init` is `E_INVALID_ARG`.
  - **`render_old_version`**: `history::render_version` on an old revision renders from the snapshot (an image changed since then shows the old file), runs no code (a cell that would write a marker file doesn't), and is reused from the cache on the second call.
- [X] T005 [US1] Implement `log`, `text_at`, `log_slide`, `init`, `render_version` (snapshot via `git archive --format=zip`, unzip, `render::render(..., preview: true)`) in `src/history.rs`. Add `qd history [--slide|--element] [--limit] [--skip] [--init]` in `src/cli.rs`, with the `history` shape in `qd schema`. Add `GET /api/history`, `POST /api/history/render` (token), `POST /api/history/init` (token) and `GET /history/<rev>/…` in `src/server.rs`. T004 passes.
- [X] T006 [P] [US1] Write a failing browser test `history_panel` in `tests/onslide.rs`:
  - View ▸ History lists the versions, with a qd badge on `qd:` commits;
  - selecting one shows the second preview with that version's title on slide 1;
  - the slide list marks the changed slides;
  - ← → step both previews;
  - outside git, the panel offers **Start a history here**, and clicking it creates the repository.
- [X] T007 [US1] Build the History panel and the second preview in `src/ui/history.js`. Add the `history` action (View, ⌥⌘H) in `src/ui/actions.json`, the layout in `src/ui/editor.css` / `index.html`, and the wiring in `src/ui/editor.js`. T006 passes.

**Checkpoint**: US1 works alone (MVP): any deck already in git can be looked through, in the editor and with `qd history`.

As built: the History panel is an `aside#history` beside the slide list, and
the version opens in `#then` beside `#preview`. `/api/history/render` also
returns `changed` (the current slides that differ, `history::changed_since`)
for the marks. The CSS rule that hid every `aside` while the properties panel
is closed now hides only `#panel`. Arrow keys move the current deck; the
version beside follows its slide (by id), so no keys of its own were needed.

---

## Phase 4: User Story 2 — Save as git commit (P1)

**Independent Test**: spec US2; quickstart §2.

- [X] T008 [P] [US2] Write failing tests:
  - **`src/history.rs` units for `message(entries)`**: ops in order with repeats folded (`edit ×3`); `(person)`, `(agent)`, `(person, agent)`; no entries gives `edit source (person)`; the subject is capped at 72 characters.
  - **`tests/history.rs` `commit_only_the_deck`**: with `other.txt` staged and then changed again, and the deck changed, `commit_deck` commits the deck alone and `git status` still shows `MM other.txt`. An untracked deck is added and committed. A deck equal to HEAD gives no commit.
  - **`commit_follows_git_config`**: a `pre-commit` hook that writes a marker runs on qd's commit. A hook that exits 1 makes the commit fail with its message, and the deck file keeps the edit.
  - **`no_commit_when_busy`**: during a conflicted merge, `commit_deck` returns `E_REPO_BUSY` and makes no commit.
  - **`cli_edit_commits_without_server`**: with `history.commit` on and no server, `qd title one X` succeeds with `"commit": {"rev"}`, and `git log -1` is `qd: title one (agent)`.
  - **`serve_commits_after_a_pause`**: with `history.pause` at 5 and `qd serve` running, three edits within 2 s and then a wait of 6 s give exactly one new commit holding all three. One more edit later gives a second commit. Stopping the server commits a pending edit.
- [X] T009 [US2] Implement `message`, `commit_deck` (R2, R3) in `src/history.rs`. Add the server's pending timer, commit on stop, and the SSE `history` event in `src/server.rs`. Add the CLI commit when there's no server, plus `"commit"` in the edit result, in `src/cli.rs`. T008 passes.
- [X] T010 [P] [US2] Write a failing browser test `history_commit_notice` in `tests/onslide.rs`: with the setting on, an edit shows "Saving a version…" until the pause ends, then "Saved as a version". A repository whose hook refuses shows a notice with the hook's message, and the edit is still in the file.
- [X] T011 [US2] In `src/ui/history.js` / `editor.js`: handle the notice (SSE `history`) and add the Settings entries (History section) and the panel's top line ("Saving a version in 12 s"). T010 passes.

**Checkpoint**: P1 complete.

---

## Phase 5: User Story 3 — See what changed (P2)

**Independent Test**: spec US3; quickstart §3.

- [X] T012 [P] [US3] Write failing tests:
  - **`src/history.rs` units for `diff(a, b)`**: matching by id, then unique title, then position; kinds `added`, `removed`, `moved` (same text, new place), `changed` (with a unified diff); unchanged slides are not listed.
  - **`tests/history.rs` `diff_versions`**: `qd diff HEAD~2` against the file, and `qd diff HEAD~2 HEAD`, list exactly the slides changed in the fixture's history, with positions.
  - **Browser test `history_compare`**: Compare lists the changed slides; opening one shows its source diff, with changed lines marked, and both renders.
- [X] T013 [US3] Implement `diff` in `src/history.rs`, `qd diff` in `src/cli.rs` and `GET /api/history/diff` in `src/server.rs`; build the compare view and the `history-compare` action in `src/ui/history.js` and `actions.json`. T012 passes.

As built: position matching pairs only slides without an explicit id, so a
slide whose id is gone is `removed` and a new id is `added`. `moved` means
same text and off the longest run of unchanged slides kept in order (Myers,
`similar`), so a slide inserted earlier doesn't make every later one
"moved", and in a swap where one slide also changed, only that one is
listed. `changed_since` (the US1 marks) now uses `diff`. Compare is the
`history-compare` action (View menu) while a version is open: the panel
lists the changed slides; choosing one shows its source diff under the list
and moves both previews to it.

---

## Phase 6: User Story 4 — Restore (P2)

**Independent Test**: spec US4; quickstart §4.

- [X] T014 [P] [US4] Write failing tests:
  - **`src/history.rs` units for `restore(current, old, what)`**:
    - a slide present in both is replaced, and no byte outside it changes (SC-003);
    - a deleted slide is inserted after its nearest earlier surviving neighbour;
    - an element with the same id is replaced;
    - an element whose slide is gone is refused with the hint;
    - the whole deck becomes the old text.
  - **`tests/history.rs` `restore_slide`**:
    - `qd restore HEAD~1 --slide one` changes only that slide;
    - `qd undo` returns the file byte-for-byte;
    - a stale `--expect` is `E_CONFLICT`;
    - with the setting on, `git log` gains a commit and loses none (SC-004).
  - **Browser test `history_restore`**:
    - with a version open, the slide's right-click menu has **Restore slide from this version**, and using it changes the slide;
    - ⌘Z takes it back;
    - **Restore this version…** asks first.
- [X] T015 [US4] Implement `restore` in `src/history.rs` and `qd restore` in `src/cli.rs` (in `API_OPS`, op name `restore … from <short>`). Add the menu entries and actions `history-restore`, `history-restore-slide` and `history-restore-element` in `src/ui/history.js` and `actions.json`. T014 passes.

As built: restore pairs slides with the same one-to-one matching as `diff`
(`history::pairs`). A restored slide keeps the current gap to the next
slide, so no byte outside it changes. An element is the one with the same
explicit id, or else at the same place in the paired slide. The edit's `op`
is the whole description (`restore slide one from a1b2c3d`) with an empty
`target`, so the commit message reads the same. In the editor, **Restore
slide from this version** (slide menus) and **Restore from this version**
(element menus) show only while a version is open; **Restore this
version…** (View) asks first. A slide that is gone now is restored with
`qd restore REV --slide REF`; the editor has no menu for it yet.

---

## Phase 7: Polish & cross-cutting

- [X] T016 [P] Docs:
  - README: a History section;
  - the qd skill: `qd history`/`diff`/`restore`, the setting, and the errors `E_NO_REPO`, `E_NO_GIT`, `E_GIT`, `E_REPO_BUSY`;
  - `specs/ROADMAP.md`: 012's status;
  - the 007 action inventory: "Added by 012".
- [X] T017 Run quickstart §1–§5 in the browser pane and record it in `specs/012-history/checklists/quickstart-run.md`.
- [X] T018 Final constitution re-check in `plan.md`; fmt, clippy, `cargo test --no-fail-fast` and `node --test vscode/test/*.test.js` green.
- [X] T019 Concision audit (`/pr-audit`), the last task before each PR is marked ready.

---

## Dependencies & Execution Order

- **Setup (T001) → Foundational (T002–T003)** → every story.
- **US1 (T004–T007)** after Foundational.
- **US2 (T008–T011)** after Foundational. It doesn't need US1's panel except for T011's top line; T008–T009 can go alongside US1.
- **US3 (T012–T013)** after US1 (versions and the second preview).
- **US4 (T014–T015)** after US3 (slide matching).
- **Polish** after the stories it documents.

Within each story: tests first (seen failing), then `history.rs` and the
CLI, then the server, then the editor.

## Parallel opportunities

- T004 (US1 tests) ∥ T008 (US2 tests): different test functions.
- T005 (log, render) ∥ T009 (commit, timer): different functions; both touch `cli.rs` and `server.rs`, so merge carefully.
- T012 ∥ T014 test writing.

## Implementation strategy

1. **MVP:** Setup, Foundational, US1 (look back, `qd history`, the panel). One PR.
2. **P1 rest:** US2 (save as git commit). One PR.
3. **P2:** US3 + US4 (compare, restore). One PR, then the polish PR.

Each PR passes the concision audit (T019) before it is marked ready and is
merged only after the person approves it.
