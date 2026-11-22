# Implementation Plan: Diagram source follows the cursor

**Branch**: `021-dot-source-follow` | **Date**: 2026-11-22 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/021-dot-source-follow/spec.md`

## Summary

With the cursor (or the resting mouse) on a box's or an arrow's line in a
`{dot}` cell's source, the box or arrow is outlined on the slide, and a
small drawing of it alone shows beside the line.

- **Which statement**: a new read-only op, `qd dot at`, runs qd's own DOT
  reader on the text as typed and answers with the statement at an offset
  (R1, R2).
- **Outline**: a class on that element of the slide's SVG, found the way
  selection already finds it. The selection is untouched (R4).
- **Preview**: `qd dot at` also writes a tiny graph of the element alone,
  with the cell's defaults. The editor lays it out with the Graphviz the
  page already loads for spec 020, and shows it in a pop-over (R3).

Nothing is written to the deck.

## Technical Context

**Language/Version**: Rust stable (edition 2024) for `qd dot at`; plain ES modules for the editor

**Primary Dependencies**: none new; the in-page Graphviz from spec 020

**Storage**: none; a little page state (data-model.md)

**Testing**:
- `cargo test`: `dot.rs` unit tests and `tests/dot.rs` for `qd dot at` (contracts/cli.md § Tests).
- `tests/onslide.rs` browser tests (contracts/editor.md § Tests), with Quarto slowed through `QD_QUARTO` to see "before the render".
- Tests that need Quarto, Graphviz or a browser skip without them.

**Target Platform**: macOS first; nothing Linux-hostile

**Project Type**: CLI + local web editor (existing)

**Performance Goals**: outline and preview within 100 ms of the cursor stopping on a 20-box diagram (SC-002); typing no slower (SC-003)

**Constraints**: no change to the file, the selection or the undo history (FR-003); nothing in presenting or exports (FR-011)

**Scale/Scope**: about 20 boxes per diagram; one diagram followed at a time

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status |
|---|---|
| I. Clean room | ✅ Our own code. Graphviz is used as Quarto ships it. The look is ours. |
| II. `.qmd` is the truth | ✅ Nothing is written. The op reads text the editor sends and returns positions in it. |
| III. CLI first | ✅ `qd dot at` is a CLI op with JSON out, in `qd schema`, usable by agents, e.g. "what is at line 4 of this diagram". The editor uses it over `/api/op`. |
| IV. Live preview | ✅ The slide still shows Quarto's render. The outline is a class on it, and the pop-over is drawn by the same engine. |
| V. Safe edits | ✅ No edit, so no conflict path. Unsaved text is only read. |
| VI. Test first | ✅ Each story's tests come first (contracts). |
| VII. Simplicity | ✅ One read-only op on the existing parser, about 120 lines of Rust. In the editor, about 150 lines in `dot.js` and a few hooks in `pane.js` and `source.js`. No second DOT parser (R1). |
| Licences | ✅ Nothing new. |

Gate: **pass**.

## Project Structure

### Documentation (this feature)

```text
specs/021-dot-source-follow/
├── spec.md, plan.md, research.md, data-model.md, quickstart.md
├── contracts/
│   ├── cli.md         # qd dot at
│   └── editor.md      # the outline, the pop-over, their tests
└── tasks.md           # /speckit-tasks
```

### Source Code (repository root)

```text
crates/qd/src/
├── dot.rs              # at(text, offset): the statement there; alone(): a lone element's graph
├── cli.rs              # DotCmd::At, schema line
├── settings.rs         # diagrams.source-preview (default true)
└── ui/
    ├── dot.js          # follow(cell, text, offset): outline + #dot-peek pop-over
    ├── pane.js         # caret, typing and resting-pointer reports inside a {dot} fence
    ├── source.js       # the same from the cell editor on the slide (US3)
    ├── editor.css      # .qd-dot-follow (in the preview's injected style), #dot-peek
    └── index.html      # #dot-peek
crates/qd/tests/
├── dot.rs              # qd dot at
└── onslide.rs          # source_follow_* browser tests
README.md, skills/quartz-deck/SKILL.md   # dot at, the outline and preview
```

**Structure Decision**: the statement lookup lives in `dot.rs`, beside the
reader it uses. The outline and the pop-over live in `dot.js`, which owns
diagrams in the editor. `pane.js` and `source.js` only report where the
cursor is.

## PRs (order of work)

1. **US1, the outline:** `qd dot at` with its tests, then the source
   pane's caret and hover reports and the outline, with browser tests.
2. **US2, the preview:** `alone`, the pop-over, the stale mark, the
   setting, and their tests.
3. **US3 and polish:** the cell editor on the slide, README and skill,
   the quickstart run with the SC-002 timing, the final re-check, and
   trying it on the person's talk (SC-005).

Each PR passes `/pr-audit`.

## Complexity Tracking

None: no principle is bent.
