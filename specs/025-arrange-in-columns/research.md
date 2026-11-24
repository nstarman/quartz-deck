# Research: Arrange elements inside columns

## R1 — Where an element sits

- **Decision**: `Deck::siblings(r) -> (&Slide, Option<&Block> parent, &[Block] list, usize index)`: walk the slide's tree to the block (`std::ptr::eq`, as `positional` does); its list is the parent's `children`, or `slide.blocks` at the top.
- **Rationale**: every op needs exactly this, and spec 024's tree has it; refs stay strings.

## R2 — Moves inside one list

- **Decision**: `rearrange(d, list, f)` takes the list instead of the slide; same-list `block move` keeps its code. `on_slide` becomes "index in `list`".
- **Rationale**: the tested top-level path is unchanged for top-level refs.

## R3 — Moves between lists

- **Decision**: two edits in one patch. **Cut**: the element plus the blank line after it (or before it, when it is last; for an only child, the element and its line ending, leaving `::: {.column}` / `:::`). **Insert**: before a target, `text + nl + nl` at its start; after, `nl + nl + text` at its end; into a column, after its last child, or after the opening fence line when it is empty. Refuse a target inside the moving element (`E_INVALID_ARG`). Moves between slides stay refused.
- **Rationale**: the cut and the insert never overlap once a target inside the element is refused, so `apply` takes them as they are; `keep_structure` checks the result.
- **Alternatives**: rewriting the whole columns block (touches more bytes, breaks FR-013).

## R4 — `--into`

- **Decision**: `qd block move X --into COLUMN` and `qd block add SLIDE --into COLUMN`; COLUMN must be a block with children or an empty `.column`/`.columns` div (spec 024's `opens_up`), else `E_INVALID_ARG`.

## R5 — The slide's last element and a column's last element

- **Decision**: top-level `block rm` of a slide's only element keeps today's `body` path. A column's only element is cut and the column stays (FR-005).

## R6 — Place freely from a column

- **Decision**: cut the element from its column and add it as a box (the same text `block_place` writes) after the slide's last top-level block, before notes, as `box_add` places boxes. The editor already sends the place and width it shows at.
- **Rationale**: Quarto positions `.absolute` against the slide, so the box lands where the editor measured it.

## R7 — A copied build comes last

- **Decision**: when `block dup` copies an element with a build, apply the duplicate, re-parse, then run `build_order` with the old order plus the copy; the patch replaces the slide's span with the result.
- **Rationale**: reuses `build_order`'s rules (indexes only when needed); only that slide changes.

## R8 — Editor

- **Decision**: drop `SLIDE_LEVEL` in `actions.js`; Move up/down name `up(ref)/i±1`; the hover bar and drag work for nested elements, with any `[data-qd]` of the same slide (first ref part) as a drop target and an empty column as an "into" target; an empty `.column[data-qd]` shows "+ Add content" by CSS `:empty::before` in the editor only, and a click opens the insert menu with `{ into: ref }`; `pasteAfter` already sends `after: ref`.

## R9 — Column width and number (P4)

- **Decision**: `qd block attr ELEMENT key=value… [--unset key]` through `with_attrs` (any element, not only columns); `qd column add COLUMNS` appends `::: {.column}` / `:::`; deleting a column is `block rm` of its ref. The Format view shows Width for a selected column.
