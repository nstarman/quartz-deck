# Implementation Plan: Layouts and placement — start from a layout, place anything anywhere

**Branch**: `009-layouts` | **Date**: 2026-11-11 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/009-layouts-and-placement/spec.md`

## Summary

A new slide can start from a layout, and any element can be taken out of the
flow and placed anywhere, with the deck staying plain Quarto
([research](research.md)).

- **Layouts are positioned boxes.** A layout writes a heading and its
  content areas as positioned boxes, the `.absolute` divs that text boxes
  already are (R1), because the deck's elements are top-level blocks and
  every box is one an agent can address and the editor can type into, move
  and resize.
- **Placeholders are empty, marked boxes.** They render as nothing, the
  editor shows a prompt over them, and filling one drops the markers (R2).
- **Placing wraps the element.** It goes inside the same kind of box, with
  its bytes unchanged, and unplacing takes the wrapper off exactly (R4).
- **Saved layouts are files** of the same form as the built-in ones: the
  person's own in qd's config folder, or the deck's in `_layouts/` beside it
  (R3).
- **Guides, snapping, nudging and stacking** are editor work and box
  reorders (R5, R6).

## Technical Context

**Language/Version**: Rust (stable, 2024) for qd; ES modules for the editor
(no build step).

**Primary Dependencies**: none new.

**Storage**: the `.qmd`; saved layouts as markdown files
(`<config>/layouts/`, `_layouts/` beside the deck); two settings (007).

**Testing**:
- **Rust unit tests (`src/layouts.rs`, `src/ops.rs`):**
  - each built-in parses; scaling to another slide size;
  - adding gives fresh ids and returns placeholder refs;
  - filling a placeholder drops its markers;
  - place then unplace is byte for byte the original, for every placeable
    kind; refusals give their reasons;
  - stacking reorders only boxes.
- **`tests/cli.rs`:** `qd layouts`, `qd add --layout`, `qd layout
  save/rm/rename` (name rules, `--replace`), `qd block place/unplace`, `qd
  box front/back/forward/backward`, one undo each.
- **`tests/serve.rs`:**
  - `GET /api/layouts`;
  - `POST /api/layouts`: 403 without the token, 409 on a taken name.
- **Browser (`tests/onslide.rs`):**
  - the gallery from + and Insert, typing into placeholders, dropping an
    image on an image placeholder;
  - ⌥-drag to place, the handles, nudge keys, unplace;
  - a refused element;
  - saving a layout;
  - guides and snapping on and off;
  - front and back.
- **`audit_measures` (008)** covers the gallery and Arrange menu (contrast,
  size, overlap).
- **SC-002 render check:** every built-in layout added and left unfilled,
  rendered with Quarto (skipped without Quarto, as other render tests are).

**Target Platform**: desktop browsers and VS Code ≥ 1.90's webview; macOS
first, Linux CI.

**Project Type**: CLI + local web editor (+ VS Code extension).

**Performance Goals**: the gallery opens in under 100 ms; drags and guides at
60 fps (rectangle arithmetic per pointer move, no layout reads in the loop
beyond the dragged box).

**Constraints**:
- clean-room;
- no build step, no new dependency;
- every action in the action list, every edit one `qd` command and one undo
  step, touching only its own bytes;
- decks render the same with Quarto alone.

**Scale/Scope**: a new `src/layouts.rs` plus 12 built-in layout files; ops
`add_layout`, `block_place`, `block_unplace`, `box_order`, placeholder fill;
CLI and two HTTP routes; the editor's gallery, placement drag, guides, nudge
and Arrange menu.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design.*

| Principle | How the plan complies | Status |
|---|---|---|
| I. Clean-room | Our own layouts and design; slide apps as general practice only. | PASS |
| II. `.qmd` is truth | A layout is only the markdown it writes; placeholders are empty boxes plus two attributes that drop on fill; placing is Quarto's own `.absolute`. Saved layouts are files the person chose to save. | PASS |
| III. CLI-first | `qd layouts`, `qd add --layout`, `qd layout save/rm/rename`, `qd block place/unplace`, `qd box front…` before the editor uses them; `qd add --layout` returns placeholder refs for agents. | PASS |
| IV. Live preview | Placement and layouts render through the same preview; the gallery's previews come from the layout's boxes. | PASS |
| V. Safe concurrent edits | Each change is one op through the store's version check; place/unplace touch only the element's bytes. | PASS |
| VI. Test-first | Each story's tests are written first and seen failing; byte-for-byte round trips are unit tests. | PASS |
| VII. Simplicity | Boxes reused for layouts and placement instead of nested column refs (R1); no new dependency. | PASS |
| Constraints 1.2.1 | `POST /api/layouts` is token-guarded and writes only `layouts/` in the config folder or `_layouts/` beside the deck, with checked names, never overwriting without `replace`. | PASS |

Post-design re-check: PASS.

Final re-check after US1–US5 (polish, 2026-11-12): PASS.
- **II:** layouts, placement and stacking write only Quarto (`.absolute` boxes, source order). Unplace restores the bytes exactly, and nothing is stored beside the deck except saved layouts the person chose to keep.
- **III:** every action is a `qd` command first.
- **V:** every edit is one patch through the store.
- **VI:** each story's tests were seen failing first.
- **VII:** no new dependency.
- **Constraint 1.2.1:** `POST /api/layouts` is token-guarded and folder-bound, with a mutation check.

## Project Structure

### Documentation (this feature)

```text
specs/009-layouts-and-placement/
├── spec.md · plan.md · research.md · data-model.md · quickstart.md
├── contracts/ (cli.md, http.md, ui.md)
└── checklists/requirements.md
```

### Source Code (repository root)

```text
src/
├── layouts.rs       # built-in + saved layouts: list, read, scale, save, rm, rename
├── layouts/*.md     # the 12 built-in layouts (include_str!)
├── ops.rs           # add_layout, block_place, block_unplace, box_order, placeholder fill in edit
├── cli.rs           # qd layouts, add --layout, layout save/rm/rename, block place/unplace, box front…
├── server.rs        # GET/POST /api/layouts
├── settings.rs      # placement.snap, ui.last_layout
└── ui/
    ├── sheet.js     # the gallery field
    ├── editor.js    # placeholder prompts (PREVIEW_CSS), ⌥-drag to place, guides, nudge, Arrange
    ├── actions.json # place, unplace, front…, nudge, layout-save, layouts-manage, snap
    └── editor.css   # gallery, guides, off-slide marker
tests/{cli,serve,onslide,actions}.rs
skills/quartz-deck/SKILL.md  # layouts and placement for agents
```

**Structure Decision**: layouts get their own module (`src/layouts.rs`) beside
`images.rs` (008); the ops stay in `ops.rs` with the other box ops they
reuse; the editor changes in place.

## Complexity Tracking

| Addition | Why needed | Simpler alternative rejected because |
|---|---|---|
| `POST /api/layouts` (writes outside the `.qmd`) | Saving a layout from the editor (US3) | Asking the person to write layout files by hand |
| Placeholder markers in the deck (`.qd-ph ph=…`) | The editor and agents must know an empty box is a placeholder (FR-004, FR-005) | Placeholder words in the file would render (FR-004) |
