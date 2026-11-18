# Implementation Plan: Diagrams that keep up with the hand

**Branch**: `020-diagram-speed` | **Date**: 2026-11-17 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/020-diagram-speed/spec.md`

## Summary

A drop waits about 3 s, and more on a real deck, for Quarto to render the
whole deck again. Graphviz itself takes milliseconds (research R1). The
editor already carries Quarto's Graphviz engine, and a spike showed that
its layout of a cell is identical to Quarto's render (R2). So:
- **Instant drawing**: after every diagram edit the editor lays out the
  cell in the page and puts that drawing in place at once, pictures
  included (R2, R3, R5).
- **No reload when nothing else changed**: when Quarto's render lands, the
  editor skips the reload if only the cells it already drew changed (R4).
- **Live routing on**: arrows are routed live while dragging, by default
  (R6).

The file and the rendered deck don't change.

## Technical Context

**Language/Version**: plain ES modules for the editor; Rust stable (edition 2024) for one setting default

**Primary Dependencies**: none new; Quarto's `graphviz-wasm.js` and `graphvizlib.wasm`, served from the person's Quarto as in spec 015

**Storage**: nothing new; the editor keeps a little page state (data-model.md)

**Testing**:
- `cargo test`:
  - browser tests in `tests/onslide.rs` (contracts/editor.md § Tests), with Quarto slowed by a wrapper script through `QD_QUARTO`, so the time between drop and render is visible;
  - the drawing and the render compared shape by shape (SC-003);
  - `dot-image`'s JS port against its Lua filter on the plugin's example deck;
  - the settings test for the new default.
- Tests that need Quarto or a browser skip without them.

**Target Platform**: macOS first; nothing Linux-hostile

**Project Type**: CLI + local web editor (existing)

**Performance Goals**: release to routed arrows < 100 ms on up to 20 boxes (SC-001); the box at the pointer's frame rate while dragging (SC-004)

**Constraints**: byte-identical file and render (FR-006); a reload is skipped only when provably nothing else changed (R4)

**Scale/Scope**: about 20 boxes per diagram; any deck size

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status |
|---|---|
| I. Clean room | ✅ Our own code; Quarto's Graphviz build used as Quarto ships it; `dot-image` is ours. |
| II. `.qmd` is the truth | ✅ Unchanged: the same ops write the same patches. The instant drawing is only what the editor shows. |
| III. CLI first | ✅ No new edit. The setting's default is in `qd settings`. |
| IV. Live preview | ✅ The rendered output is still real Quarto. A reload is skipped only when the render equals what's shown (R2, R4), and a test compares them. |
| V. Safe edits | ✅ Unchanged handlers and conflict checks. |
| VI. Test first | ✅ Each behaviour's browser test comes first, with a slowed Quarto to see "before the render". |
| VII. Simplicity | ✅ No new dependency or module: about 100 lines in `dot.js` and a small rule in `editor.js`. Faster Quarto for every edit is left to its own spec (R8). |
| Licences | ✅ Nothing new. |

Gate: **pass**.

## Project Structure

### Documentation (this feature)

```text
specs/020-diagram-speed/
├── plan.md, research.md, data-model.md, quickstart.md
├── contracts/
│   └── editor.md      # what the editor shows when, and its tests
└── tasks.md           # /speckit-tasks
```

### Source Code (repository root)

```text
crates/qd/src/
├── settings.rs         # diagrams.live-layout defaults to true
└── ui/
    ├── dot.js          # instant drawing of a cell (Graphviz, pictures, re-marking); redraw after ops
    └── editor.js       # shown {version, source, drawn}; skip the reload when only drawn cells changed
crates/qd/tests/
├── onslide.rs          # browser tests, Quarto slowed through QD_QUARTO
└── dot.rs              # dot-image JS against Lua (or plugins.rs)
README.md               # one line: diagrams draw at once; the setting
```

**Structure Decision**: the drawing stays in `dot.js`, which owns diagrams
in the editor. The reload rule goes in `editor.js`, which owns the preview.

## PRs (order of work)

1. **US1, instant drawing and no reload:**
   - redrawing a cell after each op, pictures included;
   - the reload rule;
   - tests with a slowed Quarto.
2. **US2, live routing by default:** the setting's default, and a check
   that dragging keeps the pointer's rate on a 20-box diagram.
3. **Polish:** README, quickstart run, final re-check, and trying it on the
   person's talk.

Each PR passes `/pr-audit`.

**Final re-check after the build (T013, 2026-11-18)**: pass.
- **I.** Our own code. Quarto's Graphviz is used as Quarto ships it, and
  `dot-image`'s placement is ported from qd's own filter.
- **II.** Unchanged: the same ops write the same patches. The instant
  drawing is only what the editor shows.
- **III.** No new edit. `dot show` gained a read-only `spacer` field, in
  `qd schema` and the contract. The new default is in `qd settings`.
- **IV.** Quarto's render still arrives, and a reload is skipped only when
  the text it was made from is what the editor drew. A test checks that
  the page's drawing equals the render, pictures included, and that an
  outside undo still reloads.
- **V.** Unchanged handlers and conflict checks. A cell isn't redrawn under
  a drag that's already running on it.
- **VI.** Each story's browser tests were seen failing first, against a
  Quarto slowed by 4 s. The talk's diagram exposed a bug (a linked box
  with a picture); its test failed first, too.
- **VII.** No new dependency or module: about 100 lines in `dot.js`, a
  small rule in `editor.js`, and one default.
- **Measured:** 79 ms (made-up deck) and 24 ms (the person's talk, 15
  boxes) from release to drawing, against renders of 3.9 s and 23.8 s.

## Complexity Tracking

None: no principle is bent.
