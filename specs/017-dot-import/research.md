# Research: Import a drawing as a diagram

## R1. Reading SVG

**Decision**: parse the file with `roxmltree`. It is a read-only XML tree, has
no dependencies, and is MIT OR Apache-2.0, which the licence test checks. qd
walks the tree itself:
- the elements it knows: `rect`, `circle`, `ellipse`, `text`/`tspan`,
  `line`, `polyline`, `path`, `a`, `image`, `g`, `svg`;
- each element's `transform` and its groups', composed;
- `fill`, `stroke` and `marker-*`, as attributes or in `style`.

**Rationale**: drawings nest groups and transforms, so the tree has to be
read properly. A regex reading (as `positions_from_svg` does for Graphviz's
regular output) breaks on hand-written files. Constitution VII prefers a
small, well-kept crate over hand-written parsing.

**Alternatives**:
- **`quick-xml`:** a streaming reader, so qd would build the tree itself.
  More code. Rejected.
- **`usvg`:** renders and resolves all of SVG, but brings a large tree of
  crates for what is mostly unneeded. Rejected.

## R2. Geometry

**Decision**:
- **Transforms:** a 2×3 matrix per element, the product of its ancestors'
  `transform`s and its own (`translate`, `scale`, `rotate`, `matrix`,
  `skewX/Y`). Shapes are placed by their transformed bounding box.
- **Boxes:** `rect` (with `rx`: rounded), `circle` and `ellipse` (spec,
  the person's choice).
- **Line ends:** taken from `line` (x1, y1, x2, y2), `polyline` (first and last
  points) and `path` (the `d` commands M L H V C S Q T A Z, absolute and
  relative). Only the first and last points and the direction at each end
  are kept; bends aren't (spec, the person's choice).
- **Units:** drawing units are taken as points (72 per inch), as Graphviz
  does (spec Assumptions). A box's centre becomes `pos="x/72,-y/72!"` (y
  up), normalised so the lowest, leftmost box is at 0,0, as `dot pin` does
  (`normalised`, reused). Its size becomes `width`/`height` in inches with
  `fixedsize=true`, so the box keeps the drawn size.

## R3. Which text is whose label

**Decision**:
- **Into boxes:** a `text` belongs to the smallest box that contains its
  anchor point (after transforms). Several texts in one box are joined top
  to bottom with line breaks (`\n`).
- **Left out:** text in no box is reported (`"text outside any box"`).
- **Graphviz SVGs:** each node's own `text` is its label.

## R4. Which boxes an arrow joins

**Decision**: for each end of a line:
- **Its box:** the box whose edge is nearest, if within 8 drawing units.
  Otherwise, the first box the line meets when extended from that end by up
  to 24 units, along the direction at that end.
- **No box:** the line is left out, and the report says which end found
  nothing.
- **Direction:** from the markers. `marker-end` only gives an arrow from
  start to end. `marker-start` only reverses it. Both give `dir=both`, and
  neither gives `arrowhead=none`.
- **Same box at both ends:** left out ("joins a box to itself").

**Rationale**: the spec's "near a box". 8 and 24 units are 1/9 and 1/3 of an
inch: well inside the gap between boxes in a slide diagram, and wide enough
for a hand-placed arrowhead that stops short of the box. They are two
constants, to tune on the proving case (SC-005).

## R5. Looks, links and pictures

**Decision**:
- **Fill:** gives `fillcolor` with `style=filled`. A rounded `rect` gives
  `style="rounded,filled"`, as qd's starter looks do.
- **Stroke:** gives `color`. The box's text `fill` gives `fontcolor`.
  Colours are written as given (named, `#rgb`, `#rrggbb`).
- **Font size:** a text's `font-size` gives `fontsize` in points.
- **Links:** an `a` around the box or its text gives `URL` (`href` or
  `xlink:href`).
- **Pictures:** an `image` inside a box gives `image="<href>"` and the
  spacer label (`spacer`, reused), so the picture appears through
  `dot-image`. As with `qd dot set image`, the deck gets `filters: - dot-image`
  and the extension (`plugins::with_filters`, reused). A missing file is
  imported with a note in the report (spec edge case).

## R6. The cell, and its size on the slide

**Decision**:
- **The cell:** a `{dot}` cell with `layout=neato; splines=ortho;`, one
  statement per box in drawing order (`node_stmt`, reused), then one per
  arrow.
- **Its size:** the cell options `#| fig-width` and `#| fig-height` are set
  to the drawing's width and height ÷ 96. Quarto sizes a `{dot}` figure's
  `<svg>` from those (in inches, at 96 px each: seen in spec 020's spike,
  where the defaults 10 × 5 gave 960 × 480). So the diagram shows on the
  slide at the drawing's own size.

## R7. Graphviz SVGs

**Decision**:
- **Recognised by** its root `<g class="graph">`.
- **Boxes:** each `g.node` is a box named by its `<title>` (unescaped,
  `&#45;` → `-`), at its shape's centre, with its shape kind (`polygon`
  gives `box`, `ellipse` gives `ellipse`), label, colours and link.
- **Arrows:** each `g.edge` is an arrow from its `<title>` (`a->b`, or
  `a--b` undirected), ports dropped.
- **Kept exactly:** names and arrows (SC-004). Other attributes Graphviz
  doesn't write into its SVG can't come back (fontname, penwidth), and the
  report says the drawing was a Graphviz render.

## R8. Where the cell goes

**Decision**:
- **`--on`, `--after`, `--before`:** `ops::block_add`, as `qd dot new` does.
- **`--replace REF`:** `ops::edit(REF, cell)`, so the element becomes the
  cell in one patch.
- **With a picture:** the `filters:` change is folded into the same patch, as
  `dot::set` does for `image=`.

All of them are one op with `--expect`, `--dry-run` and undo, as for every
edit.

## R9. Tests without anyone's drawings

**Decision**: made-up fixtures in `crates/qd/tests/fixtures/dot-import/`:
- `hand.svg`: four boxes (one rounded, one ellipse), three arrows (one
  bent, one double-headed), a link, a picture, a group with a `transform`,
  a stray line and a stray text.
- `graphviz.svg`: rendered once from a made-up DOT with Quarto's Graphviz,
  and committed.

The person's own diagram (SC-005) is tried outside the repository, as in
spec 015.

## Licences

- `roxmltree`: MIT OR Apache-2.0. Listed in plan.md, and checked by
  `dependency_licenses_are_allowed`.
