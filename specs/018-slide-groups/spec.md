# Feature Specification: Slide groups

**Feature Branch**: `018-slide-groups`

**Created**: 2026-11-17

**Status**: Draft

**Input**: User description: "Slide groups in qd's slide list, like Keynote's: a group is a Quarto `#` section (head = the `#` slide, sub-slides = the `##` slides up to the next `#`), shown indented under its head and collapsible in the list (collapse state kept by the editor, not in the .qmd). Indent/outdent (Tab / Shift-Tab, menu items, drag sideways) demote/promote headings. Drag, copy, cut and delete of a collapsed or picked head take its sub-slides with it. A deck setting for Quarto's navigation-mode (linear/vertical/grid). Numbering stays one running 1, 2, 3. One level deep. Title slide stays fixed first. No group-wide hide."

## Context

A talk has parts: a motivation, a method, three results, a summary. Today
qd's slide list is one flat column, so a 60-slide deck is a long scroll,
and moving a whole part means picking every one of its slides by hand.
Keynote solves this with sub-slides: a slide can sit indented under the
slide above it, and the parent can be collapsed so its children don't take
up room.

Quarto already has this structure. With the default `slide-level: 2`, a
`#` heading makes a section slide, and every `##` slide after it, up to the
next `#`, belongs to that section. The rendered reveal.js HTML wraps each
section and its slides in one vertical stack, whatever the deck's
navigation mode (checked by rendering a small deck with Quarto 1.11:
`<section><section id="part-a" class="title-slide slide level1">…</section>
<section id="a1" class="slide level2">…</section>…</section>`). Quarto's
`navigation-mode` option then decides how a presenter moves through them:
`linear` (the default: → walks every slide in order), `vertical` (→ goes
from section to section, ↓ through a section's slides) or `grid`.

qd today reads `#` slides as section slides but shows them in the list like
any other slide. This feature shows that structure in the list and lets the
presenter change it there, writing nothing but the headings Quarto already
reads.

Decisions made with the person:

- A group is a Quarto `#` section. No new syntax and no qd attribute.
- The presentation keeps the deck's own navigation mode; qd adds a deck
  setting to choose linear, vertical or grid.
- The list gets collapse and expand, moving, copying and deleting a group
  as one, and indent and outdent.
- Hiding stays per slide; there is no "hide this group".
- Slide numbers in the list stay one running count (1, 2, 3, …).
- The title slide made from the front matter stays fixed first, outside
  any group (as decided for the slide list just before this spec).

## User Scenarios & Testing *(mandatory)*

### User Story 1 — See a deck's parts in the list (Priority: P1)

The presenter opens a deck that already has `#` sections. In the slide
list, each section slide is a group head, and the slides under it are
indented beneath it. A small triangle on the head collapses the group, so
only the head shows; clicking it again opens it. The next time the deck is
opened in the editor, the same groups are still collapsed.

**Why this priority**: seeing the parts, and folding away the ones not
being worked on, is most of the value of groups, and it changes nothing in
the file.

**Independent Test**: open a deck with two `#` sections of three `##`
slides each; the list shows two heads with three indented slides under
each; collapsing one leaves four rows, and the `.qmd` is unchanged.

**Acceptance Scenarios**:

1. **Given** a deck with `#` sections, **When** it opens in the editor,
   **Then** each `#` slide is shown as a group head and the `##` slides up
   to the next `#` are indented under it; slides before the first `#` are
   not indented.
2. **Given** an open group, **When** the presenter clicks its triangle (or
   presses ← on the focused head), **Then** its sub-slides are hidden in
   the list; the triangle (or →) opens it again.
3. **Given** a collapsed group, **When** the slide shown on the canvas is
   one of its sub-slides (from the source pane, an agent, or the
   presentation), **Then** the group opens so that slide is visible in the
   list.
4. **Given** groups collapsed in one session, **When** the deck is opened
   again in the editor, **Then** the same groups are collapsed; the `.qmd`
   holds nothing about it.
5. **Given** groups, **When** the list shows slide numbers, **Then** they
   are one running count over heads and sub-slides, as before.

---

### User Story 2 — Make and undo groups: indent and outdent (Priority: P1)

The presenter has a flat deck and wants the three result slides under one
"Results" slide. They select a result slide and press Tab: it becomes a
sub-slide of the slide above it. ⇧Tab brings a sub-slide back out. The
slide menu has the same two items, and dragging a slide a little to the
right or left while moving it in the list does the same. The file changes
only in the slides' headings: `##` becomes `#` or the other way round.

**Why this priority**: without a way to make groups in the list, a
presenter has to edit headings in the source by hand.

**Independent Test**: on a flat deck of `##` slides A, B, C, D, indent C
under B; B's heading becomes `#`, C stays `##`, and the list shows C
indented under B with D also under B (as Quarto reads it). Undo restores
the file byte for byte.

**Acceptance Scenarios**:

1. **Given** a group head with a group above it, **When** the presenter
   indents it, **Then** its heading becomes `##` and it and its own
   sub-slides join the group above; the list shows the result.
2. **Given** a top-level slide directly below another top-level slide
   (before the first `#`), **When** the presenter indents it, **Then** the
   slide above becomes a group head (its heading becomes `#`) and this
   slide is its sub-slide; the editor says the head now renders as a
   section slide.
3. **Given** a sub-slide, **When** the presenter outdents it, **Then** its
   heading becomes `#`: it is now a group head, and the sub-slides that
   followed it in the old group are now its sub-slides (as Quarto reads
   the file).
4. **Given** a slide being dragged in the list, **When** the pointer moves
   sideways past a small threshold at the landing place, **Then** the drop
   marker shows the slide landing indented or not, and dropping writes
   that.
5. **Given** an indent or outdent that isn't possible (the first slide, a
   slide with nothing above it but the title slide, a sub-slide on indent,
   a head on outdent, the title slide), **When**
   the presenter tries it, **Then** nothing changes and the menu item is
   shown disabled.
6. **Given** the same change made with a command, **When** it runs,
   **Then** the result is identical to the gesture, and it is one step of
   undo.

---

### User Story 3 — Move, copy and delete a group as one (Priority: P2)

The presenter wants the whole "Methods" part after "Results". They drag the
collapsed Methods head below the Results group, and Methods moves with all
its sub-slides. Copy, cut and Delete on a collapsed head act on the whole
group too. On an open group, the head alone moves, as in Keynote, unless
the presenter picks the group's slides.

**Why this priority**: reordering parts is common when shaping a talk, but
picking every slide of a part (spec 008 US9) already works, more slowly.

**Independent Test**: collapse a group of four slides and drag it after
the next group; one move puts the four slides there in the same order, the
rest of the file is byte-identical, and one undo puts it back.

**Acceptance Scenarios**:

1. **Given** a collapsed head, **When** the presenter drags it, **Then** the
   drag image shows the group (the head with a count of its sub-slides),
   and dropping moves the head and its sub-slides together, in order.
2. **Given** a group being dragged, **When** it is over the middle of
   another group or among the top-level slides, **Then** it can't land
   there; the drop marker shows it landing at the nearest place where the
   next slide is a head, or at the end.
3. **Given** a collapsed head, **When** the presenter copies, cuts,
   duplicates or deletes it, **Then** the action covers the head and all
   its sub-slides, as one step of undo; pasting puts the whole group back,
   after the end of the group the paste target is in (or after the last
   top-level slide, if the target is one).
4. **Given** an open head, **When** the presenter moves or deletes it
   alone, **Then** only the head goes, and its former sub-slides now
   belong to the group before them (or become top-level slides if there is
   none); the editor says so.
5. **Given** a head and its sub-slides picked with ⇧-click, **When** the
   presenter drags, copies or deletes, **Then** the action covers them
   all, as for any picked slides today.

---

### User Story 4 — Choose how the presentation moves through groups (Priority: P3)

The presenter wants ↓ to step through a section's slides and → to jump to
the next section. A deck setting offers Quarto's three navigation modes:
linear, vertical and grid. Choosing one writes the deck's
`navigation-mode` option; the editor's own slide list is the same in all
three.

**Why this priority**: groups work in the default linear mode; this is for
presenters who want reveal.js's two-dimensional navigation.

**Independent Test**: set the mode to vertical; the deck's front matter
gains `navigation-mode: vertical` where its reveal.js options are, and in
the presentation ↓ goes to the next sub-slide; setting it back to linear
(the default) removes the option.

**Acceptance Scenarios**:

1. **Given** a deck, **When** the presenter sets the navigation mode,
   **Then** only that option changes in the front matter, written beside
   the deck's other reveal.js options, and the setting shows the mode the
   file has.
2. **Given** the mode set to linear, **When** it is saved, **Then** the
   option is removed rather than written, since linear is Quarto's
   default.

---

### Edge Cases

- **Slides before the first section.** They are top-level slides, shown
  without indent. After the first `#`, every `##` slide belongs to a group,
  because that is how Quarto reads the file; a slide can only stand alone
  there by being a `#` slide itself (a group of one).
- **A head with content.** A `#` slide may have its own content; it is
  shown and edited as any slide. Quarto renders it as a section slide
  (centred); the list's thumbnail shows it as rendered.
- **Hidden slides.** Hiding stays per slide. Quarto hides only the slide
  marked hidden: a hidden head's sub-slides still show in the
  presentation. The list tags each hidden slide as today.
- **Rule-started slides** (`---`) inside a section belong to that section,
  as Quarto reads them; outdenting one gives it an empty `#` heading,
  which Quarto shows as an empty section slide, and the editor says so.
- **Other slide levels.** With `slide-level: 1` every `#` is a plain slide
  and there are no groups; indent and outdent are disabled with the
  reason. With `slide-level: 3` or more, the groups are the top-level
  sections (the stacks reveal.js shows), and section slides in between are
  shown as sub-slides; indent and outdent change headings between those
  two levels only.
- **One level deep.** reveal.js stacks are two-dimensional, so a group
  can't hold a group. Indenting a head whose group would end up inside
  another group merges its sub-slides into that group, and the editor says
  so.
- **Explicit ids and attributes** on a heading are kept when indenting or
  outdenting changes its level; only the `#` marks change. A setext
  heading (underlined with `===` or `---`) is changed to the matching
  underline.
- **A group collapsed in one editor** is open in another window or for
  another person: collapse is a view setting, not part of the deck.
- **Slides removed or renamed outside qd** while collapsed: a group whose
  head is gone is simply open; nothing breaks.
- **Opaque slides** (segmentation was unsure, structural edits refused)
  refuse indent and outdent with the same reason as other structural
  edits.
- **A narrow window's slide picker** (spec 008 US8) shows sub-slides
  indented in its menu; it has no collapse.

## Requirements *(mandatory)*

### Functional Requirements

**Groups in the deck**

- **FR-001**: A group MUST be a Quarto section: its head is a slide whose
  heading is at the deck's top section level (`#` with the default
  `slide-level: 2`), and its sub-slides are the slides after it up to the
  next such heading, as in the vertical stacks Quarto renders.
  qd MUST NOT add any syntax, attribute or comment to mark groups.
- **FR-002**: Reading a deck MUST report, for each slide, whether it is a
  group head, a sub-slide (and of which head), or a top-level slide. `qd
  show` (and the deck the editor reads) MUST include this.
- **FR-003**: Indent and outdent MUST change only the heading marks of the
  slides involved; every other byte of the file stays as it was, and undo
  restores it byte for byte.
- **FR-004**: Every structural action of this spec (indent, outdent, move,
  copy, cut, duplicate and delete of a group) MUST have a `qd` command, with the same
  result as the gesture, as one undo step. Command names are settled in
  the plan.

**The slide list**

- **FR-005**: The list MUST show sub-slides indented under their head, and
  top-level slides and heads at the left edge. The title slide stays first
  and outside any group.
- **FR-006**: A head with sub-slides MUST have a control to collapse and
  expand it, reachable by pointer and by keyboard (← collapses a focused
  open head, → expands a collapsed one), with its state announced to
  assistive technology.
- **FR-007**: Collapse state MUST be kept by the editor for that deck
  across sessions on the same machine, and MUST NOT be written to the
  `.qmd`. It is view state, like the list's scroll position.
- **FR-008**: When the slide shown on the canvas is inside a collapsed
  group, the list MUST open that group.
- **FR-009**: Slide numbers in the list MUST stay one running count over
  all slides, heads included.
- **FR-010**: ↑ and ↓ in the list MUST move through the visible rows only,
  skipping the sub-slides of collapsed groups.

**Indent and outdent**

- **FR-011**: Tab (and ⌘]) on a focused slide in the list MUST indent it
  and ⇧Tab (and ⌘[) MUST outdent it; Escape MUST move focus from the list
  back to the canvas, since Tab no longer leaves the list. The slide menu
  MUST offer both, disabled with a reason when the action isn't possible.
- **FR-012**: Indenting MUST follow Quarto's reading of the file:
  indenting a head makes it a sub-slide of the group above (its own
  sub-slides join that group); indenting a top-level slide under another
  top-level slide makes that slide a head.
- **FR-013**: Outdenting a sub-slide MUST make it a head; the sub-slides
  after it in the old group become its sub-slides.
- **FR-014**: While dragging a slide in the list, moving the pointer
  sideways past a threshold MUST switch the landing between indented and
  not, shown by the drop marker before the drop.
- **FR-015**: When an indent, outdent or drop changes how a slide renders
  (a slide becoming a head renders as a section slide) or regroups other
  slides, the editor MUST say so in a short message after the change.

**Groups as one**

- **FR-016**: Dragging, copying, cutting, duplicating or deleting a
  collapsed head MUST act on the head and all its sub-slides, as one step
  of undo.
- **FR-017**: Anything that leaves a head at its landing place (a group
  moved, pasted or duplicated, an open head moved alone, or a slide
  dropped outdented) MUST land only where the slide after it is a head or
  at the end of the deck. A `#` placed anywhere else would take the slides
  after it into its group (among the top-level slides, or between two
  sub-slides of another group). Pasting or duplicating a group on a slide
  inside a group puts it after the end of that group; on a top-level slide,
  after the last top-level slide, just before the first head. A group
  dropped indented becomes sub-slides and takes nothing with it, but
  follows the same rule to keep one rule. Outdenting a sub-slide in place
  (FR-013) is the one way to split a group.
- **FR-018**: Acting on an open head alone MUST act on the head only; its
  former sub-slides then belong to the group before them, or are top-level
  if none, and the editor says so.
- **FR-019**: Picking several slides (spec 008 US9) MUST keep working
  across groups; a pick that includes a collapsed head includes its
  sub-slides.

**Presentation**

- **FR-020**: A deck setting MUST offer Quarto's navigation modes (linear,
  vertical, grid), showing the mode the file has, and MUST write only the
  `navigation-mode` option, beside the deck's other reveal.js options in
  the front matter; linear removes the option.
- **FR-021**: The navigation mode MUST NOT change the slide list.

### Key Entities

- **Group**: a section of the deck: a head slide and the sub-slides after
  it up to the next head. Defined entirely by heading levels in the
  `.qmd`.
- **Head**: a section slide at the deck's top section level (`#` by
  default); it may have its own content.
- **Sub-slide**: a slide at the slide level that comes after a head.
- **Collapse state**: per deck, per machine, which heads are collapsed in
  the list. Kept by the editor, not in the deck.
- **Navigation mode**: the deck's reveal.js `navigation-mode` option.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A deck with sections opens with its groups shown, with no
  change to the file (byte-identical before and after opening, collapsing
  and expanding).
- **SC-002**: A presenter can turn a flat run of slides into a group, or
  move a whole group of ten slides to another place, in under 10 seconds,
  each as one undo step.
- **SC-003**: Every indent, outdent and group move changes only heading
  marks and the moved slides' position; the rest of the file is
  byte-identical, checked by tests over the existing deck corpus.
- **SC-004**: A deck grouped in qd renders with plain Quarto, and its
  rendered vertical stacks match the groups the list shows, in every
  navigation mode.
- **SC-005**: Every structural action on groups has a `qd` command that
  produces the same file as the gesture.
- **SC-006**: One of the person's real talks with several parts can be
  regrouped in the list without editing the source by hand.

## Assumptions

- **Where collapse state lives**: in the editor's own per-deck storage on
  the machine (as other editor view settings are kept), so it never
  reaches the `.qmd` (constitution II: the file holds only what it can
  express). It is not an edit, so it has no `qd` command.
- **Where the navigation setting sits**: in the Slide menu, next to
  Transition (plan, research R6). qd writes `navigation-mode` under `format: revealjs:` when the
  deck keeps its options there, and at the top level otherwise, matching
  where the deck already keeps similar options.
- **Keynote's behaviour** on outdenting a slide in the middle of a group
  is matched by what the file means anyway: the slides after it become its
  sub-slides.
- **The drop threshold** for indenting while dragging is about one indent
  width (the plan settles the number), so a plain vertical drag never
  changes depth by accident.
- **Out of scope**: groups inside groups, hiding or skipping a whole group,
  outline numbers (3.1, 3.2), a separate outline view, and changing
  `slide-level` from the list.
