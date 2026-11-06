---
description: "Task list for 007-app-shell"
---

# Tasks: App shell — menus, settings, shortcuts, consistent behaviour

**Input**: Design documents from `specs/007-app-shell/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/http.md, contracts/cli.md, contracts/ui.md, quickstart.md

**Tests**: Required (constitution VI). Write each test task first and see it
fail before the implementation task after it. Browser tests live in
`tests/onslide.rs` (headless Chrome); server tests in `tests/serve.rs`; CLI
tests in `tests/cli.rs`; extension tests run with `node --test`.

**Organization**: P1: US1 (menus). P2: US2 (settings), US3 (shortcut
reference). P3: US4 (consistent behaviour), US5 (VS Code). Foundational work
(the action list and the key dispatcher) blocks US1, US3, US4 and US5.
Paths relative to repo root.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on unfinished tasks)
- **[Story]**: US1–US5

---

## Phase 1: Setup

- [X] T001 Inventory today's actions and keys in `specs/007-app-shell/checklists/action-inventory.md`. List every header button, right-click item (element, slide, empty area), property-toolbar control, formatting-toolbar button and source-pane bar button. Also list every key the editor handles: `onKey`, `sessionKey`, the toolbar's typing keys, the source pane, the splitter and the slide list. Give each a proposed action id, menu and key. This is the baseline for SC-001.
- [X] T002 [P] Add `store::config_root()` next to `cache_root()` in `src/store.rs`: `$QD_CONFIG_DIR`, else `~/Library/Application Support/quartz-deck` on macOS, else `$XDG_CONFIG_HOME/quartz-deck` or `~/.config/quartz-deck`. Add a unit test for the env override.

---

## Phase 2: Foundational — one action list, one key dispatcher (blocks US1, US3–US5)

**Purpose**: research R1. Everything that lists or handles an action reads `src/ui/actions.json`.

- [X] T003 [P] Write failing Rust tests in a new `tests/actions.rs` over the embedded list (data-model.md "Action"):
  - ids are unique;
  - every key parses (`Mod`/`Shift`/`Alt`/`Ctrl` + key);
  - no two actions share a key where their `where` overlaps;
  - no `runs_code` action fires in `text`;
  - every id in `checklists/action-inventory.md` is in the list (SC-001).
- [X] T004 Write `src/ui/actions.json` from the inventory: menus and order as in contracts/ui.md, keys, `where`, `toggle`, `runs_code`, `qd`. Embed it (`include_str!`). Serve `GET /ui/actions.js` as `export default <json>;` in `src/server.rs`. T003 passes.
- [X] T005 [P] Write failing checks.
  - **Static (Rust, `tests/actions.rs`):** scanning `src/ui/*.js` finds no key comparisons (`e.key ===`, `key ===`, `e.code ===`) outside `src/ui/keys.js`. An allowlist covers text-editing internals (menu.js arrow keys, list typing in toolbar.js), each with a one-line reason.
  - **Browser (`tests/onslide.rs`):** `window.qdActions()` reports no action without a handler, and pressing each listed key in its place runs that action (a spy wraps the handlers).
- [X] T006 Implement `src/ui/keys.js`, one keydown dispatcher built from the list:
  - ⌘ on macOS and Ctrl elsewhere;
  - the place comes from focus (slide list, slide, element, text edit, source pane);
  - printable keys never fire while typing;
  - `runs_code` actions only from a real key event.

  Register handlers by id in `src/ui/editor.js` and `src/ui/actions.js`. Remove the scattered key checks. T005 passes, and all existing browser tests stay green.

**Checkpoint**: every action and key comes from one list; behaviour unchanged.

---

## Phase 3: User Story 1 — Every action is in a menu (P1) 🎯 MVP

**Goal**: a menu bar holding every action, keyboard- and screen-reader-operable.

**Independent Test**: spec US1; quickstart §1.

- [X] T007 [US1] Write failing browser tests in `tests/onslide.rs` `menu_bar_runs_every_action`:
  - **Menus and items:** File, Edit, Insert, Slide, View, Run and Help are shown. Through the menus alone, run add slide after, duplicate slide, insert a paragraph, bold (in a text edit), toggle the source pane, Run all (on the run deck), Render and Export. Paste with nothing copied is shown disabled. With an element selected, the Edit/Insert/Slide items act on it, and its right-click menu shows the same items.
  - **Keyboard only:** F10 focuses the bar; arrows, Enter, Escape and type-ahead work; closing returns focus to where it was.
  - **Accessibility:** roles are `menubar`/`menu`/`menuitem`/`menuitemcheckbox`, with `aria-disabled`, `aria-checked` and `aria-keyshortcuts`.
  - **Equivalence:** each action run from the menu leaves the same file and one undo step, the same as its button.
- [X] T008 [US1] Implement `src/ui/menubar.js` (over `src/ui/menu.js`), the `index.html` container and the `editor.css` styles. Make the right-click menus in `src/ui/actions.js` views of the list. Keep the header buttons as quick access, with tooltips showing their keys from the list. T007 passes.

**Checkpoint**: MVP. Nothing is reachable only by right-click or a hidden key.

---

## Phase 4: User Story 2 — Settings that stick (P2)

**Goal**: per-user settings, applied live, the same in every deck, server and VS Code.

**Independent Test**: spec US2; quickstart §2.

- [X] T009 [P] [US2] Write failing unit tests in a new `src/settings.rs`:
  - **Definitions:** the six of data-model.md, with defaults.
  - **Loading:** a missing file gives the defaults; an invalid value gives its default plus one notice; unknown keys survive a save.
  - **Saving:** atomic, mode 0600.
  - **Changes:** `set` validates (`E_INVALID_SETTING` names the key and the allowed values); `reset` keeps unknown keys.
- [X] T010 [US2] Implement `src/settings.rs` with `QdError::InvalidSetting` in `src/error.rs`. T009 passes.
- [X] T011 [P] [US2] Write failing tests.
  - **`tests/serve.rs` `settings_over_http`:** GET returns `{values, defs, notices}`. POST needs the token (403 without) and JSON; a bad value gives 400; a good one gives 200, and an SSE `settings` event reaches another `/events` stream. `qd settings set` from the CLI also gives the event (file watch).
  - **`tests/cli.rs`:** `qd settings`, `qd settings set theme dark` and `qd settings reset` (JSON per contracts/cli.md, using `QD_CONFIG_DIR`).
- [X] T012 [US2] Implement it. In `src/server.rs`: the routes, the event, and watching `<config>/settings.json`. In `src/cli.rs`: `qd settings [set KEY VALUE | reset]`, plus `qd schema`. T011 passes.
- [X] T013 [US2] Write failing browser tests `settings_apply_live_and_stick`:
  - ⌘, opens Settings, and each setting applies at once: source pane right, theme dark, `source.save` explicit.
  - Restart the server on another deck: the settings hold.
  - The pane width and open state survive a port change (the `ui.*` keys).
  - Reset to defaults asks first.
  - Tests use `QD_CONFIG_DIR` in a temp folder.
- [X] T014 [US2] Implement `src/ui/settings.js` (the window and applying settings) and wire it into `src/ui/editor.js`/`editor.css`:
  - `source.side` by a class on `main`;
  - `theme` through `data-theme`, where "system" removes it;
  - `source.save` in `src/ui/pane.js`;
  - `confirm.slide_delete` in the slide-rm action;
  - `hints.shortcuts` in tooltips.

  Replace `pref()`/`localStorage` with `ui.*` settings. T013 passes.

**Checkpoint**: settings stick everywhere.

---

## Phase 5: User Story 3 — One list of shortcuts (P2)

**Goal**: a searchable reference from the list; `qd actions` for agents.

**Independent Test**: spec US3; quickstart §3.

- [X] T015 [P] [US3] Write failing tests.
  - **Browser `shortcut_reference`:** ? opens it (not while typing), Help → Keyboard shortcuts too; it has a section per menu and lists every action with a key; search filters it; Escape closes it and returns focus.
  - **`tests/cli.rs`:** `qd actions` returns the list per contracts/cli.md, the same ids as `/ui/actions.js`.
- [X] T016 [US3] Implement the reference dialog in `src/ui/menubar.js`, `qd actions` in `src/cli.rs` (plus `qd schema`), and a skill section in `skills/quartz-deck/SKILL.md` ("`qd actions` lists what the editor can do and its keys"). T015 passes.

---

## Phase 6: User Story 4 — The same rules everywhere (P3)

**Goal**: Escape/Enter, clipboard for slides, undo for every surface, visible focus.

**Independent Test**: spec US4; quickstart §4.

- [X] T017 [P] [US4] Write failing tests for `paste`.
  - **`src/ops.rs` units:** one or more slides of markdown are inserted after/before a slide, or at the end. Clashing slide and block ids are renewed, reusing `renew_id`. It is one patch. Text with no slide gives `E_PARSE`.
  - **`tests/cli.rs`:** `qd paste --after S --md -`.
- [X] T018 [US4] Implement `ops::paste`, `qd paste` in `src/cli.rs` (+ schema, skill) and the `paste` op in `/api/op`. T017 passes.
- [X] T019 [US4] Write failing browser tests `same_rules_everywhere`:
  - **Escape:** from a menu open over a text edit on a selected element, each Escape backs out one level, and the edit is kept, not discarded. Enter on a selected element starts editing.
  - **Slides on the clipboard:** select a slide in the slide list, copy it, then paste it into a second server's deck. It arrives with fresh ids, and one undo removes it.
  - **Text from elsewhere:** system-clipboard text that is not from qd pastes as a paragraph with `<` escaped.
  - **Undo:** one edit each from a menu, a shortcut, the toolbar, a drag, on-slide typing and the source pane; each is one undo step (research R9).
  - **Focus:** tabbing to a control, a slide or an element shows a focus outline in light, dark and hc.
- [X] T020 [US4] Implement it in `src/ui/editor.js`/`editor.css`:
  - Escape saves in `sessionKey`; the status text reads "Esc or click elsewhere saves"; the source popup's Cancel button still discards.
  - Slide selection and cut/copy/paste in the slide list, using the `paste` op for slides.
  - Outside text escaped on paste.
  - Focus styles.
  - Fix any undo gap T019 finds.

  T019 passes, and the older tests that expected Escape to cancel are updated.

---

## Phase 7: User Story 5 — Inside VS Code (P3)

**Goal**: a folded menu and no stolen VS Code keys in the panel.

**Independent Test**: spec US5; quickstart §5.

- [X] T021 [P] [US5] Write `vscode/default-keys.json`: VS Code's default macOS bindings that apply while a webview has focus, from the public default-keybindings reference, minus the edit keys a webview receives (research R7). Write a failing test in `vscode/test/keys.test.js`: no action's effective key in the panel (its `vscode` key, else `mac`) is on that list.
- [X] T022 [US5] Give the clashing actions `vscode` keys or `null` in `src/ui/actions.json`. Make `keys.js` use them when `?host=vscode`. Fold the bar into a ☰ menu in framed mode and below the width where it fits. Have the reference list the panel's keys there. Extend `framed_editor_takes_theme_and_messages`: no menu bar, a ☰ button with the same menus, and ⌘B does nothing in the panel. T021 passes.

---

## Phase 8: Polish & cross-cutting

- [X] T023 [P] Security: add `POST /api/settings` to `another_origin_cannot_run_code_or_edit`; the settings are unchanged after the attack. Record it in `specs/007-app-shell/checklists/security.md`, along with the file's mode 0600.
- [X] T024 [P] Docs: a README section on menus, settings and shortcuts; the skill (`qd actions`, `qd settings`, `qd paste`); `vscode/README.md` (the ☰ menu and keys in the panel); the `specs/ROADMAP.md` status.
- [X] T025 Performance (SC-008): a browser test opens each menu and the reference on a 30-slide deck in under 100 ms.
- [X] T026 Run the quickstart §1–§4 in the browser and record it in `checklists/quickstart-run.md`. §5 (VS Code) is done by the person, or left as a follow-up like 006's T022.
- [X] T027 Final constitution re-check in `plan.md`; fmt, clippy, `cargo test` and `node --test vscode/test/*.test.js` all green.
- [X] T028 Concision audit (`/pr-audit`), the last task before the PR is marked ready.

---

## Dependencies & Execution Order

- **Setup (T001–T002)** → **Foundational (T003–T006)**. T002 is needed only by US2.
- **US1 (T007–T008)** needs the foundation. It is the MVP.
- **US2 (T009–T014)** needs only T002. It can run alongside US1, since it touches different files apart from `editor.js` wiring at the end.
- **US3 (T015–T016)** needs the foundation and US1's `menubar.js`.
- **US4 (T017–T020)**: T017–T018 (`paste`) can start any time; T019–T020 need the foundation.
- **US5 (T021–T022)** needs the foundation and US1.
- **Polish** comes last. T028 is the very last task.

### Parallel opportunities

- T002 ∥ T003 ∥ T005, and T009 ∥ T011 ∥ T017, because they are in different files.
- US2's Rust side (T009–T012) ∥ US1 (T007–T008).

## Implementation Strategy

1. **MVP:** Setup + Foundational + US1, then a PR. Every action is in a menu,
   driven by one list.
2. **Second PR:** US2 + US3, settings and the shortcut reference.
3. **Third PR:** US4 + US5 + Polish, the consistency rules and VS Code, with
   the audit.

Each PR passes the concision audit before it is marked ready.
