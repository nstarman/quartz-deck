# Implementation Plan: Arrange elements inside columns

**Branch**: `025-arrange-in-columns` | **Date**: 2026-11-24 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/025-arrange-in-columns/spec.md`

## Summary

The block ops work on "the list of blocks an element sits in" instead of
`slide.blocks`. Spec 024's tree already knows that list: a slide's top level,
or a columns/column div's `children`. A move inside one list keeps using
`rearrange`, made to take any list. A move between lists is two edits in one
patch: cut the element (with the blank line beside it) out of its list, and
insert it at the target. `--into COLUMN` targets the end of a column, or its
inside when it is empty. Place freely on a nested element is a cut plus a box
at the end of the slide. A duplicated build gets the next place in the build
order. In the editor, the actions spec 024 greyed out come back, Move up/down
and the drag work on siblings, and an empty column shows "+ Add content".

## Technical Context

**Language/Version**: Rust stable (edition 2024), plain ES modules in the editor

**Primary Dependencies**: none new

**Storage**: the `.qmd` file

**Testing**: `cargo test`: unit tests in `ops.rs`, CLI tests in `tests/cli.rs`, headless-browser tests in `tests/onslide.rs`

**Target Platform**: macOS/Linux, localhost editor

**Project Type**: CLI + local web editor

**Performance Goals**: unchanged: one parse per op

**Constraints**: every op changes only the text it moves; bytes elsewhere stay

**Scale/Scope**: `deck.rs`, `ops.rs`, `cli.rs`, three editor files, two test files, the showcase, the skill

## Constitution Check

| Principle | Check |
|---|---|
| I. Clean room | Container behaviour described in our own words; no other app's code or wording. Pass. |
| II. `.qmd` is the truth | Moves are span cuts and inserts; `keep_structure` re-parses every result. Pass. |
| III. CLI first | PR 1 lands every op in the CLI; the editor (PR 2) only calls them. Pass. |
| IV. Live preview | Unchanged. Pass. |
| V. Safe concurrent edits | Unchanged write path. Pass. |
| VI. Test-first | CLI tests per op before code; browser tests per editor story. Pass. |
| VII. Simplicity | One "siblings of" lookup and two span helpers (cut, insert) replace the per-op refusals; no new types. Pass. |

## Project Structure

### Documentation (this feature)

```text
specs/025-arrange-in-columns/
├── plan.md  research.md  data-model.md  quickstart.md  contracts/cli.md  tasks.md
```

### Source Code

```text
crates/qd/src/
├── deck.rs        # Deck::siblings(r): the list an element sits in, and its index
├── ops.rs         # rearrange over any list; cut/insert helpers; block move/add/rm/dup/place with nested refs and --into; dup's build goes last; column width, add, rm (P4)
├── cli.rs         # --into on block move and block add; block attr (P4)
└── ui/
    ├── actions.js # SLIDE_LEVEL gate removed; Move up/down name siblings
    ├── editor.js  # drag across containers, drop on an empty column; "+ Add content" in an empty column
    └── inspector/format view: column width (P4)
crates/qd/tests/{cli.rs,onslide.rs}
examples/showcase/showcase.qmd, skills/quartz-deck/SKILL.md
```

## Pull requests

1. **The CLI arranges inside columns** (US6, CLI side of US1–US4, FR-010/011): `siblings`, cut/insert, nested refs for move/add/rm/dup/place, `--into`, the copied build last.
2. **The editor arranges inside columns** (US1–US3 editor side, US4 menu): actions back on, siblings for Move up/down, drag across containers, "+ Add content", paste into a column; showcase slide.
3. **Column widths and number** (US5, P4): `qd block attr`, `qd column add`, column delete; Format view fields. Built last; can be left for later if the person says so.

The duplicate-id fix the spec relies on (FR-010) is PR #137, merged first.

## Complexity Tracking

None.
