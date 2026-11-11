# Feature Specification: Layouts and placement — start from a layout, place anything anywhere

**Feature Branch**: `009-layouts`

**Created**: 2026-11-11

**Status**: Draft

**Input**: User description: "Layouts and placement (roadmap 009): work like a slide app — start a slide from a layout, then place anything anywhere on it, while the .qmd stays plain Quarto. (P1) Layouts for a new slide: Insert ▸ Slide and the slide list's + offer a gallery with a small preview of each (title only, title + content, two columns, three columns, image + text left/right, full-bleed image, section header, big number/quote, title + notes, blank); a layout is plain Quarto markdown (heading, :::: {.columns} with ::: {.column}s, .absolute boxes) with placeholder elements that are ordinary, editable elements; nothing about the layout is stored separately. CLI first: qd layouts lists built-in and saved layouts with their placeholders; qd add --layout NAME [--after SLIDE] [--title T] adds a slide and returns the placeholder refs for agents to fill with qd edit. (P1) Free placement: take any element out of the flow by ⌥-drag or "Place freely" (menu, right-click, qd block place REF --at X,Y [--size W,H]) — it is wrapped in Quarto's ::: {.absolute top= left= width=} div (same syntax as today's boxes) with its own bytes unchanged; move and resize it like a box (drag, handles, arrow keys to nudge, ⇧ for bigger steps, qd box move/resize); "Put back in the flow" (qd block unplace REF) removes the wrapper. (P2) The person's own layouts: "Save slide as layout…" keeps a slide's structure with placeholders for reuse. (P2) Alignment guides to the slide's edges, centre and other elements while dragging, snapping that can be turned off (a setting). (P3) Bring to front / Send to back for overlapping boxes, reordering the source. Every action stays in the 007 action list with its key; every edit a qd command, one undo step."

## Overview

Slide apps start a new slide from a layout (title and content, two columns,
a picture beside text) and then let anything on it be moved anywhere. The
quartz-deck editor today adds an untitled slide and keeps every element in
the slide's flow, except text and image boxes (001), which can already be
dragged and resized. This feature adds both halves, with one rule throughout:
the deck stays plain Quarto. A layout is only the markdown it writes; a
placed element is only the element, wrapped in the same positioned block that
boxes already use. Nothing about layouts or placement is kept anywhere but in
the deck, so `quarto render` without qd shows the same slides.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — A new slide from a layout (Priority: P1)

The person adds a slide and picks how it is laid out: a small gallery shows
each layout as a picture of the slide it makes. The new slide appears with
placeholders ("Click to add title", "Click to add text", "Drop an image")
where the content goes. They click a placeholder and type, or drop an image
on its image placeholder. An agent does the same from the command line: it
lists the layouts, adds a slide from one, and gets back the placeholders it
can fill.

**Why this priority**: the first step of every slide; today each starts
empty and its structure (columns, a picture beside text) has to be written
by hand in the source.

**Independent Test**: from Insert ▸ Slide and from the list's +, add a "two
columns" slide; it shows a title and two text placeholders side by side;
type into each; the file holds a heading and two columns with that text and
nothing else. `qd layouts` lists the same layouts; `qd add --layout
two-columns` returns the refs of its placeholders.

**Acceptance Scenarios**:

1. **Given** a deck, **When** the person chooses Insert ▸ Slide (or the
   list's +, or ⇧⌘N), **Then** a gallery of layouts opens beside it with a
   preview of each, the last one used preselected, and Enter or a click adds
   the slide where the + was.
2. **Given** the gallery, **When** the person picks "Image + text", **Then**
   the new slide shows a title placeholder, an image placeholder and a text
   placeholder in the layout's places, on the canvas and in its thumbnail.
3. **Given** a new slide from a layout, **When** the person clicks a text
   placeholder and types, **Then** the placeholder's words are replaced by
   theirs, and the file holds only what they typed in that place.
4. **Given** an image placeholder, **When** the person drops an image file
   on it or picks one from the image sheet (008), **Then** the image takes
   the placeholder's place and size.
5. **Given** a slide from a layout whose placeholders are still unfilled,
   **When** the deck is rendered or presented, **Then** no placeholder text
   appears in the output.
6. **Given** the command line, **When** an agent runs `qd layouts`, **Then**
   it lists each built-in and saved layout with its name, a one-line
   description and its placeholders (kind and purpose).
7. **Given** the command line, **When** an agent runs `qd add --layout
   two-columns --after intro --title "Before and after"`, **Then** one slide
   is added after "intro" with that title, and the result names each
   placeholder's element ref, so the agent can fill them with `qd edit`.
8. **Given** a slide added from a layout, **When** the person presses ⌘Z
   once, **Then** the slide is gone.

---

### User Story 2 — Place any element anywhere (Priority: P1)

The person drags a paragraph, list, image, equation or chart out of the
slide's flow and drops it where they want it on the slide. From then on it
moves and resizes like a text box. When they change their mind, "Put back in
the flow" returns it to where it sits in the source.

**Why this priority**: the other half of working like a slide app; today
only boxes made as boxes can be placed.

**Independent Test**: on a slide with a list, ⌥-drag the list to the lower
right; it stays where it was dropped on the canvas, in the presentation and
in a plain `quarto render`; drag its corner handle to resize it; arrow keys
nudge it; "Put back in the flow" returns the file byte for byte to how it
was before the first drag.

**Acceptance Scenarios**:

1. **Given** an element in the flow, **When** the person ⌥-drags it,
   **Then** it follows the pointer and stays where it is dropped, the rest
   of the slide closes up behind it, and a plain drag still reorders as
   before (003).
2. **Given** a selected element, **When** the person chooses "Place
   freely" (Arrange menu, right-click, or `qd block place REF --at X,Y
   [--size W,H]`), **Then** it is placed where it currently shows, at its
   current size.
3. **Given** a placed element, **When** the person looks at the file,
   **Then** the element's own text is unchanged, inside the same kind of
   positioned block a text box uses.
4. **Given** a placed element, **When** the person drags it, drags a
   handle, or presses an arrow key (⇧ for a bigger step), **Then** it moves
   or resizes, and each change is one undo step.
5. **Given** a placed element, **When** the person chooses "Put back in the
   flow" (or `qd block unplace REF`), **Then** the positioned block is
   removed and the element shows in the flow where its source is.
6. **Given** an element that can't be placed safely (see FR-012), **When**
   the person tries, **Then** the action is unavailable and says why
   instead of changing anything.
7. **Given** a placed element that appeared on click (a fragment), **When**
   the slide is presented, **Then** it still appears on click, in place.

---

### User Story 3 — The person's own layouts (Priority: P2)

Having made a slide they like (a picture on the left, a quote and a source
under it), the person saves it as a layout and starts later slides from it,
in this deck and others.

**Why this priority**: the built-in set covers common slides; a person's
talks repeat their own patterns.

**Independent Test**: make a slide with a placed image and two text
elements; "Save slide as layout…" with a name; the gallery and `qd layouts`
list it; a slide added from it has the same structure with placeholders in
place of the text and image.

**Acceptance Scenarios**:

1. **Given** a slide, **When** the person chooses "Save slide as layout…"
   and names it, **Then** the layout keeps the slide's structure (headings,
   columns, placed elements with their positions and sizes) with
   placeholders in place of its text and images.
2. **Given** saved layouts, **When** the gallery opens or `qd layouts`
   runs, **Then** they are listed after the built-in ones, marked as the
   person's own or the deck's.
3. **Given** a saved layout, **When** the person renames or deletes it,
   **Then** the gallery and `qd layouts` show the change; slides already
   made from it are unchanged.
4. **Given** a name already used, **When** the person saves, **Then** they
   are asked whether to replace it, and nothing is overwritten without a
   yes.
5. **Given** "Save slide as layout…", **When** the person saves, **Then**
   they choose between keeping it as their own (every deck) or with this deck
   (shared with whoever works on it).

---

### User Story 4 — Guides while dragging (Priority: P2)

While moving or resizing a placed element, lines show when it lines up with
the slide's edges or centre or with another element's edge or centre, and
the element snaps to them. Snapping can be turned off.

**Why this priority**: lining things up by eye is the slow part of free
placement.

**Independent Test**: drag a placed element near the slide's horizontal
centre; a centre line shows and the element's centre lands exactly on it;
with snapping off in Settings, no line and no snap.

**Acceptance Scenarios**:

1. **Given** a placed element being dragged, **When** its edge or centre
   comes within a few points of the slide's edge or centre or another
   element's edge or centre, **Then** a guide line shows and the element
   snaps to it.
2. **Given** a guide, **When** the person keeps dragging past it, **Then**
   the snap lets go.
3. **Given** Settings ▸ "Snap to guides" off (or a modifier key held while
   dragging), **When** the person drags, **Then** no guides show and the
   element follows the pointer exactly.

---

### User Story 5 — Front and back (Priority: P3)

When placed elements overlap, the person brings one to the front or sends
it to the back.

**Why this priority**: overlaps are rare on talk slides but must be fixable.

**Independent Test**: two overlapping placed elements; "Bring to front" on
the one behind puts it on top on the canvas and in the presentation.

**Acceptance Scenarios**:

1. **Given** overlapping placed elements, **When** the person chooses Bring
   to front, Send to back, Bring forward or Send backward, **Then** the
   stacking changes on the canvas and in the output, as one undo step.
2. **Given** a stacking change, **When** the person looks at the file,
   **Then** the order of the placed elements in the source has changed and
   nothing else has.

---

### Edge Cases

- A layout added to a deck whose slide size differs from the usual 1050×700:
  positions and sizes are scaled to the deck's slide size.
- The deck's slide size changes after elements were placed: they keep their
  numbers (as Quarto does), and may sit off the slide; the editor marks any
  element that is partly off the slide.
- A placeholder is deleted rather than filled: it simply goes; the slide
  stays valid.
- An element is dragged partly off the slide: it may sit there (decks use
  this deliberately), and the editor marks it.
- An element already inside a container (a column, a callout, an incremental
  list item): placing it is unavailable (see FR-012).
- A placed element that is a code cell: the cell (input and output) moves
  together; its output alone can't be placed.
- An element is placed and then edited by an agent through `qd edit`: its
  position is kept.
- Undo after "Place freely" then a move: two steps, each undone in order.
- VS Code's panel: placing, moving and unplacing land in the buffer as one
  edit each (006).
- A slide made from a layout is presented while still having unfilled image
  placeholders: nothing shows in their place.

## Requirements *(mandatory)*

### Functional Requirements

**Layouts**

- **FR-001**: Adding a slide (Insert ▸ Slide, the list's +, ⇧⌘N) MUST offer
  a gallery of layouts with a preview of each; Enter or a click adds the
  slide, Esc adds nothing.
- **FR-002**: The built-in layouts MUST be: title only, title + content, two
  columns, three columns, image + text (image left), text + image (image
  right), full-bleed image, section header, big number, quote, title +
  notes, and blank.
- **FR-003**: A slide added from a layout MUST be ordinary Quarto markdown
  for that structure (a heading, columns, positioned blocks), so it renders
  the same with Quarto alone; nothing records which layout it came from.
- **FR-004**: A layout's placeholders MUST be ordinary elements that the
  person edits, replaces or deletes like any other; placeholder words MUST
  NOT appear when the deck is rendered or presented.
- **FR-005**: `qd layouts` MUST list the built-in and saved layouts with
  name, description and placeholders; `qd add --layout NAME [--after S |
  --before S] [--title T]` MUST add one slide from it as one undo step and
  return each placeholder's element ref and purpose.
- **FR-006**: The gallery MUST preselect the layout used last.

**Free placement**

- **FR-007**: The person MUST be able to place an element freely by
  ⌥-dragging it on the canvas, or with "Place freely" (menu, right-click,
  key, `qd block place REF --at X,Y [--size W,H]`).
- **FR-008**: Placing MUST wrap the element in the same positioned block a
  text box uses, leaving the element's own text byte for byte unchanged.
- **FR-009**: A placed element MUST move and resize like a box: drag,
  resize handles, arrow keys to nudge (⇧ for a larger step), and `qd box
  move` / `qd box resize`.
- **FR-010**: "Put back in the flow" (`qd block unplace REF`) MUST remove
  the wrapper, leaving the element's text as it was.
- **FR-011**: Positions and sizes MUST be in the deck's slide units (the
  same numbers Quarto uses for positioned blocks).
- **FR-012**: Placing MUST be unavailable, with the reason given, for: a
  slide's title, a columns container or a single column, an element inside
  a container (column, callout, other div), and speaker notes.
- **FR-013**: An element that appeared on click MUST still appear on click
  after it is placed.

**Saved layouts**

- **FR-014**: "Save slide as layout…" MUST save the slide's structure with
  placeholders in place of its text and images, under a name the person
  gives; replacing an existing name MUST ask first.
- **FR-015**: Saved layouts MUST be kept in either of two places, chosen when
  saving: the person's own (qd's settings folder, available in every deck on
  this machine) or the deck's (a folder beside the deck, shared with
  collaborators and agents working there). The gallery and `qd layouts` show
  both, marked by where they live; a deck's layout of the same name comes
  first for that deck.
- **FR-016**: Saved layouts MUST be listed, renamed and deleted from the
  gallery and from the command line.

**Guides**

- **FR-017**: While a placed element is dragged or resized, guides MUST show
  and snap to the slide's edges and centre and to other elements' edges and
  centres within a small distance.
- **FR-018**: Snapping MUST be turned off by a setting and, for one drag, by
  holding a modifier key.

**Stacking**

- **FR-019**: Bring to front, Send to back, Bring forward and Send backward
  MUST change which placed element is on top, by changing the order of the
  placed elements in the source, as one undo step.

**Throughout**

- **FR-020**: Every new action MUST be in the action list (007) with its
  menu, key where it has one, and the `qd` command it runs.
- **FR-021**: Every change MUST be one `qd` command and one undo step, and
  MUST touch only the bytes it is about (slides and elements around it stay
  byte for byte the same).

### Key Entities

- **Layout**: a name, a one-line description, a preview, where it lives
  (built-in, the person's own, or the deck's), and the markdown it writes for a new slide,
  with its placeholders marked.
- **Placeholder**: an element of a new slide that stands for content to come
  (title, text, image), with a purpose shown to the person ("Click to add
  text") and to agents; it is an ordinary element in the file.
- **Placed element**: an element inside a positioned block, with a position
  and size in slide units; it keeps its own text and attributes.
- **Guide**: a line shown while dragging where an edge or centre lines up
  with the slide's or another element's.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A person makes a two-column slide with a title and text in
  both columns in under 30 seconds, without opening the source.
- **SC-002**: Every built-in layout, added and left unfilled, renders with
  Quarto alone without errors and shows no placeholder words.
- **SC-003**: Placing an element and putting it back leaves the file byte for
  byte as it was, for every kind of element that can be placed.
- **SC-004**: A placed element shows in the same spot (within one slide
  unit) on the canvas, in the presentation and in a plain Quarto render.
- **SC-005**: An agent fills every placeholder of a new layout slide using
  only the refs `qd add --layout` returns, with no need to read the slide
  first.
- **SC-006**: With guides on, dropping an element near the slide's centre
  lands it exactly centred in 9 of 10 tries.
- **SC-007**: Each action in this feature is undone by one ⌘Z.

## Assumptions

- Layouts are for new slides; changing an existing slide's layout
  (re-flowing its content) is out of scope for 009.
- Layouts use only what Quarto's reveal.js format provides (headings,
  columns, positioned blocks, speaker notes); none needs extra stylesheets.
- Layout positions are designed for a 1050×700 slide and scaled to the
  deck's slide size when added.
- Placeholders are kept out of the output by an editor-only marker on the
  element that Quarto ignores when showing the slide (as 008's "Click to add
  title" is editor-only); how is a planning decision.
- Text boxes and image boxes (001) are already placed elements; everything
  in User Story 2 applies to them unchanged.
- Snapping is on by default; its distance is a few points at the canvas's
  zoom.
- Keyboard nudge steps are one slide unit and ten with ⇧, matching common
  slide apps.
- The setting for snapping joins 007's Settings.
