# Feature Specification: Diagrams you edit on the slide

**Feature Branch**: `015-graphviz-diagrams`

**Created**: 2026-11-15

**Status**: Draft

**Input**: User description: "Diagrams you edit on the slide (spec 015), following the approved design in design/2026-11-15-graphviz-diagrams-design.md: a Quarto {dot} (Graphviz) cell with layout=neato, splines=ortho and pinned node positions becomes editable on the slide in qd's editor, Keynote-style: drag a box to move it, connect, rename, add and delete boxes and arrows, set style, link, image and size; every edit a `qd dot …` command producing minimal patches; a companion render plugin `dot-image` for images in boxes; an optional live-routing setting."

## Context

A talk's diagram of boxes and arrows (tools and the things they make) is
today a hand-drawn SVG pasted into the slide as raw HTML. Nothing in it can
be moved or changed in qd's editor. Quarto renders Graphviz `{dot}` cells
natively, and Graphviz can hold each box's position (`pos="x,y!"` with
`layout=neato`) while routing the arrows at right angles around them
(`splines=ortho`). This feature makes such a cell editable on the slide,
the way a slide app edits shapes and connectors. Every change is a small
edit to the cell's text, so the deck stays plain Quarto.

The approved design, with what was checked and the approaches considered,
is `design/2026-11-15-graphviz-diagrams-design.md`. Its decisions are
requirements here.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Move a box, and its arrows follow (Priority: P1)

The presenter has a diagram on a slide as a `{dot}` cell. In qd's editor
they click a box and drag it somewhere else. When they let go, the box
stays where they put it (on a quarter-inch grid), and after a moment the
arrows into and out of it re-route at right angles. A diagram that
Graphviz laid out by itself can be made placeable with one action, "Pin
layout", which fixes every box where it is now.

**Why this priority**: placing boxes by hand is the reason to leave the
hand-drawn SVG; without it nothing else matters.

**Independent Test**: on a deck with a pinned three-box diagram, drag one
box in the editor; the cell's text changes only in that box's position,
the rendered slide shows the box at the new place with re-routed arrows,
and `qd undo` puts it back.

**Acceptance Scenarios**:

1. **Given** a pinned diagram, **When** the presenter drags a box and
   drops it, **Then** only that box's position changes in the cell's
   text, snapped to 0.25 in, and the preview shows the box there with
   its arrows re-routed at right angles.
2. **Given** a drag in progress, **When** the presenter presses Escape,
   **Then** nothing changes.
3. **Given** a diagram that isn't pinned, **When** the presenter starts
   dragging a box, **Then** the editor offers "Pin layout"; accepting
   it fixes every box at its current place and makes the diagram
   placeable, with the slide looking the same as before.
4. **Given** the same move done as a command (`qd dot move CELL NODE --at
   X,Y`), **When** it runs, **Then** the result is identical to the drag.
5. **Given** comments and hand formatting in the cell, **When** any box
   is moved, **Then** every other line of the cell is unchanged, byte for
   byte.

---

### User Story 2 — Build and change the diagram on the slide (Priority: P1)

The presenter changes the diagram's structure without opening its source:
they drag from a handle on one box onto another to add an arrow, or onto
empty space to add a new box already connected; double-click a box to
rename it; delete a box (its arrows go with it) or a single arrow; flip
an arrow or turn its head off; and set a box's look, link and size in the
Properties panel. Insert ▸ Diagram starts a new placeable diagram, and
Insert ▸ Box adds a box to the selected one.

**Why this priority**: the person asked for editing "like Keynote",
adding connecting arrows and the rest, on the slide itself.

**Independent Test**: starting from Insert ▸ Diagram on an empty slide,
build a three-box diagram with two arrows, rename one box, restyle
another, delete an arrow, entirely in the editor; the deck renders with
plain Quarto and shows exactly that.

**Acceptance Scenarios**:

1. **Given** two boxes, **When** the presenter drags from one's handle
   onto the other, **Then** an arrow from the first to the second is
   added, and appears after the re-render.
2. **Given** a drag from a handle, **When** it ends on empty space,
   **Then** a new box is added there, connected from the first.
3. **Given** a box, **When** the presenter double-clicks it and types a
   new label, **Then** only its label changes; its arrows stay attached.
4. **Given** a box with arrows, **When** the presenter deletes it,
   **Then** the box and every arrow touching it are removed.
5. **Given** an arrow, **When** the presenter selects it, **Then** they
   can delete it, reverse it, or switch its arrowhead off.
6. **Given** a box, **When** the presenter picks a style in the
   Properties panel, **Then** the box takes that look (from the looks
   already in the diagram, or a starter "tool" or "artifact" look), and
   the look is written on the box itself so it renders the same
   everywhere.
7. **Given** a box, **When** the presenter sets its link or size in the
   Properties panel, **Then** the rendered box links there / has that
   size.
8. **Given** an empty slide, **When** the presenter chooses Insert ▸
   Diagram, **Then** a placeable diagram with two example boxes in the
   two starter styles is added.
9. **Given** each of these edits, **When** it is done as a `qd dot …`
   command instead, **Then** the result is identical; every edit can be
   undone with `qd undo`.

---

### User Story 3 — Pictures in boxes (Priority: P2)

Some boxes carry a small picture, such as the author's avatar on the
tools they made. The presenter sets a box's image (Graphviz's own
`image="path"`), and the rendered deck shows the picture inside the box
at its left, without covering the label, even though Quarto's own
Graphviz can't draw images.

**Why this priority**: it keeps the current diagram's look; the diagram
works without it.

**Independent Test**: a diagram with one box whose image is set renders
with plain Quarto plus the companion plugin; the picture is inside that
box, at its left, not over the label, and the deck still works offline
and as a one-file export.

**Acceptance Scenarios**:

1. **Given** a box with `image=` set and the companion plugin in the
   deck, **When** the deck renders, **Then** the picture appears inside
   that box at its left, and the label text doesn't overlap it.
2. **Given** that deck, **When** it is exported to one offline file,
   **Then** the picture is embedded and shows offline.
3. **Given** the deck's own stylesheet, **When** it styles the picture
   (for example as a circle), **Then** the picture takes that style.
4. **Given** a deck where a box's image file doesn't exist, **When** it
   renders, **Then** the box renders without a picture and the render
   warns naming the file.

---

### User Story 4 — Arrows that re-route while dragging (Priority: P3)

A presenter who wants instant feedback turns on a setting. From then on,
while they drag a box, its arrows re-route at right angles live, instead
of showing straight lines until the drop.

**Why this priority**: a comfort; the default re-route on drop is enough
to edit diagrams.

**Independent Test**: with the setting on, dragging a box shows routed
arrows that follow it before the drop; the saved result is the same as
with the setting off.

**Acceptance Scenarios**:

1. **Given** the setting is off (the default), **When** the presenter
   drags a box, **Then** its arrows show as straight lines until the
   drop.
2. **Given** the presenter turns the setting on, **When** it is turned
   on the first time, **Then** the routing engine is taken from their
   Quarto install, or fetched once, and from then on works offline.
3. **Given** the setting is on, **When** the presenter drags a box,
   **Then** its arrows re-route while it moves.
4. **Given** the setting is on, **When** the drop happens, **Then** the
   saved text and the rendered slide are the same as with it off.
5. **Given** the routing engine is missing or fails, **When** the
   presenter drags, **Then** the editor falls back to straight lines
   without an error dialog.

---

### Edge Cases

- A cell with DOT the editor can't read safely: it stays editable as text
  only, and selecting it says why.
- Subgraphs and clusters: shown and kept; their contents are editable as
  text only.
- Several arrows between the same two boxes: each can be selected and
  addressed on its own.
- A chain `a -> b -> c` written on one line: deleting one arrow of it
  keeps the other.
- Undirected diagrams (`graph`, `--`): connect adds `--`; flip does
  nothing.
- A new box's name never clashes with an existing one.
- Two boxes dropped on the same grid point: allowed; Graphviz may nudge
  arrows.
- The cell changed in the source pane or by another editor while
  dragging: the drop is refused as a conflict, not applied over it.
- A box label written as an HTML-style label: rename edits its text, and
  if it can't do so safely it says so.
- Graphviz can't route an arrow at right angles (overlapping boxes): the
  render shows what Graphviz produces; nothing is lost.

## Requirements *(mandatory)*

### Functional Requirements

**Reading and editing the cell**

- **FR-001**: qd MUST read a `{dot}` cell's statements (graph settings,
  node and edge defaults, nodes, edges including chains, attributes,
  comments, subgraphs) with each part's place in the text.
- **FR-002**: Every diagram edit MUST change only the statement it
  touches: an attribute's value in place, an attribute added to the
  statement, a new statement on its own line before the closing brace,
  or a removed statement; all other text stays byte for byte.
- **FR-003**: A cell qd can't read safely MUST be refused for diagram
  editing with a reason and stay editable as text.
- **FR-004**: Every edit MUST be a qd edit: `--dry-run`, `--expect`
  conflict refusal, undo, JSON output (Principle III, V).

**Commands**

- **FR-005**: `qd dot show CELL` MUST list the diagram's boxes (name,
  label, look, link, image, position, size), arrows, graph settings, and
  whether it is editable (and why not).
- **FR-006**: `qd dot move CELL NODE --at X,Y` MUST set the box's pinned
  position in inches, snapped to 0.25 in unless `--no-snap`.
- **FR-007**: `qd dot add`, `connect`, `rename`, `set`, `rm`, `flip`
  MUST add a box (optionally placed, styled and connected), add an arrow,
  change a box's label, set attributes (including copying another box's
  look), remove a box with its arrows or a single arrow, and reverse an
  arrow.
- **FR-008**: `qd dot pin CELL` MUST set `layout=neato`,
  `splines=ortho` and every box's current position, using positions the
  editor passes, else the last preview render's, else a Graphviz on
  PATH, else fail with a hint.
- **FR-009**: `qd dot new --after REF` MUST insert a starter diagram: a
  placeable layout, defaults, and two example boxes in the two starter
  looks.
- **FR-010**: New boxes MUST get names unused in the cell; labels are
  separate from names.

**The editor**

- **FR-011**: In the preview, the boxes and arrows of an editable diagram
  MUST be selectable; a selected box shows connect handles and a resize
  corner.
- **FR-012**: Dragging a box MUST move it with provisional straight
  arrows, save its position on drop, and re-render; Escape cancels.
- **FR-013**: Dragging from a handle MUST connect to the box it's dropped
  on, or add a connected box where it's dropped on empty space.
- **FR-014**: Double-clicking a box MUST edit its label in place.
- **FR-015**: Delete MUST remove the selected box (with its arrows) or
  arrow; the Properties panel MUST offer a box's look, link, image and
  size, an arrow's direction and arrowhead, and "Pin layout" for an
  unpinned diagram.
- **FR-016**: Insert ▸ Diagram and Insert ▸ Box MUST exist, listed with
  the editor's other actions.
- **FR-017**: A box's look MUST be a set of Graphviz attributes written on
  the box (shape, fill, outline, font colour, font, line width); the
  choices are the looks already in the diagram plus two starters.
- **FR-018**: Connect handles MUST only start a drag; which side an arrow
  meets a box is Graphviz's choice. This version writes no arrow labels.

**Pictures in boxes**

- **FR-019**: A companion render plugin MUST make a box's `image="path"`
  show inside the box at its left in the rendered deck, leaving room so
  the label isn't covered, and give the picture a class the deck's CSS
  can style.
- **FR-020**: The plugin MUST be carried by qd and installed into a deck
  (and named in its filters) when a diagram uses `image=`, as cite-links
  is; a deck with it renders with plain Quarto.
- **FR-021**: A missing image file MUST render the box without a picture
  and warn naming the file.

**Live routing**

- **FR-022**: A setting `diagrams.live-layout` (off by default) MUST,
  when on, re-route a dragged box's arrows while it moves.
- **FR-023**: The routing engine MUST come from the person's own Quarto
  install when it can be loaded, else be fetched once when the setting is
  turned on and pinned by checksum in qd's cache. It is never fetched by
  a rendered deck. Its licence (EPL-1.0) is listed and allowed for this
  engine.
- **FR-024**: Live routing MUST NOT change what's saved; if the engine is
  missing or fails, dragging falls back to straight lines quietly.

### Key Entities

- **Diagram**: a `{dot}` cell on a slide; placeable when it uses
  `layout=neato`, `splines=ortho` and pinned positions.
- **Box**: a node: name (stable), label, look (attributes), link, image,
  position (inches), size.
- **Arrow**: an edge between two boxes; direction, arrowhead; addressed
  by its ends and, for repeats, an index.
- **Look**: a named-by-example set of Graphviz attributes applied to
  boxes.
- **Companion plugin**: the render extension that draws boxes' images.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A three-box, two-arrow diagram can be built from an empty
  slide entirely in the editor, with no text editing, in under two
  minutes.
- **SC-002**: Every editor action in this feature can also be done by a
  command, with identical results, in 100% of the tests.
- **SC-003**: In every edit test, the cell's text outside the touched
  statement is unchanged byte for byte, and a no-op edit leaves a corpus
  of real DOT files identical.
- **SC-004**: After a drop, the re-routed arrows are on the slide within
  3 s on a typical laptop (the preview's usual re-render time).
- **SC-005**: Every diagram made in the editor renders with plain Quarto
  (plus the companion plugin when it has images) and passes the offline
  check.
- **SC-006**: The talk's existing pipeline diagram can be rebuilt as a
  placeable diagram that the person accepts as matching its look,
  avatars included.

## Assumptions

- Quarto (with its own Graphviz) is installed where decks render, as
  today; the measured behaviour is Quarto 1.11.9's.
- Graphviz's right-angle routing chooses arrow sides well enough for the
  proving diagram; if not, fixed sides become a later change.
- Mermaid editing and quarto-arrows in the editor are separate, later
  features.
- Free-form shapes and drawing are out of scope; boxes are Graphviz node
  shapes.
- Rebuilding the talk's existing diagram is done with the person after
  the feature lands, as the proving case; it isn't converted
  automatically.
- The order of work is six PRs: the DOT reader with `show`; the edit
  commands; the editor's select, drag and Properties; the editor's
  connect, add, rename, delete and Insert menus; the companion plugin;
  the live routing setting.
