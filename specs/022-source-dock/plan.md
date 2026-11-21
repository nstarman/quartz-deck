# Implementation Plan: Dock the source pane by dragging it

**Branch**: `022-source-dock` | **Date**: 2026-11-20 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/022-source-dock/spec.md`

## Summary

The existing setting `source.side` (left, right) gains two values,
`far-left` and `below`, and the editor lays the pane out from it.
- **Places**: one `data-source` attribute on `<main>` and CSS `order`
  for the side places; a small grid only for `below` (research R2).
- **Splitter**: the same element, its axis from the place; a new
  `ui.source_height` remembers the height below (R3).
- **Drag**: a press on the title bar's empty part, four labelled zones
  over the window, the drop writes the setting (R4).
- **Right side**: slide, inspector, source pane (spec decision; R5).
- **Narrow window**: CSS puts the pane below the slide at 700 px or less,
  without touching the setting (R6).

Editor work plus two enum values and one `ui.*` key in the core.

## Technical Context

**Language/Version**: Rust stable (edition 2024) for the setting; plain ES modules and CSS for the editor

**Primary Dependencies**: none new

**Storage**: the person's settings file (`source.side`, `ui.source_height`); nothing in the deck

**Testing**:
- `cargo test`:
  - `settings.rs` unit test: the new values are accepted, an unknown value is refused, old files keep `left` / `right`;
  - browser tests in `tests/onslide.rs`, one per acceptance scenario: each place from the setting (pane's rect against the slide's), the drag to each zone with `press_and_move` / `drag`, Esc and a drop outside cancel, a click on a title-bar button starts no drag, typing kept across a move, the splitter's axis and remembered size per place, Full width in each place, the inspector's order on the right at 1440 px, the narrow window at 650 px;
  - the existing source pane, splitter, Full width and inspector tests keep passing in the default place.

**Target Platform**: macOS first; nothing Linux-hostile; the same page in VS Code's panel

**Project Type**: CLI + local web editor (existing)

**Performance Goals**: a drop re-lays out in one frame; the pane's textarea is never re-created, so its text, caret, scroll and undo stack stay

**Constraints**: the default place and every existing layout stay as they are, except the right side's order (spec FR-009)

**Scale/Scope**: four places; about 5 editor files and `settings.rs`

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status |
|---|---|
| I. Clean room | ✅ Docking by dragging a title bar onto drop zones is common to IDEs; our own zone labels and look (R1). |
| II. `.qmd` is the truth | ✅ No deck state. |
| III. CLI first | ✅ No new edit. The place is `qd settings set source.side below`. |
| IV. Live preview | ✅ Unchanged. |
| V. Safe edits | ✅ The pane's element is moved by CSS only, never rebuilt; unsaved typing stays (SC-004, tested). |
| VI. Test first | ✅ Browser tests for each scenario before the code in each PR. |
| VII. Simplicity | ✅ The existing setting and splitter; CSS order for the side places; one small new module for the drag. |
| Licences | ✅ No new dependency. |

Gate: **pass**.

## Project Structure

### Documentation (this feature)

```text
specs/022-source-dock/
├── plan.md, research.md, data-model.md, quickstart.md
├── contracts/
│   └── editor.md      # places, zones, splitter, setting values
└── tasks.md           # /speckit-tasks
```

### Source Code (repository root)

```text
crates/qd/src/
├── settings.rs         # source.side: far-left, left, right, below; ui.source_height
└── ui/
    ├── dock.js         # new: title-bar drag, drop zones, Esc
    ├── editor.js       # data-source on <main>; splitter axis and size per place
    ├── editor.css      # order per place; the grid for below; narrow window; zones
    └── index.html      # (only if the zones need a static container)
crates/qd/tests/
├── onslide.rs          # browser tests
└── serve.rs, cli.rs    # setting values, if their assertions list the choices
README.md, skills/quartz-deck/SKILL.md   # the places, briefly
```

**Structure Decision**: one small new module for the drag. Layout stays
in CSS keyed on one attribute; the splitter stays in `editor.js`.

## PRs (order of work)

1. **US2 + US3, the places:** the setting's new values, `data-source`,
   CSS for the four places and the narrow window, the splitter's axis and
   `ui.source_height`, Full width and the inspector in each place. The
   place is chosen in Settings (the keyboard way); fully usable alone.
2. **US1, the drag:** `dock.js`, the zones, Esc and cancel; docs
   (README, skill); quickstart run; final re-check.

Each PR passes `/pr-audit`.

**Final re-check after the build (T026, 2026-11-20)**: pass.
- **I.** Our own zone labels and look, drawn with the editor's accent
  colour; no other product's screenshots or wording in the repo.
- **II.** Nothing is stored in the deck.
- **III.** No new edit. The place is `qd settings set source.side …`;
  the drag only writes that setting.
- **IV.** Unchanged.
- **V.** The pane is laid out by CSS, never moved or rebuilt; a test types,
  moves it three times and saves.
- **VI.** Each story's browser tests were written first and failed first.
- **VII.** No new dependency. One small module (`dock.js`, about 80
  lines); the places are CSS `order` plus one grid.

## Complexity Tracking

None: no principle is bent.
