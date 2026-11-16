# Research: Diagrams you edit on the slide (015)

Checked on this machine with Quarto 1.11.9, 2026-11-15. Spikes ran in the
session scratchpad, and none of their code is kept.

## R1. How Quarto renders `{dot}`

**Measured:**
- Quarto renders a `{dot}` cell to inline SVG while rendering, with the
  Graphviz build it ships (`share/js/graphviz-wasm.js` and
  `share/wasm/graphvizlib.wasm`, from hpcc-js/wasm). It runs before any
  Lua filter entry point: at `pre-ast` and at `pre-quarto`, the cell is
  already a `Div.cell` holding a `RawBlock` with the `<svg>`. No filter
  ever sees the DOT source as a code block.
- With `layout=neato`, nodes with `pos="x,y!"` stay pinned, and
  `splines=ortho` routes edges as axis-aligned runs.
- The SVG's graph group is
  `<g id="graph0" class="graph" transform="scale(1 1) rotate(0) translate(4 H)">`.
  Nodes are `<g class="node …"><title>NAME</title>…`, and edges are
  `<g class="edge"><title>a&#45;&gt;b</title>…`.
- A node's `class=` becomes `class="node <class>"`; its `URL` becomes an
  `<a xlink:href>`.
- `image="…"` is ignored silently, whether the file exists or not: no
  `<image>`, no error.
- Quarto sets the `<svg>`'s `width`/`height` to fill the space and keeps
  the `viewBox` in points.

**Decision:** qd edits the cell's source; Quarto renders it unchanged. Nothing
in qd needs Graphviz at render time.

## R2. From a drag on the SVG to `pos`

**Measured:** nodes pinned at (0,0) and (2,1) in inches render with centres
(27,−18) and (171,−90) in the graph group's units. That's a 144-unit and
72-unit difference: 72 points per inch, with y flipped.

**Decision:** a drag moves a node by a **delta**, never to an absolute SVG
point. The neato output can shift the whole graph, so absolute frames
don't line up with `pos`.
- The editor converts the pointer's screen movement into the graph group's
  units with `getScreenCTM().inverse()`.
- Then `dx_in = dx/72` and `dy_in = −dy/72`.
- It sends `qd dot move CELL NODE --by DX,DY`.
- qd adds the delta to the node's current `pos` and snaps the result to
  0.25 in.

`--at X,Y` stays for agents. For `pin`, any consistent frame works,
because only relative positions matter: centres in points divided by 72,
y negated, offset so the lowest node sits at y = 0.

## R3. Finding the diagram in the preview

**Measured:** the preview copy wraps every cell in
`::: {.qd-cell data-qd="<slide>/<i>" data-qd-lang="<lang>"}`, so the
rendered `<svg>` is inside `div[data-qd][data-qd-lang=dot]`.

**Decision:** the overlay is a new editor module, `src/ui/dot.js`.
- It finds editable diagrams by `data-qd-lang="dot"`, asking qd
  (`qd dot show`, through the existing op route) whether each is
  editable.
- It hit-tests nodes and edges by their `<g class="node|edge">`, and reads
  names from `<title>`.
- Repeated edges between the same two nodes are told apart by their order
  among edges with the same title, matching `A->B#k` in the source order.

## R4. The DOT reader

**Decision:** our own small reader, `crates/qd/src/dot.rs`. It's a
hand-written tokenizer plus a recursive-descent parser over the DOT
grammar.
- **Tokens:** IDs, quoted strings with escapes, HTML strings `<…>`
  (nested angle brackets), numbers, `{ } [ ] ; , = : -> --`, and comments
  `//`, `/* */` and `#` at the start of a line, kept as trivia.
- **Statements:** `strict? (graph|digraph) ID? { … }`; `ID = ID`;
  `graph|node|edge [ a_list ]`; node statements; edge statements with
  chains; subgraphs.
- **Spans:** every statement and every attribute carries its byte span.
  Ports (`a:e`) are parsed and kept.
- **Subgraphs** are parsed but marked read-only.
- **Read-only cells:** anything outside the grammar, or a cell whose body
  isn't one graph, is reported read-only with the token where it stopped
  (`E_UNSAFE_EDIT`).

**Rejected:**
- graphviz-rust and dot-parser parse to trees without spans or comments,
  so they can't make minimal patches;
- a Graphviz binary would add a runtime dependency only for reading text.

## R5. Images in boxes (`dot-image`)

**Measured (R1):** no filter can strip `image=` before Graphviz, and none
needs to, because Graphviz ignores it.

**Decision:** `plugins/dot-image/` is a `post-quarto` filter.
1. **Finding the images:** it reads the deck's own source
   (`quarto.doc.input_file`), finds each `{dot}` cell, and records
   node → `image` path with a small Lua scan. That scan reads node
   statements only; edge attributes carry no images.
2. **Matching cells:** it pairs cells with the rendered SVGs in document
   order, as RawBlocks containing `<svg` inside `Div.cell`.
3. **Inserting:** for each recorded node, it inserts
   `<image class="dot-image" href="PATH" x y width height>` into the
   node's `<g>`. The image goes at the node's left, square, with the
   node's height minus 2 × 4 pt.
4. **Missing files:** a missing file is skipped, with a warning naming it.

**Room for the image:** Graphviz can't know the image is there, so the
space is made in the source.
- `qd dot set CELL NODE image=PATH` (and the Properties field) also writes
  the label as an HTML-like label with a leading spacer cell:
  `label=<<TABLE BORDER="0" CELLPADDING="0"><TR><TD WIDTH="W" HEIGHT="H" FIXEDSIZE="TRUE"></TD><TD>Text</TD></TR></TABLE>>`.
  W and H come from the node's height. `rename` edits only the text cell.
- Clearing the image restores a plain label.
- The plugin places the image over the spacer: the left of the node,
  inset by the node's margin.

**Limitation:** cells pulled in with `{{< include >}}` aren't scanned, and
the plugin warns once.

## R6. Live routing's engine

**Measured:** Quarto 1.11.9 ships `share/js/graphviz-wasm.js` (hpcc-js/wasm)
and `share/wasm/graphvizlib.wasm` (930 KB). On npm,
`@hpcc-js/wasm-graphviz` 1.30.0 is Apache-2.0 and 2.08 MB unpacked;
`@viz-js/viz` 3.31.0 is MIT and 4.99 MB. Both wrap Graphviz itself
(EPL-1.0).

**Decision:**
- **The engine:** the editor uses the Graphviz build from the person's
  own Quarto install, served by qd from the path `quarto` resolves to.
  It's the same build that renders the deck, so live routes match the
  render, and nothing is downloaded. This is the pattern qd already uses
  for plotly.js, which it takes from the person's own Python.
- **Fallback:** if that file isn't found or doesn't load in the browser,
  qd downloads a pinned `@hpcc-js/wasm-graphviz` into its asset cache,
  verified by SHA-256, as for MathJax.
- **Licences:** EPL-1.0 is allowed for this engine (the person's
  decision). The Apache-2.0 wrapper is listed beside it in the licence
  test's downloaded-assets list.
- **While dragging:** the editor lays out the cell's text with the
  dragged node's provisional `pos`, at most once per animation frame,
  off the main path, and swaps in the routed edges. On drop, the normal
  move and re-render.
- **Check:** a spike confirms that Quarto's `graphviz-wasm.js` loads as a
  browser ES module. If it doesn't, the fallback becomes the main path.

**Spike result (T024, 2026-11-16):** Quarto 1.11.9's `graphviz-wasm.js`
imports in a browser, but its loader was edited by hand for Deno: laying
out calls `graphvizWasmModule()`, which reads the wasm with
`Deno.readFileSync`, and fails with "Deno is not defined". So the fallback
is the main path: `@hpcc-js/wasm-graphviz` 1.30.0, `dist/index.js`
(820 KB, one ES module with the wasm inlined, Apache-2.0 around Graphviz's
EPL-1.0), SHA-256
`77d45a274928484dd1daccd55ab3e41ddda5b4262e29240e205f7188839d9d5d`.
Entry point: `(await Graphviz.load()).layout(text, 'svg', 'neato')`,
synchronous after the load.

## R7. Patching

**Decision:**
- **Attribute set:** replace the value's span, quoting as needed (IDs
  stay bare when they're valid IDs). Otherwise, insert `, key=value`
  before the list's `]`, or ` [key=value]` before the statement's
  terminator.
- **Add:** a new line before the graph's closing `}`, with the
  indentation of the last statement, and a `;` when the surrounding
  statements use one.
- **Remove:** delete the statement's full lines when it stands alone on
  them; otherwise delete its span and one adjacent separator.
- **Chains:** removing `b -> c` from `a -> b -> c -> d` rewrites that
  statement as `a -> b` and `c -> d`, on two lines, keeping its
  attribute list on both.
- **Names:** new nodes are `n1`, `n2`, … (the first unused).
- **Positions:** written as `pos="X,Y!"` with at most two decimals.

## R8. Undo and conflicts

**Decision:** every `qd dot` edit is an ordinary deck edit
(`ops::Patch` → `Store::write`), so `--expect`, `--dry-run`, undo and
Save-as-git-commit work unchanged. The editor sends the deck version it
rendered, and a drop over a changed cell is refused (`E_CONFLICT`).

## R9. The setting

**Decision:** `diagrams.live-layout`, a boolean, default `false`, in the
settings registry (`settings.rs`), shown in Settings under "Diagrams".

## R10. Licences

- `dot.rs`, `dot.js` and the `dot-image` filter are our own code; no new
  crates.
- Downloaded or served assets (live routing): Graphviz (EPL-1.0) inside
  hpcc-js/wasm (Apache-2.0). They're listed in the licence test's asset
  list with their licences; EPL-1.0 is allowed for this asset only.

## R11. The constitution

No amendment needed. These are editor features (VII), a render plugin
(1.4.0's render plugins), and an optional asset like MathJax.
