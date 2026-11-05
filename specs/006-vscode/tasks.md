---
description: "Task list for 006-vscode"
---

# Tasks: VS Code Integration

**Input**: Design documents from `specs/006-vscode/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/http.md, contracts/cli.md, contracts/extension.md, quickstart.md

**Tests**: Required (constitution VI). Write each test task first and see it
fail before the implementation task after it. Rust tests need Quarto for the
server cases; extension tests run with `node --test` (no npm packages);
VS Code itself is exercised by the quickstart run.

**Organization**: P1: US1 (panel) + US2 (same deck, no lost changes) — they
ship together. P2: US3 (cursor ↔ slide). P3: US4 (run, render, export).
Paths relative to repo root.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on unfinished tasks)
- **[Story]**: US1–US4

---

## Phase 1: Setup

- [ ] T001 Create `vscode/` with `package.json` (name `quartz-deck`, publisher `nstarman`, `license` per `LICENSE`, `engines.vscode`, `extensionKind: ["workspace"]`, `activationEvents: ["onLanguage:quarto", "workspaceContains:**/*.qmd"]`, `main: "extension.js"`, the commands and setting of contracts/extension.md, an editor-title menu entry on `.qmd`), an empty `extension.js` exporting `activate`/`deactivate`, `vscode/README.md` (build: `npx @vscode/vsce package`; install: `code --install-extension`), and `vscode/.vscodeignore` (exclude `test/`)
- [ ] T002 [P] CI: add `actions/setup-node` and a step `node --test vscode/test/` to `.github/workflows/ci.yml`

---

## Phase 2: Foundational — the buffer overlay (blocks US1/US2)

**Purpose**: while VS Code is attached, every qd read/write goes to the buffer (research R3–R5).

- [X] T003 [P] Write failing unit tests in `src/store.rs`:
  - with `<cache>/buffer.qmd` present, `Store::read` returns its text and `write` replaces it (CAS on its version, undo journal entry), leaving the `.qmd` untouched;
  - without it, behaviour is unchanged;
  - `undo`/`redo` work on the overlay;
  - `attach(text)` / `detach()` create and remove it.
- [X] T004 Implement the overlay in `src/store.rs`: `fn backing(&self) -> PathBuf` (overlay if present, else the deck) used by `read_text`, `replace`, `undo`/`redo`; `pub fn attach(&self, text)`, `pub fn detach(&self)`, `pub fn attached(&self) -> bool` — T003 passes
- [X] T005 [P] Write failing HTTP tests in `tests/serve.rs` `buffer_overlay_over_http`:
  - all three endpoints need the token (403 otherwise);
  - `open` → `{version}` and `session.json` `"buffer": true`;
  - `/api/source` serves the overlay;
  - `POST /api/buffer {base, text}` with the right base → 200 and the preview re-renders; with a stale base → 409 `{version, text}`;
  - a panel op (`/api/op` title) → a `buffer_edit {base, version, edits}` event whose edits turn the old text into the new, and the `.qmd` on disk is unchanged;
  - a CLI `qd edit` while attached → also a `buffer_edit`, the disk unchanged;
  - `qd undo` → a `buffer_edit` back;
  - `close` → overlay gone, `"buffer"` cleared;
  - `qd stop` also detaches.
- [X] T006 Server in `src/server.rs`:
  - `POST /api/buffer/open` (attach, set session flag);
  - `POST /api/buffer` (CAS under the store lock: write, or 409 with the current overlay);
  - `POST /api/buffer/close`;
  - watch the overlay file. On a change not made by `/api/buffer`, diff old → new (`similar`, byte offsets) and emit `buffer_edit`; re-render either way;
  - detach on shutdown.

  T005 passes.

**Checkpoint**: qd can treat a VS Code buffer as the deck.

---

## Phase 3: User Story 1 + 2 — Panel beside the text; one deck, no lost changes (P1) 🎯 MVP

**Goal**: "Open in quartz-deck" shows the editor beside the `.qmd`; edits flow both ways as unsaved, undoable buffer edits; nothing is ever lost.

**Independent Test**: spec US1 and US2 Independent Tests; quickstart §1–§2.

- [X] T007 [P] [US2] Write failing `node --test` tests in `vscode/test/sync.test.js` for `vscode/sync.js`:
  - `applyEdits(text, edits)` reproduces the server's new text;
  - `merge(base, mine, theirs)` keeps non-overlapping changes from both and reports overlap as a conflict with both versions;
  - the sync state machine: buffer change → post with `base`; a 409 → merge → re-post; `buffer_edit` with a matching base → one `WorkspaceEdit`; a mismatched base → full resync;
  - the extension's own echoes are ignored.

  All against a fake `vscode` / fetch.
- [X] T008 [US2] Implement `vscode/sync.js` (pure: no `vscode` import; the caller passes `applyWorkspaceEdit`, `post`, `onConflict`) — T007 passes
- [X] T009 [P] [US1] Write failing tests in `vscode/test/panel.test.js` for `vscode/panel.js`:
  - the webview HTML is one full-size iframe of the editor link plus `?host=vscode&theme=…[&run=external]`, with a CSP allowing only that origin;
  - messages from the iframe other than `reveal`/`ready` are dropped;
  - theme kinds map to `light`/`dark`/`hc`.
- [X] T010 [US1] Implement `vscode/panel.js` — T009 passes
- [X] T011 [US1] Implement `vscode/extension.js`:
  - `quartz-deck.open`: find `qd` (setting or PATH); a missing qd or Quarto gives an error message naming the install;
  - spawn `qd serve --port 0` for the active `.qmd` and read the first JSON line;
  - `asExternalUri` for the link;
  - one panel per deck (reveal if open);
  - `POST /api/buffer/open` with the document text;
  - wire `sync.js` to `workspace.onDidChangeTextDocument` and the server's event stream (`buffer_edit`);
  - the conflict choice via `showWarningMessage` (Keep mine / Take theirs / Compare → `vscode.diff`);
  - dispose → `qd stop`;
  - `WebviewPanelSerializer` to restore;
  - follow `onDidChangeActiveColorTheme`.
- [X] T012 [P] [US1] Editor framed mode in `src/ui/editor.js` + `src/ui/editor.css`:
  - `?theme=light|dark|hc` overrides the colour scheme (CSS variables, including a high-contrast set);
  - with `?host=vscode`, accept `postMessage` only from `window.parent`, and only the `theme` / `follow` / `run` types;
  - post `ready` when loaded;
  - `?run=external` hides the run bar and gutter (spec 004 UI).
- [X] T013 [US1] Browser test in `tests/onslide.rs` `framed_editor_takes_theme_and_messages`: the editor page inside a test parent page (iframe, as the webview does) gets dark colours with `?theme=dark`, ignores a message from a non-parent, hides Run with `run=external`, posts `ready` — T012 passes
- [X] T014 [US2] VS Code run (quickstart §1–§2) by hand in the real app; record SC-001/SC-002/SC-003 in `specs/006-vscode/checklists/vscode-run.md`; fix what it finds

**Checkpoint**: MVP — the panel in VS Code, edits both ways, nothing lost.

---

## Phase 4: User Story 3 — Cursor and slide follow each other (P2)

**Goal**: the VS Code cursor feeds spec 005's focus and the panel follows; selecting in the panel reveals the source.

**Independent Test**: spec US3 Independent Test; quickstart §3.

- [X] T015 [P] [US3] Write failing tests:
  - `vscode/test/focus.test.js` for `vscode/focus.js`: a cursor/selection gives a focus snapshot with `where: "text"`, the line, the selected text clamped to 2,000 chars, refs from blocks' line ranges, debounced; a `reveal {lines}` message gives a `revealRange` without focus;
  - `tests/serve.rs`: `/api/focus` accepts `where: "text"`;
  - `src/focus.rs` unit test updated.
- [X] T016 [US3] Implement:
  - `src/focus.rs`: allow `"text"`;
  - `vscode/focus.js`: snapshot from `onDidChangeTextEditorSelection`; the extension posts it with tab `vscode:<deck>` and sends `follow {line}` to the iframe;
  - `src/ui/editor.js`: on `follow`, show the line's slide (the 004 line → slide mapping) without counting it as the person's focus; post `reveal {lines}` to the parent on element selection when framed;
  - `vscode/extension.js`: on `point` (spec 005), also reveal the source.

  T015 passes.
- [X] T017 [US3] Browser test addition to `framed_editor_takes_theme_and_messages`: `follow {line}` shows that slide; clicking an element posts `reveal` with its lines to the parent

**Checkpoint**: the two views follow each other; `qd focus` sees the VS Code cursor.

*As built*: simpler than planned. The panel's editor already maps a source
caret to slide and elements (its source pane does this), so VS Code's cursor is
sent to it as a `follow {line, first, last, text}` message and goes down the
same path. The editor tab reports it with `where: "source"`, so there is no
`vscode/focus.js`, no `vscode:<deck>` tab and no server change. A panel selection
or an agent's `qd focus --set` posts `reveal {lines}`; the extension moves the
cursor there without taking keyboard focus.

---

## Phase 5: User Story 4 — Run, render, export (P3)

- [X] T018 [P] [US4] Write failing tests in `vscode/test/commands.test.js`: render/export/exportPdf/checkOffline spawn `qd` with the right args in the deck's folder and turn its JSON (or its stderr error) into a success or error message; `quarto.quarto` installed → `run=external`
- [X] T019 [US4] Implement the palette commands and the Quarto-extension detection in `vscode/extension.js` (output channel `quartz-deck`) — T018 passes

---

## Phase 6: Polish & cross-cutting

- [ ] T020 [P] Security (SC-006): extend `tests/onslide.rs` `another_origin_cannot_run_code_or_edit` with `/api/buffer/open`, `/api/buffer`, `/api/buffer/close` (nothing attached, nothing changed); `framed_editor…` already checks the non-parent message; record in `specs/006-vscode/checklists/security.md`
- [ ] T021 [P] README: "VS Code" section (build/install the `.vsix`, what syncs, unsaved-until-save); skill note (with VS Code attached, qd edits land in the buffer); `specs/ROADMAP.md` status
- [ ] T022 VS Code quickstart §3–§5 in the real app incl. Remote-SSH (SC-004, SC-005); record in `checklists/vscode-run.md`
- [ ] T023 Final constitution re-check in `plan.md`; fmt, clippy, `cargo test`, `node --test vscode/test` green
- [ ] T024 Concision audit (`/pr-audit`), last task before the PR is marked ready

---

## Dependencies & Execution Order

- Phase 1 → Phase 2 (overlay) → US1+US2 → US3 → US4 → Polish.
- T003 → T004; T005 → T006 (needs T004).
- US1/US2:
  - T007 → T008 and T009 → T010 can run in parallel;
  - T011 needs T008, T010 and T006;
  - T012 → T013;
  - T014 last.
- US3 needs US1/US2 (panel, messages): T015 → T016 → T017.
- US4 needs T011: T018 → T019.
- Polish after the stories; T023 → T024 last.

## Parallel Examples

```text
Phase 2: T003 (store tests) ∥ T005 (HTTP tests)
US1/US2: T007 (sync tests) ∥ T009 (panel tests) ∥ T012 (editor framed mode)
Polish:  T020 ∥ T021
```

## Implementation Strategy

1. MVP = Phases 1–3 (panel + overlay + sync) → PR.
2. US3 (cursor ↔ slide) → PR. 3. US4 → PR (small; may join US3's).
4. Polish → PR. Each PR passes the concision audit before it is marked ready.
