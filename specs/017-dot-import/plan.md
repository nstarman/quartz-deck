# Implementation Plan: Import a drawing as a diagram

**Branch**: `017-dot-import` | **Date**: 2026-11-20 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/017-dot-import/spec.md`

## Summary

`qd dot import FILE.svg` reads a drawing and writes one pinned `{dot}` cell.
It handles hand-written SVG and SVG that Graphviz rendered.
- **Reading:** a new module, `dot_import.rs`, reads the SVG with `roxmltree`
  and composes transforms (research R1, R2).
- **Boxes:** rectangles, circles and ellipses with their text become boxes
  (R3).
- **Arrows:** lines whose ends are near two boxes become arrows, routed by
  the diagram (R4).
- **Looks, links and pictures:** carried over with qd's existing helpers
  (R5).
- **Size and placement:** the cell is sized to show at the drawing's size
  (R6). It's placed or swapped in with the existing ops, in one undo step
  (R8).
- **Report:** everything left out is listed with a reason.

## Technical Context

**Language/Version**: Rust stable (edition 2024)

**Primary Dependencies**: `roxmltree` (new; MIT OR Apache-2.0, no dependencies of its own)

**Storage**: the `.qmd`; with a picture, the deck's `filters:` and `_extensions/dot-image`, as `qd dot set image` does today

**Testing**:
- **Unit tests in `dot_import.rs`:** transforms; path ends and directions; text into boxes; arrow ends near a box or along the line; names from labels; looks.
- **CLI tests in `tests/dot.rs`:**
  - `dot import` on `hand.svg` and `graphviz.svg` (made-up fixtures, research R9): JSON report, file, `--dry-run`, `--replace`, undo, errors;
  - the result read back by `qd dot show` and edited by `qd dot move`.
- **A Quarto render of an imported deck:** the boxes land within 5 px of the drawing (SC-002), and the picture shows. It skips without Quarto.

**Target Platform**: macOS first; nothing Linux-hostile

**Project Type**: CLI + local web editor (existing); CLI only here (spec)

**Performance Goals**: a 20-box drawing imports in well under a second (SC-001 asks < 5 s)

**Constraints**: byte-exact patches elsewhere in the deck; the cell renders with plain Quarto (plus `dot-image` for pictures); nothing written on any error

**Scale/Scope**: drawings of up to about 50 shapes

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status |
|---|---|
| I. Clean room | ✅ SVG and DOT are open formats; our own reader; no other app's files or code. Exports from proprietary apps are out of scope. |
| II. `.qmd` is the truth | ✅ The result is an ordinary `{dot}` cell (spec 015). Nothing is kept outside the deck. |
| III. CLI first | ✅ It is a command (the person chose CLI only), with JSON, `--dry-run`, `--expect`, `qd schema` and the skill. |
| IV. Live preview | ✅ It's an edit like any other: the preview re-renders. |
| V. Safe edits | ✅ One op, version-checked, one undo step; nothing written on an error. |
| VI. Test first | ✅ Unit tests for each reading step, then CLI tests on fixtures, then a render check. |
| VII. Simplicity | ✅ One new module and one small, well-kept crate, rather than a hand-made XML parser (R1). Writing the cell reuses `node_stmt`, `spacer`, `normalised`, `block_add`, `edit` and `with_filters`. |
| Licences | ✅ `roxmltree`: MIT OR Apache-2.0, checked by `dependency_licenses_are_allowed`. |

Gate: **pass**.

## Project Structure

### Documentation (this feature)

```text
specs/017-dot-import/
├── plan.md, research.md, data-model.md, quickstart.md
├── contracts/
│   └── cli.md          # qd dot import: arguments, the cell written, the report
└── tasks.md            # /speckit-tasks
```

### Source Code (repository root)

```text
crates/qd/
├── Cargo.toml                  # roxmltree
└── src/
    ├── dot_import.rs           # new: read the drawing; boxes, arrows, report; write the cell
    ├── dot.rs                  # node_stmt, spacer, normalised made pub(crate) for it
    ├── cli.rs                  # `qd dot import`; schema; API_OPS
    └── lib.rs                  # mod dot_import
crates/qd/tests/
├── dot.rs                      # CLI tests
└── fixtures/dot-import/
    ├── hand.svg                # made-up hand-written drawing
    └── graphviz.svg            # made-up DOT rendered by Graphviz
skills/quartz-deck/SKILL.md, README.md
```

**Structure Decision**: one new module beside `dot.rs`. Reading SVG is a
separate concern from editing DOT, and `dot.rs` is already long.

## PRs (order of work)

1. **US1, hand-written drawings:**
   - the reader (R1–R5), the cell (R6) and placing it (R8);
   - the fixtures, unit and CLI tests, and the render check.
2. **US2, Graphviz drawings:** R7, with its fixture and tests.
3. **Polish:** README, skill, quickstart run, and the person's own diagram
   (SC-005, outside the repository).

Each PR passes `/pr-audit`.

**Final re-check after the build (T018, 2026-11-20)**: pass.
- **I.** Our own reader; SVG and DOT are open formats. The fixtures are
  made up, and the person's drawings were tried outside the repository.
- **II.** The result is an ordinary `{dot}` cell, read by `qd dot show` and
  edited by every `qd dot` command (tested). Nothing is kept outside the
  deck.
- **III.** A command with JSON, `--dry-run`, `--expect`, `qd schema`, the
  skill and the README.
- **IV.** It's an edit like any other; the render was checked at one unit to
  one pixel.
- **V.** One op and one undo step; nothing is written on any error (tested).
- **VI.** PR 2's tests were seen failing first. PR 1's unit tests were
  written with the code, as noted in tasks.md.
- **VII.** One module, `dot_import.rs`, reusing `dot.rs`'s writers and
  placing; one small crate (`roxmltree`). Along the way, `positions_from_svg`
  learned to read rounded boxes, which `qd dot pin` needed.
- **Licences.** `roxmltree` is MIT OR Apache-2.0, checked by the licence
  test.

## Complexity Tracking

| Addition | Why | Simpler option rejected because |
|---|---|---|
| `roxmltree` crate | nested groups and transforms need a real XML tree | regex reading breaks on hand-written files; a hand-made XML parser is more code to keep |
