# Feature Specification: Builds inside columns

**Feature Branch**: `024-builds-in-columns`

**Created**: 2026-11-23

**Status**: Draft

**Input**: User description: "Elements inside a slide's `:::: {.columns}` / `::: {.column}` divs can be selected on the slide and given builds (appear/leave, order) from the properties column's Animate view, like top-level elements."

## Context

qd addresses the elements of a slide by their place among the slide's
top-level blocks: `results/0`, `results/1`, and so on. A columns block is
one top-level block, so everything inside it is one element. On a
two-column slide the presenter can make the whole pair of columns appear
on a click, but not the list in the left column, and not the picture in
the right one. Trying spec 023's builds on a real talk ran into this on
most content slides, since a talk puts text beside a figure all the time.

This feature lets the presenter reach the elements inside columns, on the
slide and from the CLI, and gives them builds there as on any other
element.

Decisions proposed for the person (2026-11-23):

- **Only columns open up.** A `.columns` div holds its `.column` divs, and
  each `.column` div holds the elements written in it. Every other div
  stays one element, as today: a `::: {.fragment}` wrapper that qd wrote
  around a list, a callout, a placed box.
- **A ref is a path.** An element inside columns is named by its slide and
  the place at each level: `results/1/0/2` is the third element of the
  first column of the slide's second block. `results/1/0` is that column
  itself. Top-level refs (`results/1`) and `#id` refs keep working as they
  are, and an `#id` given to an element inside a column finds it.
- **A click picks the innermost element.** Clicking the list in a column
  selects the list. Escape steps out: from the list to its column, from
  the column to the columns block, from there to nothing selected, as
  Escape does today.
- **Builds first.** The ops that builds need take nested refs: setting an
  element's build, listing a slide's builds, ordering them, and adding or
  removing a class (which "one item at a time" uses). Editing an
  element's text takes them too, since it changes only that element's
  text. Moving, duplicating, deleting and placing a nested element are
  refused with a message saying so, for now.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Build the elements of a two-column slide (Priority: P1)

The presenter has a slide with a list in the left column and a picture in
the right. They click the list; it is selected, and the Animate view shows
its appear and leave. They choose Fade in. They click the picture and
choose Fade up. The slide's build list shows both, the list first. They
drag the picture above the list. When they present the slide, the picture
comes in on the first click and the list on the second.

**Why this priority**: This is the case that failed on the real talk, and
the whole feature exists for it.

**Independent Test**: Open a deck with a two-column slide in the editor,
give the two elements builds from the Animate view, reorder them, then
step through the slide in the presented deck and check what shows after
each click.

**Acceptance Scenarios**:

1. **Given** a two-column slide, **When** the presenter clicks the list in
   the left column, **Then** the list alone is selected (not the columns
   block), and the Animate view shows its build controls.
2. **Given** the list is selected, **When** they choose an appear effect,
   **Then** the deck's text gives the list that build, and the text
   outside the list is unchanged byte for byte.
3. **Given** both elements have builds, **When** the presenter looks at
   the build list, **Then** it shows them in the order they will show,
   named by their text or picture description.
4. **Given** the build list, **When** they drag the second build above
   the first (or use ⌥↑), **Then** the presented slide shows them in the
   new order.
5. **Given** the list in a column is selected, **When** they press
   Escape, **Then** its column is selected; Escape again selects the
   columns block; again, nothing.
6. **Given** a list inside a column, **When** they tick "One item at a
   time", **Then** the list's items come in one per click when presenting.

---

### User Story 2 — The same from the CLI (Priority: P2)

An agent or a script works on the same deck without the editor. `qd show`
lists a slide's elements with the ones inside columns, each with its ref.
`qd build set results/1/0/0 --appear fade-in` and `qd build set
results/1/1/0 --appear fade-up` give them builds; `qd build list results`
lists them; `qd build order results results/1/1/0 results/1/0/0` puts the
picture first.

**Why this priority**: Every editor change goes through a CLI op, and
agents drive qd from the command line. The editor story cannot work
without this one, so it is also the first to build.

**Independent Test**: Run the commands above on a fixture deck and check
the resulting text and JSON.

**Acceptance Scenarios**:

1. **Given** a two-column slide, **When** `qd show` reads it, **Then**
   the columns block lists its columns and each column lists its
   elements, each with a ref.
2. **Given** a nested ref, **When** `qd build set` gives that element a
   build, **Then** only that element's text changes, and `qd build list`
   reports the build under that ref.
3. **Given** builds at the top level and inside columns on one slide,
   **When** `qd build order` names them all, **Then** they show in that
   order, and naming them in the order they are written removes every
   `fragment-index` again.
4. **Given** a nested ref, **When** `qd block class` adds or removes a
   class, **Then** it changes that element only.
5. **Given** a nested ref, **When** `qd block move`, `dup`, `rm` or
   `place` is run on it, **Then** qd refuses with a message that names
   the limit and leaves the deck as it was.
6. **Given** a ref that points past the end of a column, or into an
   element that is not columns, **Then** qd answers that the element is
   not found.

---

### User Story 3 — The showcase deck shows it (Priority: P3)

The showcase deck gains a two-column slide whose left column has a list
and whose right column has a picture, each with a build, in an order that
differs from the order they are written.

**Why this priority**: The showcase is how the person checks each new
build of qd by hand.

**Independent Test**: Open the showcase in the editor and in the
presented deck; the slide's builds list and present as written.

**Acceptance Scenarios**:

1. **Given** the showcase deck, **When** the presenter steps through the
   new slide, **Then** the elements come in in the order set by the build
   list, not the order they are written.

### Edge Cases

- A columns block that itself has a build, with builds inside it too:
  both are listed, and reveal.js shows the block first unless an order
  says otherwise.
- Columns inside a column: refs go one level deeper, and the same rules
  apply.
- Text in a `.columns` div outside any `.column` div: it is an element of
  the columns block like a column is.
- A column with nothing in it: it is still selectable, and has no
  elements.
- An element inside a column with an explicit `{#id}`: `#id` finds it,
  and it is reported by that id, as top-level elements are.
- A code cell inside a column keeps rendering as it does today.
- Editing a nested element's text so that it would close its column or
  start a new slide is refused, as editing any element is.
- A columns block written with the shorthand `:::: columns` (no braces)
  opens up like `:::: {.columns}`.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The deck model MUST give a `.columns` div its contents as
  elements, and a `.column` div its contents as elements, at any depth;
  no other kind of div opens up.
- **FR-002**: An element inside columns MUST have a ref made of its
  slide's ref and its place at each level, separated by `/`, and an
  explicit `{#id}` on it MUST resolve it anywhere in the deck.
- **FR-003**: Top-level refs, `#id` refs and every op on top-level
  elements MUST behave exactly as before.
- **FR-004**: `qd build set`, `qd build list`, `qd build order`, `qd block
  class` and `qd edit` MUST accept nested refs; `qd build list` and `qd
  build order` MUST include builds at every depth, in the order the slide
  shows them.
- **FR-005**: Ops that move, copy, delete or place an element MUST refuse
  a nested ref with a clear message and leave the deck unchanged.
- **FR-006**: `qd show` (and the slide data the editor reads) MUST list the
  elements inside columns with their refs.
- **FR-007**: In the editor, a click MUST select the innermost element
  under the pointer, its outline MUST show on that element only, and the
  Animate view MUST show and change that element's build.
- **FR-008**: Escape on a nested element MUST select the element that
  holds it; on a top-level element it clears the selection as today.
- **FR-009**: Actions the editor offers for an element that the CLI
  refuses for a nested ref MUST be shown as unavailable for one, with the
  reason, rather than failing after a click.
- **FR-010**: The rendered deck MUST look and present the same as before
  for slides whose text qd did not change.
- **FR-011**: Every change MUST leave the rest of the deck's text unchanged
  byte for byte.

### Key Entities

- **Element**: a block of a slide, at the top level or inside columns. It
  has a ref, a kind, its text's place in the file, and, for a columns or
  column div, the elements inside it.
- **Ref**: `SLIDE/N` for a top-level element, `SLIDE/N/M/...` inside
  columns, or `#id` for an element with an explicit id.
- **Build**: as in spec 023, an element's appear and leave effects and its
  place in the slide's order.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: On a two-column slide, the presenter gives the list and the
  picture a build each and swaps their order in under a minute, without
  opening the source.
- **SC-002**: The presented slide shows the two elements in the order the
  build list gives, on two clicks.
- **SC-003**: Each CLI scenario in User Story 2 passes as an automated
  test, and the editor scenarios 1–5 of User Story 1 pass as browser
  tests.
- **SC-004**: Every existing test passes unchanged, except where a test
  read a columns block's whole text as one element on purpose.

## Assumptions

- Columns are written the way Quarto documents them: a `.columns` div
  containing `.column` divs. Other side-by-side layouts (placed boxes,
  `layout-ncol` figures) are out of scope.
- Moving an element between columns, and adding an element to a column
  from the editor, are left for a later spec; the source pane covers them.
- Arrows (spec 016) and placed boxes keep addressing top-level elements.
- The presented order follows reveal.js: `fragment-index` where set, then
  the order in the page, which for nested elements is the order they are
  written.
