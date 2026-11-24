# Implementation Plan: Builds inside columns

**Branch**: `024-builds-in-columns` | **Date**: 2026-11-23 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/024-builds-in-columns/spec.md`

## Summary

A `.columns` or `.column` div gets its contents as child blocks, parsed by
the scanner the slide already uses, and its children get ids that extend
its own: `SLIDE/N/M/K`. `Deck::element` resolves those paths and finds
explicit ids at any depth. The build ops, `block class` and `edit` work
on a child as they do on a top-level block, since they change only the
block's own span. The ops that rearrange a slide's top-level list (move,
dup, rm, place, box order, block add at an element) refuse a child. The
preview copy tags children with `data-qd`, so the editor's existing
`closest('[data-qd]')` click already picks the innermost element; the
editor learns to find a child's slide and block from a path, to step out
with Escape, and to disable the refused actions.

## Technical Context

**Language/Version**: Rust stable (edition 2024), plain ES modules in the editor

**Primary Dependencies**: none new

**Storage**: the `.qmd` file

**Testing**: `cargo test` — unit tests in `deck.rs`, `ops.rs`, `render.rs`; CLI tests in `tests/cli.rs` / `tests/ops.rs`; headless-browser tests in `tests/onslide.rs`

**Target Platform**: macOS/Linux, localhost editor

**Project Type**: CLI + local web editor

**Performance Goals**: parsing stays linear; a nested scan covers each line at most once more per level of columns

**Constraints**: byte-identical round trip; unchanged slides render as before

**Scale/Scope**: about 4 Rust files, 3 JS files, 2 test files, the showcase deck and the agent skill

## Constitution Check

| Principle | Check |
|---|---|
| I. Clean room | Quarto's documented columns syntax only. Pass. |
| II. `.qmd` is the truth | Children are spans into the same text; every op still patches one span. Round-trip tests extend to columns. Pass. |
| III. CLI first | Nested refs land in the CLI (PR 1) before the editor uses them (PR 2); `qd schema` and the skill document the ref form. Pass. |
| IV. Live preview | The preview is still Quarto's render; only `data-qd` tags are added inside columns. Pass. |
| V. Safe concurrent edits | No change to how writes are checked. Pass. |
| VI. Test-first | Deck and op tests first, then CLI and browser tests per story. Pass. |
| VII. Simplicity | One `children` field and a recursive walk; no new types for refs (a ref stays a string). Pass. |

Re-checked after Phase 1: no change.

## Project Structure

### Documentation (this feature)

```text
specs/024-builds-in-columns/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/refs.md
└── tasks.md
```

### Source Code

```text
crates/qd/src/
├── deck.rs        # Block.children, nested scan of .columns/.column, element() paths, explicit ids at depth
├── ops.rs         # build_list/order walk children; refusals for nested refs; Ctx ids include children
├── render.rs      # preview_copy tags children (recursive), cells inside columns untouched
├── cli.rs         # block_json "children"; schema text for refs
└── ui/
    ├── editor.js  # elementTarget by path; Escape steps out
    └── actions.js # rearranging actions disabled for a nested element, with why
crates/qd/tests/
├── ops.rs / cli.rs   # US2
└── onslide.rs        # US1
examples/showcase/showcase.qmd   # US3
skills/quartz-deck/SKILL.md      # nested refs
```

**Structure Decision**: the existing layout; no new files outside the spec.

## Pull requests

1. **Nested refs in the deck and the CLI** (US2): deck model, ops, `qd show`,
   `qd schema`, skill. The editor keeps working: children are not yet
   tagged in the preview, so it sees what it saw before.
2. **Select and build inside columns in the editor** (US1, US3): preview
   tags, `elementTarget` by path, Escape, disabled actions, browser tests,
   the showcase slide.

## Complexity Tracking

None.
