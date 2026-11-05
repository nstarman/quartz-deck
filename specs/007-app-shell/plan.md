# Implementation Plan: App shell — menus, settings, shortcuts, consistent behaviour

**Branch**: `007-app-shell` | **Date**: 2026-11-05 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/007-app-shell/spec.md`

## Summary

One JSON list of actions, `src/ui/actions.json`, drives everything the
person uses to find and run an action:
- the menu bar,
- the right-click menus,
- tooltips,
- the shortcut reference,
- a single key dispatcher,
- and, embedded in the binary, `qd actions` for agents.

Because they all read one list, they can't drift (FR-001, FR-014).

Settings move out of the browser's per-port storage into a per-user
`settings.json` that every qd server reads. It is served at
`/api/settings`, changed through the token-guarded POST or `qd settings set`,
and pushed live to open editors over SSE. So settings stick across decks,
restarts and VS Code (US2).

Other parts:
- **Escape** saves an edit in progress instead of discarding it (the
  clarification).
- **Slides** can be copied and pasted through a new `paste` op that reuses
  `dup`'s fresh-id logic.
- **In VS Code** the bar folds into one ☰ menu, and keys VS Code owns are
  remapped or menu-only, checked against a list of VS Code's defaults.

Details: [research.md](research.md).

## Technical Context

**Language/Version**: Rust (stable, 2024) for qd; ES modules for the editor
(no build step); CommonJS for the VS Code extension (unchanged except tests).

**Primary Dependencies**: none new. serde_json and notify are already used.

**Storage**: `<config>/settings.json` per user (mode 0600, atomic writes).
`<config>` is `$QD_CONFIG_DIR`, else `~/Library/Application
Support/quartz-deck` on macOS, else XDG config. Nothing is added to the deck
or its folder.

**Testing**:
- **Rust unit tests:** setting definitions, validation, unknown-key
  preservation and reset; `actions.json` validity (unique ids, keys parse, no
  clashes per place, no VS Code-reserved keys in the panel); the `paste` op
  (fresh ids, one patch).
- **`tests/cli.rs`:** `qd actions`, `qd settings` and `qd paste`.
- **`tests/serve.rs`:** `/api/settings` GET/POST, the 403 without a token,
  400 on bad values, the SSE `settings` event (including after a CLI change),
  and `/ui/actions.js`.
- **Browser (`tests/onslide.rs`):**
  - the menu bar runs every action;
  - keyboard-only menus and ARIA roles;
  - every listed shortcut runs its action, and no action lacks a handler;
  - Escape order and that it keeps the edit;
  - slide copy/paste across two servers;
  - one undo per surface;
  - settings apply live and survive a restart;
  - the folded menu when framed.
- **Static check:** no key handling outside the dispatcher, apart from an
  allowlist.

**Target Platform**: desktop browsers (Chrome, Safari, Firefox current) and
VS Code ≥ 1.90's webview; macOS first, Linux CI.

**Project Type**: CLI + local web editor (+ VS Code extension from 006).

**Performance Goals**:
- menus and reference open in < 100 ms on 30 slides (SC-008);
- a setting change is visible in < 0.5 s (SC-003).

**Constraints**:
- clean-room: our own menu design;
- no build step;
- spec 004's rule that code runs only on the person's own action;
- every deck-changing action keeps its `qd` command.

**Scale/Scope**: about 80 actions in 7 menus and 6 settings. Editor JS grows
by a few hundred lines; much of it moves out of today's scattered key
handlers.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design.*

| Principle | How the plan complies | Status |
|---|---|---|
| I. Clean-room | Menu layout, labels and settings are our own, from our specs and the WAI-ARIA menubar pattern (public). | PASS |
| II. `.qmd` is truth | Settings are the person's preferences, not deck state, and live outside the deck. Paste is a normal minimal-span op. | PASS |
| III. CLI-first | `qd paste` before the editor's slide paste; `qd actions` and `qd settings` give agents the same lists and switches. | PASS |
| IV. Live preview | Unchanged. The `source.save` setting chooses when source typing reaches the file (and so the preview), never whether. | PASS |
| V. Safe concurrent edits | Paste goes through the store's CAS like every op. Settings writes are atomic, and a lost race between two writers loses only a preference. | PASS |
| VI. Test-first | Rust tests for definitions, the action list and paste, written before the code; browser tests for every story. | PASS |
| VII. Simplicity | One data file instead of four places that list keys; reuse of `menu.js` and `dup`'s id logic; no framework, no new crate. | PASS |
| Constraints 1.2.0 | Run actions keep `runs_code`, which never fires while typing and only on the person's key or click; settings POST is token-guarded. | PASS |

Post-design re-check: PASS. The one behaviour change, Escape saving instead
of cancelling, is the person's own choice (spec Clarifications).

## Project Structure

### Documentation (this feature)

```text
specs/007-app-shell/
├── plan.md · research.md · data-model.md · quickstart.md
├── contracts/ (http.md, cli.md, ui.md)
└── checklists/requirements.md
```

### Source Code (repository root)

```text
src/
├── settings.rs      # definitions, load (defaults on bad values), save (atomic, 0600), reset (+ unit tests)
├── ops.rs           # paste: split markdown into slides, insert with fresh ids (reuses renew_id)
├── cli.rs           # qd actions, qd settings [set|reset], qd paste
├── server.rs        # /api/settings GET/POST, /ui/actions.js, SSE settings, watch the settings file
├── store.rs         # config_root() next to cache_root()
└── ui/
    ├── actions.json # THE action list (embedded in qd; served as /ui/actions.js)
    ├── keys.js      # one key dispatcher from the list; ⌘/Ctrl by platform; VS Code overrides
    ├── menubar.js   # menu bar + folded ☰ menu over menu.js; shortcut reference dialog
    ├── settings.js  # Settings window; apply settings live; ui.* layout memory
    ├── actions.js   # right-click menus become views of the list (handlers stay here)
    ├── editor.js    # handlers registered by id; Escape order; slide copy/paste
    ├── index.html   # menubar container; header buttons kept as quick access
    └── editor.css   # menubar, reference dialog, focus ring, source on the right
vscode/default-keys.json   # VS Code defaults the panel must leave alone (R7)
tests/{cli,serve,onslide}.rs, vscode/test/keys.test.js
skills/quartz-deck/SKILL.md  # qd actions, qd settings, qd paste
```

**Structure Decision**: the editor gets three small modules (keys, menu bar,
settings), each with one job. The action list is data, not code.

## Complexity Tracking

| Addition | Why needed | Simpler alternative rejected because |
|---|---|---|
| A per-user settings file and endpoints | Settings must survive port changes and reach VS Code (FR-008) | `localStorage` is per origin, and the port changes every run |
| A static scan for stray key handling | FR-014: the reference must never miss a key | a list can't know about a key handled outside it without a check |
