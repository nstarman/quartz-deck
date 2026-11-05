---
description: "Task list for 005-shared-focus"
---

# Tasks: Shared Focus

**Input**: Design documents from `specs/005-shared-focus/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/cli.md, contracts/http.md, quickstart.md

**Tests**: Required (constitution VI). Write each test task first and see it
fail before the implementation task after it. Browser tests need Chrome and
Quarto and skip with a message without them; CI has both.

**Organization**: P1: US1 (reading the focus). P2: US2 (pointing the editor).
Paths relative to repo root.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on unfinished tasks)
- **[Story]**: US1, US2

---

## Phase 1: Setup

- [X] T001 Add the focus result shapes to `qd schema`: `focus` entry in `output_of` in `src/cli.rs` (`{"open",…}` read, `{"ok","result","ref"}` set) per contracts/cli.md

---

## Phase 2: Foundational (blocking both stories)

**Purpose**: the focus snapshot type, tab tracking on the event stream, and the editor's shared focus object.

- [X] T002 [P] Write failing unit tests in `src/focus.rs`: a snapshot (data-model.md) parses; `selection.text` is clamped to 2,000 chars with `truncated: true`; more than 50 element refs are cut; empty `tab`/`version` is refused (`E_INVALID_ARG`); on read, refs that no longer resolve in a given deck are dropped, `slide: null` kept, and `deck_version` reported next to `version`
- [X] T003 Implement `src/focus.rs` (`Snapshot` with serde, `Snapshot::clamp`, `report(&self, deck, age) -> Value`), register in `src/lib.rs` — T002 passes
- [X] T004 [P] Write failing HTTP tests in `tests/serve.rs` `focus_over_http`: `GET /api/focus` and `POST /api/focus` without the token, with a wrong token, or with a foreign Origin → 403; with no `/events?tab=` stream open → `{"open": false}`; with a stream open but nothing posted → `open: true, age: null` + hint; after a post → the snapshot with `age` ≥ 0; after the stream closes → `open: false`; two tabs posting → the later one is reported; a post naming a deleted element → that ref dropped
- [X] T005 Server side in `src/server.rs`: `App.focus` (open tab ids + latest snapshot with arrival `Instant`); `/events` takes `?tab=` and registers/unregisters the tab with a drop guard on the stream; `POST /api/focus` (authorized, clamp, keep if tab open) → 204; `GET /api/focus` (authorized) → `Snapshot::report` against the current deck — T004 passes
- [X] T006 [P] Editor focus object `src/ui/focus.js`: tab id (`crypto.randomUUID()`), `set(partial)` merges and notifies listeners, posts the snapshot with the session token debounced ~100 ms (skipped, with one notice, when the page has no token), re-posts on window `focus` / `visibilitychange`; `editing` flag with a 3 s quiet timer; export `get`, `set`, `on`, `tabId`. `src/ui/run.js` exports its token (or `focus.js` reads it the same way) — no second copy of the fragment logic
- [X] T007 Open the event stream as `/events?tab=<focus.tabId>` in `src/ui/editor.js`; add `focus.js` to `UI_MODULES` in `src/server.rs`

**Checkpoint**: the server knows which tabs are open and stores what they report.

---

## Phase 3: User Story 1 — An agent knows what "this" means (P1) 🎯 MVP

**Goal**: `qd focus` returns the slide, elements, selected text (slide or source), source line, versions, age; `{"open": false}` without an editor.

**Independent Test**: spec US1 Independent Test; quickstart §1.

- [X] T008 [P] [US1] Write failing browser test `focus_follows_the_person` in `tests/onslide.rs`: go to a slide → `qd focus` slide matches, `elements: []`; click a paragraph → that element; select two words while editing it → `selection {text, where: "slide"}`; open Source, select lines spanning two blocks → both elements, `where: "source"`, `line` set; each visible to `qd focus` within 0.5 s (SC-001, measured); `qd title` edit → `version` ≠ `deck_version`; `qd stop` → `{"open": false}`
- [X] T009 [P] [US1] Write failing CLI tests in `tests/serve.rs` (or `tests/run_cli.rs` if it needs no browser): `qd focus` with no server → `{"open": false}` exit 0; with a server and a posted snapshot → the snapshot's fields; 100 reads leave the deck file byte-identical (SC-005)
- [X] T010 [US1] `qd focus` in `src/cli.rs`: `Cmd::Focus { set: Option<String> }`; read path = `server::running` → `to_server` GET `api/focus` (extend `to_server` with the method), no server → `{"open": false}` — T009 passes
- [X] T011 [US1] Slide view → focus in `src/ui/editor.js`: slide navigation (`state.anchor` changes) → `focus.set({slide, elements: []})`; element selection (`state.sel`) → `elements: [ref]`; text selection inside an open edit (or the preview frame) → `selection {where: "slide"}`; edit session open/closed → `editing`
- [X] T012 [US1] Source pane → focus in `src/ui/pane.js`: caret line → `line` and the block under it (blocks' `lines` from the editor's slide data); textarea selection → `selection {where: "source"}` and every block whose lines intersect; keystrokes → `editing` (3 s quiet). Rewire 004's `onCaretLine` slide sync as a `focus.on` listener (FR-012) — T008 passes
- [X] T013 [US1] Skill `skills/quartz-deck/SKILL.md`: a "this / here / the selected …" rule (run `qd focus` first; act when open and fresh; ask when closed/empty/old) and a table row; `qd schema` lists `focus`; skill sync test passes

**Checkpoint**: MVP — agents can resolve "this".

---

## Phase 4: User Story 2 — An agent points the person at a slide or element (P2)

**Goal**: `qd focus --set <ref>` moves the most recent tab and flashes the element; deferred with a "Go to …" offer while editing.

**Independent Test**: spec US2 Independent Test; quickstart §2.

- [X] T014 [P] [US2] Write failing HTTP tests in `tests/serve.rs`: `POST /api/focus/set` needs the token (403 otherwise); bad ref → 404 `E_NOT_FOUND`; no open tab → `no_editor`; open tab not editing → `moved` and a `point {tab, anchor, ref}` event on that tab's stream; tab reported `editing: true` → `deferred`
- [X] T015 [P] [US2] Write failing browser test `pointing_moves_or_offers` in `tests/onslide.rs`: `qd focus --set results/1` → the editor shows the slide within 1 s (SC-002) and the element has the flash class; with the source pane open its lines are revealed; while typing in a paragraph, 20 × `--set` elsewhere → every result `deferred`, the view never moves, the typed text arrives intact (SC-003), and a "Go to …" offer is shown; clicking it goes there
- [X] T016 [US2] Server `POST /api/focus/set` in `src/server.rs`: authorize, resolve the ref (`d.element` / `d.slide`, preview anchor via `render::preview_id`), target = latest tab, emit `Ev::Run("point", …)` (or a new `Ev::Point`), answer `moved`/`deferred`/`no_editor` from the latest snapshot's `editing` — T014 passes
- [X] T017 [US2] `qd focus --set <ref>` in `src/cli.rs` via `to_server` POST; no server → `{"ok": true, "result": "no_editor"}`
- [X] T018 [US2] Editor `point` handling in `src/ui/focus.js` + `src/ui/editor.js`: ignore other tabs' events; when not editing go to the anchor, select/flash the element (CSS class for ~1.5 s), reveal its source lines; when editing show a dismissable "Go to <title>" offer (`role="status"`, keyboard reachable) that navigates on click; styles in `src/ui/editor.css` — T015 passes

**Checkpoint**: both directions work.

---

## Phase 5: Polish & cross-cutting

- [X] T019 [P] Extend `another_origin_cannot_run_code_or_edit` in `tests/onslide.rs` with `GET /api/focus`, `POST /api/focus`, `POST /api/focus/set`: unreadable/unsettable from another origin (SC-006); note in `specs/005-shared-focus/checklists/security.md`
- [X] T020 [P] README: "Shared focus" lines in the editor section; `specs/ROADMAP.md` status
- [ ] T021 Quickstart §1–§4 with the release build in the browser pane; record timings (SC-001, SC-002) in `checklists/quickstart-run.md`
- [ ] T022 Agent eval (SC-004): 10 fresh agents, the person has a sentence selected in the editor, task "make this shorter"; grade: only that element changed, no "which one?" question, no usage errors; record in `checklists/agent-eval.md`
- [ ] T023 Final constitution re-check in `plan.md`; fmt, clippy, full suite green
- [ ] T024 Concision audit (`/pr-audit`), last task before the PR is marked ready

---

## Dependencies & Execution Order

- Phase 1 → Phase 2 → US1 → US2 → Polish.
- T002 → T003; T004 → T005 (needs T003); T006/T007 need nothing in Rust.
- US1: T008, T009 (tests, parallel) → T010, T011, T012 → T013.
- US2 needs T005 (tab tracking, latest snapshot) and T011/T012 (`editing`):
  T014, T015 (tests, parallel) → T016 → T017 → T018.
- T019–T022 after both stories; T023 → T024 last.

## Parallel Examples

```text
Phase 2:  T002 (src/focus.rs tests) ∥ T004 (tests/serve.rs) ∥ T006 (src/ui/focus.js)
US1:      T008 (browser test) ∥ T009 (CLI test)
US2:      T014 (HTTP test) ∥ T015 (browser test)
Polish:   T019 ∥ T020
```

## Implementation Strategy

1. MVP = Phases 1–3 (agents can read the focus) → PR.
2. US2 (pointing) → PR.
3. Polish → PR. Each PR passes the concision audit before it is marked ready.
