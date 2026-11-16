# Diagrams you edit on the slide: Graphviz cells in qd's editor

2026-11-15. From a brainstorm with the person who owns quartz-deck. A talk's
pipeline diagram (tool boxes and artifact boxes joined by right-angle
arrows, some boxes carrying the author's avatar and a link) is today a
hand-written SVG included as raw HTML. The person wants such diagrams to be
part of the slide, editable in qd's editor the way Keynote edits shapes and
connectors. Next step after approval: spec 015 through spec-kit
(specify → plan → tasks).

## Intent

A diagram on a slide is a Quarto `{dot}` cell (Graphviz), which Quarto
renders natively. qd's editor makes it editable by hand, on the slide:
- drag a box, and its arrows re-route at right angles;
- drag from one box to another to connect them;
- double-click to rename, add and delete boxes and arrows, change a box's
  style or link.

Every edit is a small change to the cell's text, so the deck stays plain
Quarto and renders without qd.

This is the first of three diagram projects. The others, later and each
brainstormed on its own, are:
- quarto-arrows (arrows drawn at coordinates) in the editor;
- Mermaid structural editing.

## What was checked (Quarto 1.11.9, 2026-11-15)

- **Graphviz is built into Quarto.** `{dot}` is rendered at render time to
  inline SVG, so the preview needs no code run.
- **Pinned positions hold.** `layout=neato` with `pos="x,y!"` keeps every
  node where it is pinned (in inches), and `splines=ortho` routes the
  edges as horizontal and vertical runs. A test render put four pinned
  nodes exactly at their positions, with axis-aligned edges.
- **The SVG names everything.** It labels each node `<g class="node"><title>NAME</title>`
  and each edge `<title>a->b</title>`. A node's `class="tool"` comes out
  as `class="node tool"`, and its `URL` as an `<a href>`.
- **Images don't render.** Quarto's sandboxed Graphviz can't load image
  files, so a node's image is silently dropped (no `<image>` in the SVG).
  The companion plugin below fills this gap.
- **Mermaid** is rendered in the browser by mermaid.js bundled with Quarto
  (offline). It has no syntax for positions. That's why diagrams with
  placed boxes are Graphviz.

## Goals

1. **Drag to place.** Dragging a node in the preview rewrites only that
   node's `pos`, snapped to a 0.25 in grid. The arrows re-route on the
   re-render after the drop.
2. **Keynote-style editing on the slide:**
   - select a node or an edge;
   - connect two nodes by dragging from a node's edge handle onto another
     node (dropping on empty space creates a new node there, already
     connected);
   - rename a node by double-clicking (this changes its `label`, never its
     name, so edges stay attached);
   - add a node (Insert ▸ Box);
   - delete a node, with its edges, or delete an edge;
   - flip an edge or turn its arrowhead off;
   - set a node's style, link and size in the Properties panel.
3. **Start a diagram.** Insert ▸ Diagram adds a `{dot}` cell set up for
   editing:
   - `layout=neato; splines=ortho;`
   - node and edge defaults;
   - two example nodes in the two starter styles.
4. **Make an existing `{dot}` cell editable.** "Pin layout" writes
   `layout=neato`, `splines=ortho` and every node's current position as
   `pos="x,y!"`, so a diagram Graphviz laid out by itself becomes one you
   place by hand.
5. **CLI first.** Every edit is a `qd dot …` command with JSON output.
   Agents and the editor use the same operations.
6. **Avatars and pictures in boxes** through a companion render plugin,
   `dot-image`. A node's standard Graphviz attribute `image="path"` shows
   the image inside the node's box in the rendered deck.
7. **Live routing as a setting** (off by default). While dragging, the
   arrows re-route on every move, laid out in the browser.

## Non-goals

- Mermaid editing, and quarto-arrows in the editor (later projects).
- Free-form drawing: curves, shapes other than Graphviz node shapes,
  freehand.
- Editing inside subgraphs or clusters. They are kept and shown, but only
  editable as text.
- Automatic conversion of existing SVG diagrams. A hand-made SVG is
  rebuilt by hand, or by an agent, as a `{dot}` cell.

## Anti-goals (a failure even if it works)

- A deck that renders only with qd. A `{dot}` cell must render with plain
  Quarto, and with `dot-image` installed when it uses `image=`.
- Losing the person's text. Comments, line breaks, attribute order and
  hand formatting outside the statement an edit touches must stay byte for
  byte (Principle II).
- Positions or diagram state kept anywhere but the `.qmd`.
- Live routing changing what's saved. It's a preview while dragging; the
  saved positions and the rendered deck are the same with it on or off.

## Constraints

- Principles II (minimal patches), III (CLI first, JSON, stable error
  codes), V (atomic, conflict-checked edits) and VII (no new crate unless
  needed; plugins are Quarto extensions first).
- `{dot}` stays an ordinary cell to qd's deck model. Diagram editing is a
  layer over its source.
- Offline: `dot-image` and the editor's diagram code are bundled. The live
  routing engine is downloaded once on enabling the setting, pinned by
  SHA-256 in qd's cache (as MathJax is today), and never fetched by a
  rendered deck.
- Licences: every browser build of Graphviz is Graphviz itself, under
  EPL-1.0, which isn't on qd's allow-list. *Decided by the person:* EPL-1.0
  is allowed for this one engine. qd downloads it on demand into its cache
  and doesn't build it in. EPL is compatible with private use (a weak
  copyleft that applies only to redistributed changes of Graphviz). The
  plan adds it to the licence check as a downloaded asset, with its
  licence, as MathJax is listed.

## Approaches considered

**Where DOT editing lives**
- **A: in qd's Rust core, as commands (chosen).** A small span-keeping DOT
  reader and `qd dot …` operations produce minimal patches. The editor's
  overlay on the preview SVG calls them. Agents get every edit, undo works,
  and formatting survives.
- **B: in the editor's JavaScript.** It edits the DOT text and saves the
  whole cell. Less code, but agents can't connect or move. Rejected (III).
- Existing Rust DOT crates (such as graphviz-rust) parse to a tree without
  source positions or comments, so they can't make minimal patches. Hence
  our own reader.

**Feedback while dragging**
- **(i) Re-render on drop (default).** While dragging, the node moves and
  its edges show as provisional straight lines. On release qd saves, and
  the routed edges appear after the preview re-renders (about 1–2 s today).
- **(ii) Live routing (setting).** A browser Graphviz lays the cell out on
  each move and draws the routed edges. On release, the same save and
  re-render as (i). Several MB, so it's downloaded on enabling, not
  shipped.

**Pictures in boxes**
- **1: companion render plugin `dot-image` (chosen).** The source keeps
  Graphviz's own `image=`; the plugin makes it render under Quarto.
- 2: no pictures in diagrams. Rejected: the person wants the avatars.
- 3: a separate positioned image on top of the box. Rejected: it wouldn't
  move with the box.

## Design

### The DOT reader (Rust, `crates/qd/src/dot.rs`)

It reads one cell's source into statements, each with its byte span:
- `graph` or `digraph` (`strict` allowed), with the graph's ID;
- graph-level `key=value` and `graph|node|edge [attrs]` statements;
- node statements `NAME [attrs]`;
- edge statements `a -> b [attrs]`, including chains `a -> b -> c`;
- subgraphs and clusters, kept and read-only;
- each attribute's key and value with spans, covering quoted strings,
  HTML labels `<…>` and bare IDs;
- comments (`//`, `/* */`, `#` lines), skipped but kept.

Anything outside this subset makes the cell read-only for diagram editing,
with a reason (`E_UNSAFE_EDIT`), never a wrong patch.

**Edits are minimal patches:**
- **Set an attribute:** replace its value in place, or add it to the
  statement's attribute list (creating `[ … ]` when there is none).
- **Add a node or edge:** a new line before the closing `}`, indented like
  the statements around it.
- **Remove:** delete the statement's lines. Removing one edge of a chain
  `a -> b -> c` splits the chain into the edges that remain.
- **Names:** new nodes get fresh IDs (`n1`, `n2`, …) unused in the cell.
  Labels are separate from names.

### Commands (`qd dot …`, all edits: `--dry-run`, `--expect`, undo)

| Command | Does |
|---|---|
| `qd dot show CELL` | nodes (name, label, class, URL, image, pos, size), edges, graph settings; whether the cell is editable, and why not |
| `qd dot move CELL NODE --at X,Y` | sets `pos="X,Y!"` (inches, snapped unless `--no-snap`) |
| `qd dot add CELL [--label L] [--at X,Y] [--class C] [--from NODE] [--to NODE]` | a new node, optionally connected |
| `qd dot connect CELL A B` | adds `A -> B` |
| `qd dot rename CELL NODE LABEL` | sets `label` |
| `qd dot set CELL NODE\|A->B KEY=VALUE…` | any attributes (`URL`, `image`, `width`, `arrowhead`, `dir`, …); `--style-of OTHER` copies another node's style attributes |
| `qd dot rm CELL NODE\|A->B` | removes a node (with its edges) or an edge |
| `qd dot flip CELL A->B` | reverses an edge |
| `qd dot pin CELL [--positions JSON]` | `layout=neato`, `splines=ortho`, every node pinned at its current position |
| `qd dot new --after REF` | inserts a starter diagram cell |

`CELL` is a block ref, as elsewhere. Several edges between the same two
nodes are addressed as `A->B#2`.

`pin` needs the current layout. The editor passes the positions it read
from the preview SVG (`--positions`). From the CLI alone, qd takes them
from the last preview render in its cache, else from `dot -Tjson` if a
Graphviz is on PATH, else fails with a hint (`E_NOT_FOUND`).

### Styles *(my call)*

A box's style is a set of Graphviz attributes written on the node itself:
`shape`, `style` (`rounded,filled`), `fillcolor`, `color`, `fontcolor`,
`fontname`, `penwidth`. So a diagram looks the same everywhere, PDF
included, with no stylesheet to install.
- "Style" in the Properties panel lists the looks already used in the cell
  (grouped by those attributes) plus two starter ones (a dark "tool" and a
  light "artifact").
- Applying a style sets those attributes on the node (`qd dot set …
  --style-of OTHER` copies another node's).
- `class=` stays available as a free hook for the deck's own CSS, but
  nothing in qd depends on it.

### Sides and labels *(decided by the person: try it)*

With `splines=ortho`, Graphviz picks the side where each arrow meets a box,
and ignores ports; it handles edge labels poorly. So:
- the four connect handles are only places to start a drag; they don't set
  ports;
- this version writes no edge labels.

This is checked on the proving case, the pipeline diagram. If it needs
arrows from fixed sides, or labels, that becomes a design change (routing
hints, or another layout), raised then.

### Positions: from the SVG to `pos`

Graphviz's SVG works in points (1/72 in), with y growing upward inside a
`translate(…)` transform. The editor converts a dragged node's SVG centre
to inches with y flipped, and qd snaps it to 0.25 in. The browser side
never writes text; it sends inches to `qd dot move`.

### The editor

- **Overlay:** in the preview, every node of an editable `{dot}` cell is
  hit-testable through its `<title>`. Selecting it shows handles: four
  connect handles at the edges, and a resize corner (`width`/`height`,
  `fixedsize`).
- **Dragging:** the node's SVG group moves, its edges become provisional
  straight lines, and on drop `qd dot move` runs and the preview
  re-renders. Escape cancels.
- **Connecting:** drag from a connect handle. A line follows the pointer.
  Dropping on a node runs `connect`; dropping on empty space runs `add
  --from` at that point.
- **Double-click** edits the label in place; Enter runs `rename`.
- **Edges:** clicking one selects it. Delete removes it; the Properties
  panel has flip and the arrowhead.
- **Properties panel:** for a node, label, style (see Styles), link, image
  and size; for the graph, the "Pin layout" button when the cell isn't
  pinned.
- **Menus:** Insert ▸ Diagram and Insert ▸ Box (inside a selected
  diagram), in `actions.json` like every action.
- **Not pinned:** an unpinned `{dot}` cell (no `neato` + `pos`) can be
  selected and edited structurally. Dragging a node first offers "Pin
  layout".

### Live routing (setting `diagrams.live-layout`, off)

- **The engine:** the Graphviz build from the person's own Quarto install
  (Quarto ships hpcc-js/wasm), served by qd, so live routes match the
  render. If it can't be loaded, a pinned `@hpcc-js/wasm-graphviz` is
  downloaded into qd's asset cache, verified by SHA-256, the way MathJax
  is.
- **While dragging:** the editor lays out the cell's current text, with
  the dragged node's provisional `pos`, at most once per animation frame,
  and draws the routed edges in place of the straight ones.
- **On drop:** the same save and preview re-render as without it.
- **Fallback:** if the engine is missing or fails, the editor falls back
  to straight lines silently. The setting's description says the routes
  may differ slightly from Quarto's own Graphviz version.

### `dot-image` (render plugin, `plugins/dot-image/`)

A Quarto filter extension. qd carries it and `qd dot` installs it, the
same way as cite-links.
*Revised in planning (research R1, R5): Quarto runs Graphviz before any
filter sees the cell, so nothing can strip `image=` beforehand. That isn't
needed, because Graphviz ignores it silently.*
1. **Finding the images:** a `post-quarto` filter reads the deck's own
   source (`quarto.doc.input_file`). It records node → image path for
   each `{dot}` cell, and pairs cells with their rendered SVGs in
   document order.
2. **Drawing them:** it inserts an `<image class="dot-image">` into each
   recorded node's group, at the node's left. CSS can round it
   (`clip-path: circle()`), as on today's diagram.

**Room for the image** is made in the source. `qd dot set … image=PATH`
also writes the label as an HTML-like label with a blank leading cell the
size of the image. `rename` edits only the text cell, and clearing the
image restores a plain label. The test checks that the image and the label
text don't overlap.

The image is a plain file next to the deck, so `qd export` inlines it as
it does other images, and offline decks keep it.

## Order of work (each its own PR)

1. **DOT reader and `qd dot show`.** The round-trip test (parse, then a
   no-op edit, gives identical bytes) runs over a corpus of real-world DOT.
2. **The edit commands** (`move`, `add`, `connect`, `rename`, `set`, `rm`,
   `flip`, `pin`, `new`).
3. **Editor: select, drag to move with re-render on drop, Properties.**
4. **Editor: connect, add, rename in place, delete, Insert menus.**
5. **`dot-image` plugin**, with an example deck and render tests.
6. **Live routing setting.**

Then, with the person: the talk's pipeline diagram rebuilt as a `{dot}`
cell, as the proving case.

## Left to the builder

- The browser Graphviz build (for example viz.js or hpcc-js/wasm), after
  checking its size and that its wrapper's licence is allowed; the Graphviz
  inside is EPL-1.0, allowed as above.
- `dot-image`'s exact padding mechanism, and the image's default size and
  inset.
- How handles and provisional lines look; they follow the editor's
  existing selection style.
- The DOT corpus for the round-trip test (Graphviz's own gallery examples
  under a permissive licence, or written for the test).
