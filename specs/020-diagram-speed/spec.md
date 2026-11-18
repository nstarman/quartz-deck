# Feature Specification: Diagrams that keep up with the hand

**Feature Branch**: `020-diagram-speed`

**Created**: 2026-11-17

**Status**: Draft

**Input**: User description: "Plan performance speedups of Graphviz {dot} interactivity. It's very slow to drag things around."

## Context

Spec 015 made `{dot}` diagrams editable on the slide. Dragging a box feels
slow. Measured on a made-up two-slide deck with a seven-box diagram
(2026-11-17), one drop of a box costs:

| Step | Time |
|---|---|
| Writing the edit to the file | 0.01 s |
| Waiting before the preview renders | 0.2 s |
| Quarto rendering the whole deck again | 2.8 s (5 s when run alone, against 2.9 s for the same deck with no diagram) |
| Reloading the preview | 0.06 s |
| Graphviz laying out the diagram in the browser | 2–6 ms for 20 boxes and 25 arrows |

So the time is almost all Quarto's render of the whole deck, which grows
with the deck. A talk with many slides and figures takes much longer.
Until it finishes, the dropped box sits where it was dropped with straight
stand-in lines for its arrows; then the whole preview reloads. The editor
already carries the same Graphviz engine that Quarto uses, and it lays the
diagram out in milliseconds.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — A drop shows its result at once (Priority: P1)

The presenter drags a box and lets go. The box stays there and its arrows
are routed to it straight away, as they will be in the rendered deck. They
can drag the next box immediately, as many times as they like, without
waiting for anything.

**Why this priority**: it is the slowness the person feels.

**Independent Test**: in a deck whose full render takes several seconds,
drag three boxes one after another as fast as possible. After each
release, the arrows are routed within a frame or two, and each next drag
starts at once. When the background render ends, nothing on the slide
moves or flashes.

**Acceptance Scenarios**:

1. **Given** a box is dropped, **When** the edit is saved, **Then** the
   diagram is drawn as Graphviz lays it out with the box there, arrows
   routed, within 100 ms of the release.
2. **Given** a render is still running, **When** the presenter drags
   another box, **Then** the drag works as usual and its drop also shows
   at once.
3. **Given** the background render finishes, **When** it replaces what was
   shown, **Then** the diagram looks the same, the selection and the
   shown slide stay, and nothing flashes.
4. **Given** the other diagram edits (connect two boxes, add, rename,
   restyle, delete, flip an arrow, nudge with arrow keys), **When** each is
   made, **Then** it shows at once in the same way.
5. **Given** a box with a picture (spec 015's pictures in boxes), **When**
   it is moved, **Then** its picture moves with it in the instant drawing.

---

### User Story 2 — Arrows follow the box while dragging (Priority: P2)

While the presenter drags a box, its arrows are routed live to where it
is, as Graphviz will route them, rather than shown as straight stand-in
lines.

**Why this priority**: it makes the drag itself feel direct. It already
exists as a setting ("Route arrows while dragging", spec 015 US4) that is
off by default; the measured layout time makes it cheap enough to be on.

**Independent Test**: drag a box with two arrows across a 20-box diagram.
The arrows are routed live and the drag keeps the screen's frame rate.

**Acceptance Scenarios**:

1. **Given** the default settings, **When** a box is dragged, **Then** its
   arrows are routed live.
2. **Given** a diagram large enough that a layout takes longer than a
   frame, **When** a box is dragged, **Then** the box still follows the
   pointer at full rate, and the arrows catch up as layouts finish.

---

### Edge Cases

- **The instant drawing and Quarto disagree** (a Graphviz feature the
  browser can't match, a Quarto option): the render's result wins when it
  arrives, and the difference is reported in the browser console for
  debugging, not to the person.
- **The engine can't be loaded** (an old Quarto without it): edits work as
  today, waiting for the render.
- **An edit from outside** (the CLI, an agent, another editor) to a
  diagram: shown when the render finishes, as today.
- **A render error**: shown as today; the instant drawing stays.
- **Undo and redo** of a diagram edit show at once like the edit itself.
- **Other kinds of edit** (text, slides, boxes): unchanged by this
  feature.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: After any diagram edit made in the editor, the editor MUST
  show the edited diagram as Graphviz lays it out, without waiting for
  Quarto's render.
- **FR-002**: The editor MUST NOT block the next edit while a render runs.
- **FR-003**: When the render finishes, the preview MUST take it without
  moving or flashing the diagram and without losing the selection or the
  shown slide.
- **FR-004**: The instant drawing MUST match the rendered diagram: same
  boxes, positions, routes, colours, labels, links and pictures.
- **FR-005**: Arrows MUST be routed live while dragging by default; the
  existing setting can turn it off.
- **FR-006**: The saved file and the rendered deck MUST be exactly what
  they are today: this changes only what the editor shows in between.

### Key Entities

- **Instant drawing**: the diagram as the editor's own Graphviz lays it
  out from the saved cell, shown until the render arrives.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: From releasing a dragged box to its routed arrows showing:
  under 100 ms on a diagram of up to 20 boxes, whatever the deck's size.
- **SC-002**: Ten boxes can be dragged one after another with no wait
  between drags.
- **SC-003**: When the render arrives, the diagram moves by no more than 1
  slide pixel.
- **SC-004**: While dragging with live routing, the box follows the
  pointer at the screen's frame rate on a 20-box diagram.

## Assumptions

- **Same engine**: the editor uses the Graphviz build that ships with the
  person's Quarto (spec 015 research R6), so the instant drawing and the
  render agree.
- **Pictures**: the editor draws pictures into the instant drawing the way
  the `dot-image` filter does, since both are qd's own.
- **Not in scope**: making Quarto's own render of the deck faster
  (rendering only the changed slide, keeping Quarto running between
  renders). That would speed up every kind of edit, and it is a separate
  piece of work.
