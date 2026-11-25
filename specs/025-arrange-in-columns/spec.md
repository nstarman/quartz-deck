# Feature Specification: Arrange elements inside columns

**Feature Branch**: `025-arrange-in-columns`

**Created**: 2026-11-24

**Status**: Draft

**Input**: User description: "Make columns behave as slide apps treat containers: inside a column, elements reorder, duplicate, delete and insert as on the slide; an element can be moved to the other column or out to the slide, and in from it; an empty column offers to add content; placing an element freely takes it out of its column."

## Context

Spec 024 gave the elements inside a slide's columns refs of their own
(`results/1/0/2`), so they can be selected and given builds. Everything
that rearranges them is refused: moving, duplicating, deleting, inserting
next to them and placing them freely. To change what is in a column, the
presenter has to edit the source.

Slide apps have no columns as such, but they all have containers: a
layout's placeholders, groups, text boxes. They treat them the same way:

- inside a container, things are reordered, copied and deleted as they
  are on the slide;
- an object is dragged from one container to another, or out onto the
  slide, and cut and paste does the same;
- an empty placeholder says what can go in it;
- the presenter never sees ids, and a copy never clashes with its original.

This feature makes qd's columns behave like that.

Decisions proposed for the person (2026-11-24):

- **A column is a small slide.** Move up and down, duplicate, delete,
  insert before and after, cut, copy and paste work on an element inside
  a column as they do on a top-level element, within that column.
- **Moves cross containers.** An element moves next to any other element
  on the same slide: in the other column, at the top level, or the other
  way round. A column can also be named as the target, which puts the
  element at its end, so an empty column can be filled. Moving between
  slides stays cut and paste.
- **A column can be empty.** Moving or deleting a column's last element
  leaves the column, with nothing in it. It stays on the slide, and the
  editor shows "+ Add content" in it.
- **Placing freely takes it out.** Placing an element inside a column
  freely takes it out of the column and puts it on the slide as a
  positioned box, where it was showing.
- **A copied build comes last.** A copy of an element with a build comes
  in on the click after the slide's last build, as slide apps add a copied
  animation at the end.
- **Column layout is a later step.** Setting a column's width and adding or
  removing a column are part of this spec at the lowest priority, and may
  be left for a later one.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Rearrange inside a column (Priority: P1)

The presenter has a list and a short paragraph in the left column. They
select the paragraph and press ⌥↑: it moves above the list. They duplicate
the list, then delete the copy. They right-click the paragraph and insert
a picture after it. Every change stays inside the left column.

**Why this priority**: It's the everyday edit, and it only needs the
slide's existing actions to work one level down.

**Independent Test**: On a two-column slide, use Move up, Duplicate,
Delete and Insert after on an element in a column, and check the file
after each.

**Acceptance Scenarios**:

1. **Given** two elements in a column, **When** the presenter moves the
   second up, **Then** they swap inside the column and nothing outside the
   column changes.
2. **Given** an element in a column, **When** they duplicate it, **Then**
   the copy follows it in the same column, with fresh ids for any explicit
   ids in it.
3. **Given** an element in a column, **When** they delete it, **Then** it
   is gone and the column keeps its other elements.
4. **Given** an element in a column, **When** they insert after it,
   **Then** the new element goes into the same column, right after it.
5. **Given** the first element of a column, **Then** Move up is
   unavailable for it; the last one cannot move down. These moves don't
   leave the column.

---

### User Story 2 — Move between columns and the slide (Priority: P2)

The presenter drags the picture from the right column into the left one,
under the list; the drop line shows where it will land. Then they drag the
paragraph out of the column, under the columns block, onto the slide
itself. Last, they drag a top-level element into the right column, which
is now empty.

**Why this priority**: Moving content between columns is the second
thing people do with a two-column slide, after rearranging inside one.

**Independent Test**: Drag between columns, out to the slide and in from
it, and check the file after each drop. The same moves from the CLI.

**Acceptance Scenarios**:

1. **Given** an element in one column, **When** it is dropped before or
   after an element of the other column, **Then** it moves there, and the
   text of both columns is otherwise unchanged.
2. **Given** an element in a column, **When** it is dropped before or
   after a top-level element, **Then** it leaves the column and becomes a
   top-level element there.
3. **Given** a top-level element, **When** it is dropped next to an element
   in a column, **Then** it moves into that column.
4. **Given** an empty column, **When** an element is dropped on it, **Then**
   it becomes that column's only element.
5. **Given** a cut element, **When** the presenter pastes it after an
   element in a column, **Then** it lands in that column.
6. **Given** a move that would put a columns block inside one of its own
   columns, **Then** qd refuses it and the deck is unchanged.

---

### User Story 3 — Fill an empty column (Priority: P2)

A column has nothing in it, after its last element was moved away or
because the presenter wrote it empty. The editor shows "+ Add content" in
it. Clicking it opens the same insert menu as an empty slide, and the new
element goes into that column.

**Why this priority**: Without it, an empty column can only be filled
from the source, and moving elements out leaves empty columns behind.

**Independent Test**: Empty a column, click "+ Add content", choose a
paragraph, and check it is inside that column.

**Acceptance Scenarios**:

1. **Given** an empty column, **When** the slide shows, **Then** the column
   shows "+ Add content", in the editor only, never when presenting.
2. **Given** "+ Add content", **When** the presenter picks a kind of
   element, **Then** that element is added inside the column.

---

### User Story 4 — Place an element from a column freely (Priority: P3)

The presenter wants the picture from the right column to sit over the
corner of the slide. They choose Place freely on it. It leaves the column
and becomes a positioned box where it was showing; they then drag it to
the corner.

**Why this priority**: Less common than rearranging, but it is the last
"inside columns" refusal from spec 024 that people meet.

**Independent Test**: Place an element of a column freely, and check the
column no longer has it and a box with its text is on the slide at its
old position.

**Acceptance Scenarios**:

1. **Given** an element in a column, **When** it is placed freely, **Then**
   it is removed from the column and added to the slide as a positioned box,
   at the place and width it was showing.
2. **Given** that box, **When** the presenter puts it back in the flow,
   **Then** it becomes a top-level element of the slide (not back in the
   column).

---

### User Story 5 — Column widths and number (Priority: P4)

With a column selected, the Format view shows its width. The presenter
changes it from 50% to 40%. They add a third column, and later remove it.

**Why this priority**: Widths and counts are set once per slide and are
quick to change in the source; the other stories come first.

**Independent Test**: Change a column's width and add and remove a column
from the Format view and from the CLI, checking the file.

**Acceptance Scenarios**:

1. **Given** a selected column, **When** the presenter sets its width,
   **Then** the column's `width` attribute changes and nothing else.
2. **Given** a selected columns block, **When** they add a column, **Then**
   an empty column is added at the end.
3. **Given** a column, **When** they delete it, **Then** it goes with
   everything in it, and the columns block keeps the others.

---

### User Story 6 — The CLI does all of it (Priority: P1)

Every move above has a CLI form: `qd block move`, `dup`, `rm` and `add`
with nested refs, `qd block move ELEMENT --into COLUMN`, `qd block place`
on a nested ref, and the column width, add and remove operations.

**Why this priority**: The editor writes through these ops, and agents use
them directly.

**Independent Test**: Run each operation on a two-column fixture and check
the resulting text and JSON.

**Acceptance Scenarios**:

1. **Given** a nested ref, **When** `qd block move`, `dup`, `rm` or `add`
   runs on it, **Then** it acts inside the element's column.
2. **Given** refs in different containers on one slide, **When** `qd block
   move A --before B` runs, **Then** A moves next to B, in B's container.
3. **Given** a column ref, **When** `qd block move A --into COLUMN` runs,
   **Then** A ends up last in that column.
4. **Given** refs on two different slides, **Then** `qd block move` refuses,
   as it does today.

### Edge Cases

- A column holding columns: moves work at every depth, and a block can't
  move into itself.
- Moving the only element of a slide into a column: the slide has the
  columns block left.
- A ref changes when an element moves; the result reports the element's
  new ref, and the editor keeps it selected.
- Text between columns in a `.columns` block outside any `.column`: it is
  an element of the columns block and moves like any other.
- An element with a build moved to another container keeps its build and
  its place in the build order.
- Duplicating a whole columns block renews every explicit id inside it.
- Columns written with the shorthand `::: column` keep their form when
  their contents change.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Move, duplicate, delete and insert-before/after MUST accept
  an element inside a column and act within that column.
- **FR-002**: A move MUST accept a target in another container on the same
  slide (another column, the top level, or a column from the top level)
  and put the element next to that target, in the target's container.
- **FR-003**: A move MUST accept a column as the target and put the element
  at the end of that column, including when the column is empty.
- **FR-004**: A move that would put an element inside itself MUST be
  refused, leaving the deck unchanged.
- **FR-005**: Moving or deleting a column's last element MUST leave the
  column, empty.
- **FR-006**: The editor MUST show "+ Add content" in an empty column, and
  what is added from it MUST go into that column.
- **FR-007**: Dragging an element MUST show where it will land, in any
  container on the slide, and dropping it MUST make that move.
- **FR-008**: Paste after or before an element in a column MUST put the
  pasted content in that column.
- **FR-009**: Placing an element of a column freely MUST take it out of the
  column and add it to the slide as a positioned box at the place and width
  it was showing.
- **FR-010**: Every copy (duplicate, paste, slide duplicate) MUST give fresh
  ids to the explicit ids inside it, at every depth.
- **FR-011**: A copy of an element with a build MUST come in on the click
  after the slide's last build.
- **FR-012**: A column's width MUST be settable, and a column addable to
  and removable from a columns block (P4).
- **FR-013**: Every op MUST change only the text it moves or edits; the rest
  of the deck stays byte for byte.
- **FR-014**: Each op's result MUST report the moved or added element's ref
  after the change.

### Key Entities

- **Container**: the list an element sits in: a slide's top level, a
  columns block, or a column.
- **Element ref**: as in spec 024, `SLIDE/N[/M…]` or `#id`.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: On a two-column slide, the presenter swaps two elements in a
  column, moves one to the other column, and fills an emptied column, in
  under a minute and without opening the source.
- **SC-002**: No action in an element's menu says "inside columns" any more
  (except those in User Story 5 if it is deferred).
- **SC-003**: Each CLI scenario in User Story 6 passes as an automated test;
  scenarios of User Stories 1–3 pass as browser tests.
- **SC-004**: Every existing test passes, except those that checked the
  spec 024 refusals, which change to check the new behaviour.

## Assumptions

- The person's real talk uses two-column slides of text beside a figure;
  that is the case to get right first.
- Moving elements between slides by drag stays out of scope: cut and paste
  covers it.
- A column's width is the `width` attribute Quarto documents for `.column`.
- Arrows (spec 016) keep pointing at what they pointed at; an arrow whose
  end moves is not redrawn by this feature.
