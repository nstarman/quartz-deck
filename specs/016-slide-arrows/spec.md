# Feature Specification: Arrows on the slide

**Feature Branch**: `016-slide-arrows`

**Created**: 2026-11-17

**Status**: Draft

**Input**: User description: "Arrows on the slide (spec 016; the roadmap's \"quarto-arrows in the editor\"): draw and edit arrows on a slide the way you would in Keynote, built on Emil Hvitfeldt's quarto-arrows Quarto extension (MIT, `{{< arrow from=\"x,y\" to=\"x,y\" ... >}}`, filter `arrows`), so the deck stays plain Quarto plus that extension, and every edit is a `qd arrow …` command. Arrows are free; their ends snap to elements with the existing guides. (P1) draw and move; (P2) curve and style; (P2) labels; (P3) appears on click. CLI: `qd arrow add|move|set|rm|show`."

## Context

Talks point at things: an arrow from a label to a peak in a figure, from
one box to another, to the part of an equation that matters. Today the
presenter draws those arrows in another program and pastes in an image, or
writes SVG by hand, and nothing about them can be changed in qd's editor.

Emil Hvitfeldt's quarto-arrows extension (MIT) draws an arrow from a
shortcode: `{{< arrow from="x,y" to="x,y" >}}`, with options for a curve,
colour, width, dashes, heads, a label, and a draw-in animation on click.
With `position="absolute"`, its coordinates are the slide's own canvas
pixels: a spike on a 1600×900 deck drew `from="0,0" to="1600,900"` exactly
corner to corner. This feature makes those arrows editable on the slide,
and keeps every change a small edit to the shortcode, so the deck stays
plain Quarto plus that one extension.

Decisions made with the person: build on quarto-arrows rather than a filter
of qd's own; arrows are **free**, placed at coordinates, so moving an
element later doesn't move an arrow, but while drawing or dragging, an
arrow's ends snap to elements and to the slide with the existing guides.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Draw an arrow and move it (Priority: P1)

The presenter wants an arrow from a caption to a point in a figure. They
choose Insert ▸ Arrow and drag on the slide from where it should start to
where it should point. An arrow appears, drawn the way the rendered deck
will draw it. As they drag, the ends catch on the edges and centres of the
elements on the slide and on the slide's own edges and centre, with a
guide line, as boxes do; holding ⌘ places an end freely. Later they click
the arrow to select it, drag either end to a new place, drag the middle to
move the whole arrow, nudge it with the arrow keys, or delete it. Each
change is one step of undo.

**Why this priority**: drawing and placing arrows is the feature; the rest
refines it.

**Independent Test**: on a deck with one slide, Insert ▸ Arrow and a drag
add one arrow line to the slide's source, with the dragged start and end
in slide pixels; the rendered slide shows the arrow there; dragging an end
changes only that end; `qd undo` takes each change back.

**Acceptance Scenarios**:

1. **Given** a slide, **When** the presenter chooses Insert ▸ Arrow and
   drags from one point to another, **Then** one arrow is added to the
   slide's source at those points, and the deck's folder gains the arrows
   extension the first time.
2. **Given** an arrow being drawn or dragged near an element's edge or
   centre, or the slide's, **When** it comes within a few screen pixels,
   **Then** that end snaps there and a guide line shows; with ⌘ (Ctrl)
   held it doesn't.
3. **Given** a selected arrow, **When** the presenter drags its start or
   end handle, **Then** only that end changes; **When** they drag the
   arrow's line, **Then** both ends move together.
4. **Given** a selected arrow, **When** the presenter presses an arrow
   key, **Then** it moves 1 px (10 px with ⇧); **When** they press Delete,
   **Then** it's removed.
5. **Given** a drag in progress, **When** Escape is pressed, **Then**
   nothing changes.
6. **Given** the same change made with a command (`qd arrow add`,
   `qd arrow move`, `qd arrow rm`), **When** it runs, **Then** the result is
   identical to the gesture.

---

### User Story 2 — Curve and style (Priority: P2)

The presenter wants the arrow to curve around a figure, in the section's
colour, dashed, with a different head. A selected arrow shows a handle in
its middle; dragging it bends the arrow. The Properties panel sets its
colour, line width, dashes, the head's style, and whether it has a head at
the start, the end, or both.

**Why this priority**: straight black arrows cover many slides, but a
talk's arrows usually need its colours and some curve.

**Independent Test**: bend an arrow by its middle handle and set colour,
width, dash and head in Properties; each change edits only that option in
the arrow's line, and the render matches.

**Acceptance Scenarios**:

1. **Given** a selected arrow, **When** the presenter drags its middle
   handle sideways, **Then** the arrow curves through the handle and its
   line records the curve; dragging it back onto the straight line
   removes the curve.
2. **Given** a selected arrow, **When** colour, width, dash, head style or
   heads are set in Properties, **Then** only those options change in its
   line, and options left at their defaults aren't written.

---

### User Story 3 — A label on the arrow (Priority: P2)

The presenter wants the arrow to say what it points at. Double-clicking an
arrow opens a small sheet for its label; the label is drawn along the
arrow, at its start, middle or end.

**Why this priority**: an arrow with a word next to it is one of the
commonest slide annotations; a separate text box would drift away from it.

**Independent Test**: double-click an arrow, type a label, confirm; the
arrow's line gains the label and the render shows it; clearing the label
removes the option.

**Acceptance Scenarios**:

1. **Given** an arrow, **When** the presenter double-clicks it and enters
   a label, **Then** the label is added to its line and drawn along it.
2. **Given** a label, **When** its position (start, middle, end) is chosen
   in Properties, **Then** only that option changes.

---

### User Story 4 — Appears on click (Priority: P3)

The presenter wants the arrow to appear when they click, drawn in from its
start. The element bar's "Appears on click", the same toggle other
elements use, turns this on for an arrow.

**Why this priority**: a nicety for pacing a talk; arrows work without it.

**Independent Test**: turn on "Appears on click" for an arrow; its line
gains the draw-in option; presenting, the arrow is hidden until the click
and then draws in.

**Acceptance Scenarios**:

1. **Given** a selected arrow, **When** "Appears on click" is turned on,
   **Then** the arrow appears on the next click when presenting, drawn in
   from its start; turning it off removes the option.

---

### Edge Cases

- **Arrows written by hand.** An arrow line written with quarto-arrows'
  syntax and `position="absolute"` is read and editable like one qd made,
  and every option qd doesn't edit is kept as it was. Waypoints and
  control points are kept and drawn; dragging an end moves only that end.
- **Arrows in the flow.** An arrow without `position="absolute"` sits in
  the text's flow, so its coordinates aren't slide pixels. It's shown, and
  stays editable as text, but isn't moved on the slide.
- **Arrows qd can't read** (a malformed shortcode, an option written in a
  way qd doesn't parse safely) stay editable as text, with the reason.
- **A slide of another size.** Coordinates follow the deck's own canvas
  size (for example 1050×700 or 1600×900), so arrows land in the same
  place in the editor and in the render.
- **An arrow off the slide.** Ends may be dragged past the slide's edge, as
  in a slide app; the arrow is clipped by the slide when rendered.
- **Moving an element** an arrow points at doesn't move the arrow (arrows
  are free).
- **Overlap.** An arrow drawn over an element is selected by clicking its
  line, a hit area a few pixels wide; a click elsewhere on the element
  selects the element.
- **The extension is missing** (deleted from the deck's folder): the
  arrow renders as Quarto's warning for an unknown shortcode; adding or
  editing an arrow in qd installs it again.
- **An old installed copy** of the extension is replaced by qd's pinned
  version, and a copy the person edited is left alone with a warning, as
  for qd's other extensions.
- **Offline the first time.** With no cached copy and no network, adding
  the first arrow fails with a clear error and changes nothing; arrows
  already in a deck keep rendering from the deck's own copy.

## Requirements *(mandatory)*

### Functional Requirements

**Arrows in the deck**

- **FR-001**: An arrow MUST be one line in the slide's source holding one
  quarto-arrows shortcode with `position="absolute"`, so the deck renders
  with plain Quarto plus the arrows extension.
- **FR-002**: Coordinates MUST be in the slide's canvas pixels, the same
  in the editor and in the rendered deck.
- **FR-003**: The first arrow added to a deck MUST install the arrows
  extension into the deck's folder. (It provides a shortcode, so the deck
  needs no `filters:` entry; research R2.) Undo removes the arrow; the
  extension's files may stay, unused.
- **FR-004**: The extension MUST be quarto-arrows' tagged release v0.3.0,
  downloaded from its own release once, checked against a pinned SHA-256
  and kept in qd's cache; qd MUST NOT ship the extension's code itself.
  It is installed the way qd installs its other extensions (by version; a
  copy the person edited is kept, with a warning).
- **FR-004a**: Without a cached copy and without network (or with
  downloads turned off), adding the first arrow MUST fail with a clear
  error saying what to do, and change nothing.

**Commands (CLI first)**

- **FR-005**: `qd arrow add SLIDE --from X,Y --to X,Y [options]` MUST add an
  arrow and return its ref; options cover curve, colour, width, dash, head
  style, heads at start and end, label and its position, and appears on
  click.
- **FR-006**: `qd arrow move REF` MUST move the start (`--from X,Y`), the
  end (`--to X,Y`) or both (`--by DX,DY`).
- **FR-007**: `qd arrow set REF KEY=VALUE…` MUST set or (with `KEY=`)
  remove options, changing nothing else in the line.
- **FR-008**: `qd arrow rm REF` MUST remove an arrow; `qd arrow show REF`
  MUST report its start, end, every option, and whether it can be edited
  (with the reason when it can't).
- **FR-009**: Every command MUST be a normal qd edit: JSON output, the
  version check (`--expect`), a dry run, and one undo step; and be listed
  in `qd schema` and the skill.
- **FR-010**: An edit MUST change only the arrow's own line, and within it
  only the options being changed; option order and quoting written by
  hand are kept.

**On the slide**

- **FR-011**: Insert ▸ Arrow MUST let the presenter draw an arrow by
  dragging on the slide.
- **FR-012**: Clicking an arrow MUST select it, showing handles at its
  start, end and middle; dragging a start or end handle moves that end,
  dragging the line moves the whole arrow, and dragging the middle handle
  bends it.
- **FR-013**: While drawing or dragging, an end MUST snap to the edges and
  centres of the slide's elements and of the slide within a few screen
  pixels, with a guide line, under the existing "Snap to guides"
  setting; ⌘/Ctrl MUST place it freely.
- **FR-014**: The arrow keys MUST nudge a selected arrow (1 px, ⇧ 10 px);
  Delete MUST remove it; Escape mid-drag MUST cancel.
- **FR-015**: The Properties panel MUST show and set an arrow's colour,
  width, dash, head style, heads at start and end, and label position.
- **FR-016**: Double-clicking an arrow MUST open its label for editing.
- **FR-017**: The element bar's "Appears on click" MUST turn an arrow's
  draw-in on and off.
- **FR-018**: While an end or the whole arrow is dragged, the editor MUST
  show the arrow at its new place; the saved line MUST be the same as the
  equivalent command's.
- **FR-019**: Every gesture MUST also be reachable from a menu or the
  keyboard, and the handles MUST have accessible names.

**Arrows qd didn't write**

- **FR-020**: An arrow written by hand with quarto-arrows' syntax and
  `position="absolute"` MUST be editable, keeping every option qd doesn't
  change, waypoints and control points included.
- **FR-021**: An arrow without `position="absolute"`, or one qd can't read
  safely, MUST be shown but edited only as text, with the reason from
  `qd arrow show`.

### Key Entities

- **Arrow**: one shortcode line on a slide. Its start and end, in slide
  pixels; optional curve or control points or waypoints; style (colour,
  width, dash, head style, heads); label and label position; appears on
  click. Free: it doesn't refer to any element.
- **Arrows extension**: quarto-arrows v0.3.0, downloaded once into qd's
  cache and installed in the deck's folder.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A presenter can add an arrow from one element to another in
  under 10 seconds, without typing.
- **SC-002**: An arrow's ends land in the rendered deck within 1 slide
  pixel of where they were dropped in the editor.
- **SC-003**: Every edit changes only the arrow's own line; a no-op edit
  leaves the deck byte-for-byte unchanged.
- **SC-004**: A deck with arrows renders with plain Quarto (with the
  arrows extension) and passes qd's offline check; the one-file export
  shows them.
- **SC-005**: Every editor action on arrows has a `qd arrow` command that
  gives the same file.
- **SC-006**: The arrows on one of the person's real slides can be redrawn
  in the editor and accepted by them as matching.

## Assumptions

- **Where an arrow sits:** each arrow is its own line, by default after the
  slide's content and before its speaker notes, so it is one element of
  the slide like any other.
- **Arrows in the flow** (without `position="absolute"`) are shown but not
  moved on the slide; converting them would change where they appear.
- **Installing:** quarto-arrows has tagged releases (v0.1.0, v0.2.0,
  v0.3.0, the latest, of 2026-10-05). Its README says "MIT" but the
  repository has no licence file, so qd doesn't redistribute it: qd
  downloads the v0.3.0 release the first time a deck gets an arrow, checks
  its SHA-256 and caches it, as it does for MathJax. The version stays
  fixed until qd updates the pin. (The person decided not to ask the
  author for a licence file.)
- **Snapping** uses the guides and the setting from specs 009 and 015;
  there is no grid for arrows, and positions are whole pixels.
- **Labels** use quarto-arrows' own label option, plain text first; its
  math labels render as the extension renders them.
- **Out of scope:** arrows attached to elements (re-routing when an element
  moves), arrows inside diagrams (spec 015 has its own), and drawing other
  shapes.
