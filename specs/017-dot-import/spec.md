# Feature Specification: Import a drawing as a diagram

**Feature Branch**: `017-dot-import`

**Created**: 2026-11-17

**Status**: Draft

**Input**: User description: "qd dot import (spec 017): turn a drawing of boxes and arrows, given as an SVG file, into a pinned `{dot}` diagram cell on a slide (spec 015), so a hand-drawn diagram becomes one the person can edit on the slide. CLI only. Read hand-written SVG and SVG that Graphviz produced; arrows connect to the nearest boxes; anything that can't be placed is reported and left out."

## Context

Spec 015 made `{dot}` diagrams editable on the slide. Its proving case
was a talk's diagram drawn by hand as SVG: rectangles with text, lines
with arrowheads, links and pictures. Rebuilding it as a `{dot}` cell meant
reading every coordinate, colour and label off the SVG and writing it out
by hand. It worked, and nothing important was lost, but it took a long
time. A person with more such drawings, or an agent helping them, should
be able to do it in one command, and then refine the result on the slide.

Decisions made with the person: read two kinds of SVG, hand-written SVG
and SVG that Graphviz produced; connect arrows to the nearest boxes their
ends touch or point at; offer it as a command only, not an editor action.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — A hand-drawn diagram becomes an editable one (Priority: P1)

The presenter has a diagram drawn as SVG: boxes with labels, arrows
between them, some boxes linking somewhere, some with a small picture.
They (or an agent) run `qd dot import drawing.svg --on SLIDE`. A pinned
diagram cell appears on the slide with one box for each box in the
drawing, in the same places, sizes and colours, with the same labels,
links and pictures, and one arrow for each line between two boxes. They
can now drag boxes, connect and restyle them on the slide as with any
diagram.

**Why this priority**: it is the feature: the hand transcription it
replaces is what took the time.

**Independent Test**: import a made-up hand-drawn SVG of four boxes,
three arrows, one link and one picture; the new cell lists four boxes and
three arrows with the drawing's labels, link and picture; rendered, each
box sits within a few pixels of where it was in the drawing.

**Acceptance Scenarios**:

1. **Given** an SVG with boxes (rectangles with text inside), **When** it
   is imported, **Then** each becomes a box in a new pinned diagram cell,
   at the same place and size relative to the drawing, with its label.
2. **Given** lines between boxes, **When** imported, **Then** each becomes
   an arrow from the box nearest its start to the box nearest its end,
   pointing the way the drawn arrowhead points (or both ways, or none).
3. **Given** a box's fill, outline and text colours, a link around it, or
   a picture inside it, **When** imported, **Then** the box carries the
   same look, link and picture (the picture as in spec 015's pictures in
   boxes).
4. **Given** a line whose ends are far from any box, or text not inside a
   box, **When** imported, **Then** it is left out and listed in the
   command's report with the reason, and nothing else is affected.
5. **Given** `--replace REF` on the slide element holding the drawing
   (an image of the SVG, or the SVG as raw HTML), **When** imported,
   **Then** that element is replaced by the diagram in one edit, and one
   undo puts it back.

---

### User Story 2 — A Graphviz drawing comes back as a diagram (Priority: P2)

The presenter has an SVG that Graphviz drew earlier, for example from an
old render or another tool, but not its DOT source. Importing it gives a
pinned diagram with the original box names, labels and arrows, placed as
they were drawn.

**Why this priority**: less common than hand drawings, but Graphviz names
every box and arrow in its output, so it can be read back exactly.

**Independent Test**: render a made-up `{dot}` diagram with Quarto, import
the SVG; the new cell has the same box names, labels and arrows, with
positions matching the rendered ones.

**Acceptance Scenarios**:

1. **Given** an SVG produced by Graphviz, **When** imported, **Then** its
   boxes keep their names and labels, its arrows join the same boxes, and
   each box is pinned where it was drawn.

---

### Edge Cases

- **Nested groups and transforms**: shapes inside groups that move or
  scale them are placed where they appear, not where their own
  coordinates say.
- **Text over several lines** (several text pieces stacked in a box)
  becomes one label with line breaks.
- **Two boxes overlap**: each is imported; an arrow's end goes to the box
  whose edge it is nearest.
- **Round shapes**: a circle or ellipse with text inside becomes a box of
  that shape (`shape=circle` or `shape=ellipse`), and arrows connect to it
  like any box.
- **An arrow drawn as a filled shape** (an outline, not a line) can't be
  read as a line; it is left out and reported, as are other shapes qd
  doesn't recognise (free shapes, polygons other than rectangles).
- **A line with several bends** becomes one arrow between its end boxes;
  its route is left to the diagram's own routing.
- **A picture whose file is missing** next to the deck is imported with
  its path, and the report says the file wasn't found.
- **Nothing recognisable** (no boxes): the command fails, explains, and
  changes nothing.
- **A file that isn't SVG, or can't be read**: fails, changes nothing.
- **Drawings exported from Keynote, Illustrator, Inkscape or draw.io**
  (shapes flattened to outlines, text split into pieces) are out of scope;
  importing one does what it can and reports the rest.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: `qd dot import FILE --on SLIDE` (or `--after REF`,
  `--before REF`) MUST add one pinned diagram cell made from the drawing
  in `FILE`; `--replace REF` MUST put it in place of that element instead.
- **FR-002**: It MUST be one edit: the version check, a dry run, and one
  undo step, with JSON output.
- **FR-003**: Each box (a rectangle, rounded or not, a circle or an
  ellipse, with its text) MUST
  become one box, pinned at the same place relative to the drawing, with
  the same size, label, fill, outline and text colours, and its link and
  picture when it has them.
- **FR-004**: Each line (straight or bent, with or without arrowheads)
  whose two ends are each near a box MUST become one arrow between those
  boxes, with heads matching the drawing's.
- **FR-005**: Positions MUST be in the diagram's units (inches), scaled so
  the diagram's size on the slide matches the drawing's.
- **FR-006**: An SVG produced by Graphviz MUST keep its box names and
  arrows exactly, and pin each box where it was drawn.
- **FR-007**: Everything left out (stray lines, text outside boxes,
  shapes not recognised, pictures not found) MUST be listed in the output
  with the reason; nothing is dropped silently.
- **FR-008**: The result MUST be an ordinary spec 015 diagram: readable by
  `qd dot show`, editable on the slide, and rendered by plain Quarto (plus
  the pictures extension when it has pictures).
- **FR-009**: A drawing with no boxes, or a file that can't be read as
  SVG, MUST fail with a clear error and change nothing.

### Key Entities

- **Drawing**: the SVG file read: its boxes, lines, text, links and
  pictures, and its own size.
- **Import report**: the boxes and arrows made (with their names), and the
  parts left out with reasons.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Importing a hand-drawn diagram of up to 20 boxes takes one
  command and under 5 seconds.
- **SC-002**: Imported boxes, rendered, sit within 5 slide pixels of
  where they were in the drawing (scaled to the same size).
- **SC-003**: Every box with text and every line between two boxes in a
  hand-written drawing is imported; nothing is dropped without being
  reported.
- **SC-004**: A Graphviz-drawn SVG imports with 100% of its box names and
  arrows kept.
- **SC-005**: The diagram the person rebuilt by hand in spec 015 could be
  produced by the importer from its original SVG with only colour or
  routing touch-ups left to do.

## Assumptions

- **Scale**: the diagram is drawn at 72 units to the inch, as Graphviz
  does, so a drawing 1440 units wide becomes a diagram 20 inches wide;
  the cell's figure width and height are set so it shows at the drawing's
  size on the slide.
- **Names** (the person's choice): boxes of a hand-written drawing are
  named from their labels (lowercase, letters and digits), made unique
  where two match.
- **Bends** (the person's choice): a bent line becomes one arrow between its
  boxes, routed by the diagram (`splines=ortho`); the drawn path isn't kept.
- **Looks**: colours are written on each box, as spec 015 does; the
  diagram uses `layout=neato` and `splines=ortho` like qd's other
  diagrams.
- **Arrow ends**: an end counts as near a box when it is within a short
  distance of its edge (a few pixels, scaled with the drawing), or when
  the line points at the box and meets it if extended a little; otherwise
  it is reported.
- **Out of scope**: SVGs exported from slide or drawing apps (flattened
  outlines, split text), Mermaid, and an editor action to import.
