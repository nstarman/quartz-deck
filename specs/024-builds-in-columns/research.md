# Research: Builds inside columns

## R1 — Which divs get children

- **Decision**: a div whose first class (braced or shorthand) is `columns`
  or `column`. Its inner lines (between the opening and closing fences)
  are scanned by the same `scan` the slide uses.
- **Rationale**: qd wraps a list in `::: {.fragment}` (or `::: {.incremental}`)
  to give it a build. If every div opened up, a click would land on the
  list inside, whose build lives on the wrapper; the existing top-level
  builds would break. Columns are the case the person hit.
- **Alternatives**: every div (breaks wrappers, above); any div with a
  class from a list in settings (configuration nobody asked for).

## R2 — Ref format

- **Decision**: a child's auto id is its parent's id plus `/i`, so
  `results/1/0/2`. `Deck::element` splits the ref at the first `/`
  into slide ref and index path, and walks `blocks` then `children`.
  A child with `{#id}` keeps that id, and `#id` / `id` search every depth.
- **Rationale**: top-level refs stay exactly as they are; the path reads
  like the structure. Slide ids never contain `/` (Pandoc's auto ids drop
  it, and `@N` has none), so the first `/` is the split. Today's
  `rsplit_once` gives the same answer for one level.
- **Alternatives**: a flat numbering of all elements on a slide
  (`results/5`): renumbers the top level, breaking every existing ref
  and test. Column-relative names (`results/1/left/0`): three-column
  decks have no left/right.

## R3 — Which ops take a nested ref

- **Decision**: take it: `build set`, `build list`, `build order`,
  `block class` (through `with_attrs`), `edit`, `show`, focus `--set`.
  Refuse with `E_INVALID_ARG` "… is inside columns: … it in the source
  for now": `block move`, `block dup`, `block rm`, `block add` with
  `--before/--after` a nested ref, `block place`, `block unplace`, the
  `box` ops, `restore --element`, `run` of a cell.
- **Rationale**: the taken ops replace one span and re-parse
  (`keep_structure`), which holds for a child. The refused ones splice the
  slide's top-level list and compute neighbours there.
- **How**: one helper `top_element(d, r)` that calls `element` and refuses
  a block not in `slide.blocks`; the refused ops call it instead of
  `element`.

## R4 — Builds across depths

- **Decision**: `build_list` walks the slide's blocks in pre-order
  (parent before children, in source order) and sorts as today; `build
  order`'s "named in source order" check uses the same walk.
- **Rationale**: reveal.js orders fragments by `data-fragment-index`, then
  by their order in the page, which is pre-order.

## R5 — Preview tagging

- **Decision**: `preview_copy`'s per-block loop becomes a function of a
  block list that recurses into a block's children after tagging the
  block's own fence. Children of kind `Cell` are copied verbatim, as
  the whole columns block is today.
- **Rationale**: the tag of a div goes on its opening fence
  (`insert_attr`), so its inner lines can be emitted child by child.
  Kernel runs and stored outputs only know top-level cells, so tagging a
  nested cell would show an empty placeholder.

## R6 — Editor selection

- **Decision**: `closest('[data-qd]')` already returns the innermost
  tagged element. `elementTarget(ref)` splits at the first `/` and walks
  `blocks`/`children` of the fetched slide; it adds `nested: true` and
  `parent` (the ref without its last part). The `deselect` key (Escape)
  selects `parent` when there is one. Element actions that the CLI
  refuses get `enabled: !t.nested` and a `why`.
- **Rationale**: smallest change to selection; the outline, Format view
  and Animate view key off `state.sel` and need nothing new.
