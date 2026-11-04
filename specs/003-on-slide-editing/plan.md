# Implementation Plan: On-Slide Editing

**Branch**: `003-on-slide-editing` | **Date**: 2026-11-04 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/003-on-slide-editing/spec.md`

## Summary

The rendered slide becomes the editing surface. Clicking a typeable element
(title, paragraph, list, quote, text box) makes it editable in place with a
full formatting toolbar; on save the editor serializes the element back to
markdown and sends it through the existing `edit` op, which writes only the
bytes that changed. An element whose rendering does not serialize back to its
exact source (math, code, shortcodes, unusual markup) opens an in-place source
editor over the element instead — a **round-trip gate** decides this
automatically per element. Slide and element properties move to a toolbar
attached to the selection and to right-click **context menus** fed by one
action table. New `qd block add|move|rm|dup` ops cover structure, and an
element-level `--expect-text` check lets the person and an agent edit
different elements at the same time without false conflicts. Re-renders wait
until an edit ends. The form panel becomes a hidden "Source" view.

## Technical Context

**Language/Version**: Rust stable, MSRV 1.89 (unchanged); editor in plain
JavaScript/CSS served by `qd serve` (unchanged approach, no build step).

**Primary Dependencies**: none new. Browser tests use `chromiumoxide` (added
in spec 002).

**Storage**: the `.qmd` only (unchanged); the Source panel's open/closed state
in the browser's `localStorage` until spec 004 adds Settings.

**Testing**: Rust unit tests for the new ops (`block *`, minimal-patch trim,
`--expect-text`); CLI tests with snapshots; headless-Chrome integration tests
that drive the real editor (type, format, save, menus, conflicts) and assert
on the file bytes; a corpus round-trip test over ≥ 20 decks measuring the
typeable share and verifying no gate false-positives (SC-002/003).

**Target Platform**: desktop browsers (Chrome, Safari, Firefox current);
mouse/trackpad + keyboard. Touch out of scope.

**Project Type**: additions to the existing single crate + editor assets.

**Performance Goals**: typed text visible immediately; real render replaces it
≤ 3 s after save (SC-004); context menu opens < 100 ms.

**Constraints**: `.qmd` byte-identical outside the edited span (constitution
II); every browser action has a CLI equivalent (III); no lost edits under
concurrent agent edits (V).

**Scale/Scope**: decks up to ~100 slides; elements up to a few KB of source.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design.*

| Principle | How the plan complies | Status |
|---|---|---|
| I. Clean-room | Built from public docs (Pandoc/Quarto markdown, reveal.js, DOM/ARIA specs); no slides.com material or code; the round-trip and menu designs are our own. | PASS |
| II. `.qmd` is truth | Edits go through ops; server trims each replacement to the changed bytes; round-trip gate prevents saving anything the serializer can't reproduce; rendering stays with Quarto. | PASS |
| III. CLI-first | New `qd block add/move/rm/dup` and `edit --expect-text`; every menu/toolbar action maps to a `qd` command (contract table); Skill + schema + sync test updated. | PASS |
| IV. Live preview | The preview *is* the editor; renders deferred only while an edit is open, then applied. | PASS |
| V. Safe writes | Element-level conflict check; unsaved text kept on conflict; shared undo journal unchanged. | PASS |
| VI. Test-first | Op tests and browser-driven tests written before each piece. | PASS |
| VII. Simplicity | No framework: native contenteditable + execCommand + small serializer; one action table for menu, toolbar and keys. See Complexity Tracking. | PASS (justified) |

Post-design re-check: PASS — data model adds no state to the deck; the only
browser-only state is the edit session and panel visibility.

Final check after implementation (T038, 2026-11-04): PASS. I — built from
public docs (Pandoc/Quarto markdown, reveal.js, DOM/ARIA); no slides.com
material. II — every on-slide edit goes through a qd op that writes only the
element's span (heading never re-pinned by element edits); the round-trip gate
keeps untouched markup; `Attrs` now keeps untouched attribute items as written;
browser tests assert exact bytes and undo restores them. III — every menu,
bar and key action is a `qd` command (`qd block add|move|rm|dup|class`,
`edit/title --expect-text`), documented in the skill; the API maps them 1:1.
IV — the preview is the editor; renders wait while an edit is open. V —
element-level conflict checks, conflict bar keeps both versions, 50-edit
person+agent stress test in CI. VI — tests written first for each piece (CLI,
ops, 17 browser tests over a corpus). VII — no framework or new dependency;
two concision audits (−122 and −60 lines). Deviations recorded in tasks.md
(4-deck corpus; T006 built with US6; server-side trim unnecessary).

## Project Structure

### Documentation (this feature)

```text
specs/003-on-slide-editing/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── cli.md        # qd block *, edit --expect-text, show hashes
│   └── ui.md         # action table: menu/toolbar/shortcut → qd command
└── tasks.md          # /speckit-tasks
```

### Source Code (repository root)

```text
src/
├── ops.rs        # + block_add/move/rm/dup; edit trims to minimal patch; expect_text check
├── deck.rs       # + per-block source hash
├── cli.rs        # + `qd block …`, `edit --expect-text`; API_OPS += block ops
├── server.rs     # serve new UI modules
└── ui/
    ├── index.html    # Source toggle; toolbar + menu containers
    ├── editor.js     # wiring: selection, sessions, deferred reloads (slimmed)
    ├── actions.js    # one action table (label, shortcut, enabled, run)
    ├── inline.js     # contenteditable sessions, round-trip gate, md serializer
    ├── toolbar.js    # formatting + property toolbars
    ├── menu.js       # context menu component (ARIA, keyboard)
    ├── source.js     # in-place source editor overlay
    └── editor.css
tests/
├── blocks.rs         # block ops + minimal patch + expect-text
├── onslide.rs        # headless-Chrome editor tests (gated on browser)
└── fixtures/corpus/  # ≥ 20 decks for round-trip coverage
```

**Structure Decision**: same crate; the editor grows from one script into a
few plain ES modules (one concern each) because inline editing, menus and
toolbars together would triple `editor.js`.

## Complexity Tracking

| Addition | Why needed | Simpler alternative rejected because |
|---|---|---|
| Own markdown serializer (~300 lines JS) | Full toolbar (clarified) must produce markdown | Plain-text-only editing was declined by the user; a framework adds a second document model |
| Round-trip gate | Guarantees untouched constructs are never rewritten | Trusting the serializer alone risks silent source changes (violates II) |
| `--expect-text` | Agent edits elsewhere must not block the person's save (US5) | File-level `--expect` refuses unrelated saves |
| ES modules split | Keeps each concern readable | One 2000-line script |
