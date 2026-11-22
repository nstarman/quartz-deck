# Research: Diagram source follows the cursor

## R1. Which statement holds the cursor: ask the server's parser

**Decision**: a new read-only op, `qd dot at`, takes a `{dot}` cell's text
as the person has it (saved or not) and a byte offset in it. It answers
with the statement there: a box, an arrow (and which link of a chain), a
cluster, a whole-diagram line, or nothing. The editor sends the text it
has, so the answer follows typing that isn't saved yet.

**Why**: `dot.rs` already reads DOT keeping every statement's span
(`Stmt.span`, `Operand::Node.span`), and it already knows names, ports,
chains and the `A->B#2` arrow refs that the editor's `edgeNamed` uses.
A second DOT reader in JavaScript would have to agree with it on quoting,
HTML labels, comments, ports and subgraphs.

**Cost**: one local request per cursor stop. `dot show` on a 20-box cell
takes 2–12 ms from the editor (spec 020 audit), so this is within SC-002's
100 ms.

**Alternatives**:
- A JavaScript statement splitter in the editor: no round trip, but a
  second parser to keep in step.
- Statement spans in `dot show`: these refer to the saved text, so they
  go wrong as soon as the person types.

## R2. Text that doesn't parse mid-typing

**Decision**: `qd dot at` parses the whole cell, so an unclosed quote or
bracket anywhere stops it. It then answers `{"ok": true, "at": null,
"stopped": {reason, at}}`. The editor keeps the last outline and the last
preview, marks the preview as out of date (FR-007), and shows no error
over the text.

**Why**: the reader stops at the first error and has no partial result.
A cell is typed in for a few seconds at a time, so keeping the last good
answer is enough.

## R3. The preview: one element drawn alone by the in-page Graphviz

**Decision**: `qd dot at` also returns `alone`, a small DOT graph that
holds:
- the cell's graph settings, except layout, positions and size;
- its `node` and `edge` defaults;
- the box's merged statements, or for an arrow, its statement and its two
  ends' boxes.

The editor lays that out with the Graphviz the page already loads for
spec 020 (`dot.js` `graphviz()`), adds pictures the way `pictures()` does,
and shows the SVG in a small pop-over beside the line.

**Why**: the same engine and the same defaults as the slide, so SC-004
holds. The server already has the helpers (`node_stmt`, attribute
merging) to write a statement. Drawing a lone box takes 2–6 ms.

**Alternative**: crop the box out of the slide's drawing. That is cheap,
but it can't show a change before it is saved, which is the point of
Story 2.

## R4. The outline on the slide

**Decision**: a class, `qd-dot-cursor`, on the `<g class="node">`,
`<g class="edge">` or `<g class="cluster">` of the cell's SVG in the
preview. It is found with `nodeNamed` and `edgeNamed`, which `dot.js`
already uses for selection, and a cluster by its `<title>`. Its look is a
dashed accent outline, so it differs from the selection's solid one.

**Why**: it reuses the existing name to SVG mapping. A class touches
neither the selection (FR-003) nor the file. A preview reload drops it,
and the next caret report puts it back.

## R5. From the source pane's cursor to a cell and an offset

**Decision**: the pane already reports the caret's line to the editor
(`caretMoved` → `onCaret`). The editor goes further only when the caret is
inside a ```` ```{dot} ```` fence of the pane's own text. It finds the fence
in `ta.value`, takes the body and the caret's offset in it, and names the
cell by the deck block whose lines contain the fence (spec 005's
`sourceFocus` lookup).

**Hover**: a `mousemove` over the textarea, resting for 300 ms, gives a
line from the pointer's y and the highlight layer's per-line elements
(#122), and a column from the monospace character width. It then goes
through the same path. `mouseleave` gives the outline back to the caret.

**Debounce**: 60 ms on caret and typing, so a held arrow key asks once
when it stops.

## R6. The cell editor on the slide (Story 3)

**Decision**: the same follow path, fed by `#srcedit`'s textarea. Its text
is the whole cell, fences included. The pop-over sits beside that textarea.

## R7. A setting for the preview

**Decision**: `diagrams.source-preview`, default `true`, in `qd settings`
like `diagrams.live-layout`. Off hides the pop-over; the outline stays
(FR-010).
