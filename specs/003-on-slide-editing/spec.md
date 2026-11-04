# Feature Specification: On-Slide Editing

**Feature Branch**: `003-on-slide-editing`

**Created**: 2026-11-04

**Status**: Draft

**Input**: User description: "Direct on-slide editing in the quartz-deck browser editor: instead of filling in a side-panel form, the user edits the actual slide elements in the live preview. Click a title, paragraph, list item or text box on the slide and type in place; edit background colour/image, transitions and classes from small controls attached to the slide/element rather than a form; add, delete and reorder elements on the slide directly. Every change is still written to the .qmd as the minimal source edit (the .qmd stays the source of truth, byte-identical outside the edited span), goes through the same qd ops as the CLI (so Claude in chat and the person in the browser can edit simultaneously with --expect conflict protection), and works with undo/redo. Content that cannot be edited faithfully as rich text (code cells, math, raw HTML, shortcodes, complex inline markup) falls back to editing its markdown source in place, over the element. The side panel stays available as an advanced/source view but is no longer the primary way to edit."

## Clarifications

### Session 2026-11-04

- Q: How much formatting can the person apply while typing on the slide? →
  A: A full formatting toolbar (word-processor style): bold, italic,
  underline, strikethrough, code, link, headings levels inside the slide,
  bulleted/numbered lists and indentation, text colour and highlight,
  alignment, font size steps. Each is written as the markdown construct for
  it, or — where markdown has none (colour, alignment, size) — as a Pandoc
  span/div with a class or style on just the affected text.
- Q: What happens to the form side panel? → A: Hidden by default; a "Source"
  toggle opens it as the slide's markdown and advanced properties.
- Q: Which element types does Insert offer? → A: Paragraph, list, image,
  text box and equation; code cells, columns and callouts are inserted via
  source editing for now.

## Context

Today the editor shows the rendered slide in the middle and a form on the
right: to change a title the person finds the "Title" field, types there and
presses Apply, then looks back at the slide to see the result. Only
positioned boxes can be manipulated on the slide itself (drag and resize,
spec 001 US4). The person's attention is split between two places, and the
form says nothing about where on the slide a field's content appears.

This feature makes the slide the editing surface: what you see is what you
click and type into. The `.qmd` file remains the only source of truth
(constitution principle II): every on-slide change becomes the smallest
source edit that expresses it, through the same operations the CLI offers
(principle III), so an agent in chat and a person in the browser keep editing
the same deck at the same time (principle V).

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Edit text where it appears (Priority: P1)

The person clicks a slide's title, a paragraph, a list item or a text box in
the preview, types, and leaves the element. The change is saved to the
`.qmd` and the slide re-renders with it, without the person touching a form.

**Why this priority**: Changing words is by far the most common edit, and it
is the one the form makes most awkward. Alone it already turns the editor
from a form into a slide editor.

**Independent Test**: In a deck with a title, a paragraph and a bulleted list,
change each one on the slide; the `.qmd` diff shows only those spans changed,
and the re-rendered slide shows the new text.

**Acceptance Scenarios**:

1. **Given** a slide titled "Methods", **When** the person clicks the title on
   the slide, replaces it with "Methods and data" and clicks elsewhere,
   **Then** the heading line in the `.qmd` reads `## Methods and data` with
   its attributes unchanged, and nothing else in the file changes.
2. **Given** a paragraph with **bold** and a [link](https://example.org),
   **When** the person edits a plain word in it, **Then** the bold and the
   link are kept exactly as written in the source.
3. **Given** a bulleted list, **When** the person edits the second item and
   presses Enter at its end, **Then** a new empty item is added after it and
   can be typed into.
4. **Given** the person is editing an element, **When** they press Escape,
   **Then** the edit is abandoned and the file is unchanged.
5. **Given** an edit was saved, **When** the person presses Undo (or the agent
   runs `qd undo`), **Then** the previous text is restored on the slide and in
   the file.

---

### User Story 2 — Edit what can't be typed into as source, in place (Priority: P1)

For an element that cannot be edited faithfully as rendered text — math, a
code cell, raw HTML, a shortcode, a figure with options, a callout, complex
inline markup — clicking it opens its markdown source in an editor drawn over
the element on the slide, at its position. Saving writes that source back and
the slide re-renders.

**Why this priority**: Without it, every slide that contains one such element
would push the person back to the form, defeating US1; Quarto decks routinely
contain math and code.

**Independent Test**: On a slide with an equation and a code cell, edit each
via its in-place source editor; the `.qmd` changes only in those spans and the
slide shows the updated equation and code.

**Acceptance Scenarios**:

1. **Given** a slide with display math, **When** the person clicks it,
   **Then** an in-place source editor opens over it showing the TeX source,
   and saving re-renders the equation.
2. **Given** any element at all, **When** the person chooses "Edit source" on
   it, **Then** they get the same in-place source editor, even for elements
   that support typing directly.
3. **Given** the source the person saves would break the slide structure
   (e.g. it adds a new slide heading), **When** they save, **Then** the edit
   is refused with a message saying why, and their text is kept in the
   editor so it is not lost.

---

### User Story 3 — Change slide and element properties from controls on the slide (Priority: P2)

Selecting a slide or an element shows a small toolbar attached to it with its
common properties: for a slide, background colour, background image,
transition and visibility; for an element, its classes (e.g. `.fragment`,
`.smaller`, `.incremental`) and delete. Changes apply immediately and are
written to the `.qmd` like any other edit.

**Why this priority**: Properties are less frequent than text but still the
main reason the form exists today; once text and properties are on the slide,
the form is no longer needed for everyday work.

**Independent Test**: Set a background colour, a transition and a `.fragment`
class from the on-slide controls; the `.qmd` heading and element attributes
change accordingly and the preview reflects each change.

**Acceptance Scenarios**:

1. **Given** a slide with no background, **When** the person picks a colour in
   the slide toolbar, **Then** the slide heading gains
   `background-color="<colour>"` and the preview shows it.
2. **Given** a paragraph, **When** the person turns on "Appears on click"
   (fragment) in its toolbar, **Then** the paragraph gains the `.fragment`
   class in the source and appears on the next step in the preview.
3. **Given** a slide hidden from the presentation, **When** the person views
   it, **Then** the toolbar shows that it is hidden and lets them unhide it.

---

### User Story 4 — Add, delete and reorder elements on the slide (Priority: P2)

The person adds a new paragraph, list, image or text box at a chosen place on
the slide, deletes an element, or drags an element above or below another
one, all on the slide.

**Why this priority**: Restructuring a slide is common but less frequent than
editing what is there; until this exists it can still be done through source
editing (US2).

**Independent Test**: Add a list after the first paragraph, move it above
the paragraph, then delete the paragraph; the `.qmd` shows exactly those
structural changes and nothing else.

**Acceptance Scenarios**:

1. **Given** a slide with two paragraphs, **When** the person uses the insert
   control between them and picks "List", **Then** an empty list item appears
   there, ready for typing, and the source gains a list block in that place
   once the person types into it.
2. **Given** a slide with three elements, **When** the person drags the third
   above the first, **Then** the source blocks are reordered and each block's
   own text is byte-identical to before.
3. **Given** an element, **When** the person deletes it, **Then** its block is
   removed from the source and Undo brings it back unchanged.

---

### User Story 5 — Edit together with an agent (Priority: P1)

While the person edits on the slide, an agent edits the same deck through the
CLI. Neither silently overwrites the other: changes from the other side
appear in the preview, and if both change the same element the person is told
and keeps their unsaved text.

**Why this priority**: Working on a deck with Claude in chat while editing
directly in the browser is the main way this editor is used; on-slide editing
must not regress the safety the form-based editor already has.

**Independent Test**: With the person mid-edit on a paragraph, the agent
changes a different slide and then the same paragraph; the first change
appears without disturbing the edit, the second is reported as a conflict and
the person's text is preserved.

**Acceptance Scenarios**:

1. **Given** the person is typing in an element, **When** the agent edits a
   different element or slide, **Then** the preview updates without losing the
   person's cursor position or unsaved text.
2. **Given** the person is typing in an element, **When** the agent changes
   that same element and the person then saves, **Then** the save is refused
   as a conflict, the person sees the agent's version and still has their own
   text to re-apply or discard.
3. **Given** any on-slide edit, **When** it is saved, **Then** it is recorded
   exactly like the equivalent `qd` command, so `qd undo` from the CLI undoes
   it and the agent sees it on its next read.

---

### User Story 6 — Right-click for the full set of actions (Priority: P2)

Right-clicking an element on the slide, a slide in the slide list, or an
empty area of the slide opens a context menu with every action available for
it, as in other desktop editors. The toolbar of US3 keeps only the most
common actions; the context menu has the full set.

**Why this priority**: Right-click is where people look first for "what can I
do with this"; without it, actions beyond the few toolbar buttons are hard to
discover.

**Independent Test**: Right-click a paragraph, a slide in the list and an
empty part of a slide; each menu lists the actions in FR-020 for that target,
each action works, and each has the same effect as its `qd` command.

**Acceptance Scenarios**:

1. **Given** a paragraph, **When** the person right-clicks it, **Then** a menu
   offers Edit, Edit source, Cut, Copy, Paste, Duplicate, Delete, Move up,
   Move down, Insert before/after (paragraph, list, image, text box, equation), and
   Appearance (fragment and other classes).
2. **Given** a slide in the slide list, **When** the person right-clicks it,
   **Then** a menu offers Add slide before/after, Duplicate, Hide/Show, Delete,
   Background…, Transition…, Edit slide source and Copy link to slide.
3. **Given** an open context menu, **When** the person uses the arrow keys and
   Enter, or presses Escape, **Then** the menu works and closes as a standard
   menu does; the keyboard's context-menu key or Shift+F10 opens it for the
   selected item.
4. **Given** an action that cannot apply (e.g. Move up on the first element),
   **When** the menu opens, **Then** that action is shown disabled, not hidden.
5. **Given** each menu action, **When** it has a keyboard shortcut, **Then**
   the menu shows the shortcut next to it.

---

### Edge Cases

- An element whose rendered text differs from its source (smart quotes,
  `--` rendered as an en dash, footnotes, citations `[@key]`, cross-references
  `@fig-x`, emoji shortcodes): typing directly must never write the rendered
  form back over the source form; such elements use source editing (US2), or
  only the edited run of plain text changes.
- Incremental lists and fragments: when editing, all fragments of the slide
  are shown so hidden content can be reached and edited.
- Columns (`:::: columns`), callouts and other fenced divs: content inside is
  editable; the fence lines themselves only through source editing.
- Elements generated by code cells (plots, tables printed by code): not
  editable as text; clicking opens the cell's source (US2).
- Speaker notes (`::: notes`): not visible on the slide; editable from the
  slide toolbar ("Notes") in a source editor, not lost on any other edit.
- Opaque slides (raw HTML or constructs qd does not model): editable only as
  whole-slide source, as today.
- The re-render after a save takes 1–2 s: the edited text stays visible as
  typed in the meantime (no flash back to the old text), and a render error
  keeps the last good slide visible with the error shown, as today.
- The title slide (front matter): title, subtitle, author and date editable in
  place on the title slide; other options remain in the source view.
- Empty slide: offers an insert control so the first element can be added.
- Long edits: an edit open for a long time is not saved partially and is not
  lost if the preview re-renders because of a change elsewhere.

## Requirements *(mandatory)*

### Functional Requirements

**Text editing (US1)**

- **FR-001**: Slide titles, paragraphs, list items, block quotes and text
  boxes MUST be editable by clicking them in the preview and typing.
- **FR-002**: Saving an edit MUST change only the source span of the edited
  element; the rest of the `.qmd` MUST stay byte-identical, including
  attributes, spacing, and unchanged inline markup inside the element.
- **FR-003**: An edit MUST be saved when the person leaves the element (click
  elsewhere, Tab, or ⌘/Ctrl+Enter) and abandoned with Escape.
- **FR-003a**: While editing text, a formatting toolbar MUST offer bold,
  italic, underline, strikethrough, inline code, link, bulleted and numbered
  list, indent/outdent, text colour, highlight, alignment and font size
  steps, each with its standard shortcut where one exists (⌘B, ⌘I, ⌘U, ⌘K…).
- **FR-003b**: Formatting MUST be written as the markdown construct when one
  exists, otherwise as a Pandoc span or div with a class or style around only
  the affected text; applying and then removing a format MUST leave the
  source byte-identical to before.
- **FR-003c**: Formatting already in the source that the toolbar cannot
  represent MUST be kept unchanged, and the element MUST offer source editing
  for it.
- **FR-004**: Enter at the end of a list item MUST create a new item; Enter at
  the end of a paragraph MUST create a new paragraph after it; Backspace in an
  empty item or paragraph MUST remove it.
- **FR-005**: Text typed on the slide MUST be written to the source in a form
  that renders back to what was typed (characters with markdown meaning are
  escaped as needed).

**Source editing in place (US2)**

- **FR-006**: Elements that cannot be edited faithfully by typing on the
  rendered slide (see Edge Cases) MUST open an in-place source editor over the
  element when clicked.
- **FR-007**: Every element MUST also offer "Edit source", and every slide
  "Edit slide source", using the same in-place editor.
- **FR-008**: A source edit that would change the slide structure (add or
  remove a slide heading or break a fence) MUST be refused with an
  explanation, keeping the person's text in the editor.

**Properties on the slide (US3)**

- **FR-009**: Selecting a slide MUST show controls for background colour,
  background image, transition and hidden/visible; selecting an element MUST
  show its classes (with common ones offered by name, e.g. "Appears on click"
  for `.fragment`) and delete.
- **FR-010**: Each property change MUST be written to the source as the
  matching attribute or class change and appear in the preview.

**Structure (US4)**

- **FR-011**: The person MUST be able to insert any element type from FR-019
  before or after any element, and on an empty slide.
- **FR-012**: The person MUST be able to delete any element and to move an
  element before or after another element on the same slide by dragging.
- **FR-013**: Moving an element MUST keep its source text byte-identical.

**Context menus (US6)**

- **FR-020**: Right-clicking an element, a slide in the slide list, or an
  empty part of a slide MUST open a context menu with the full set of actions
  for that target (listed in US6), replacing the browser's own menu there;
  actions that cannot apply MUST be shown disabled.
- **FR-021**: Context menus MUST follow standard menu behaviour: keyboard
  navigation, Escape to close, opening with the context-menu key or
  Shift+F10, shortcuts shown beside actions, and staying within the window.
- **FR-022**: Cut, Copy and Paste of elements MUST carry the element's
  markdown source, so pasting into the same or another slide (or into a text
  editor) reproduces it exactly.

**Shared rules**

- **FR-014**: Every on-slide edit MUST be carried out by the same operation a
  `qd` command performs (an equivalent command exists for each), so the CLI and
  the browser give identical results and share one undo history.
- **FR-015**: Every on-slide edit MUST be refused, not applied, when the file
  changed since the element was read, and the person MUST keep their unsaved
  text when that happens.
- **FR-016**: Changes made elsewhere (CLI, agent, text editor) MUST appear in
  the preview without discarding an edit in progress.
- **FR-017**: All on-slide controls MUST be usable from the keyboard (move
  between elements, start editing, save, cancel) and MUST have accessible
  names.
- **FR-018**: The side panel MUST be hidden by default and open with a
  "Source" toggle, showing the selected slide's or element's markdown and its
  advanced properties.
- **FR-019**: The insert control MUST offer paragraph, list, image, text box
  and equation; other constructs (code cell, columns, callout) are added
  through source editing in this feature.

### Key Entities

- **Editable element**: a block of a slide (title, paragraph, list, list item,
  quote, text box, image, other) with its source span, whether it can be typed
  into directly or needs source editing, and its classes and attributes.
- **Edit session**: one open edit of one element: the element, the file
  version it was read at, the unsaved text, and whether it ended saved,
  cancelled or in conflict.
- **Slide properties**: background colour and image, transition, hidden,
  classes — the attributes of the slide heading.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: A person changes a slide title, a paragraph and a list item
  without opening the side panel, each in under 10 seconds from first click.
- **SC-002**: Across a corpus of at least 20 real decks, every on-slide edit
  in the test suite leaves the `.qmd` byte-identical outside the edited
  element, and undo restores the file byte-for-byte (100%).
- **SC-003**: Every element in that corpus can be edited on the slide, either
  by typing or through the in-place source editor; none requires the side
  panel.
- **SC-004**: The typed text is visible on the slide at once and the
  re-rendered slide replaces it within 3 seconds of saving, with no flash of
  the old text.
- **SC-005**: In a simultaneous-edit test (person on the slide, agent through
  the CLI, 50 interleaved edits), no edit from either side is lost and every
  same-element collision is reported to the person.
- **SC-006**: Everyday edits (text, background, transition, fragment, add,
  delete, reorder) are possible with the side panel closed; a first-time user
  completes a 6-step editing task without help in under 3 minutes.

## Assumptions

- The editor, preview, operations, undo history and conflict detection from
  spec 001 are the foundation; this feature changes how edits are started in
  the browser, not how they are stored.
- Rendering stays with Quarto; the slide shown after a save is always the
  real rendered output, never an approximation kept by the editor.
- Positioned boxes keep their existing drag and resize (spec 001 US4); text
  boxes additionally become typeable (US1).
- A new operation is added for any on-slide action that has no `qd` command
  yet (e.g. moving or deleting a single element within a slide), so FR-014
  holds; it is also documented for agents in the skill.
- Touch devices are out of scope for this feature; mouse/trackpad and
  keyboard are required.
- One person edits in the browser at a time; the agent is the other editor.
