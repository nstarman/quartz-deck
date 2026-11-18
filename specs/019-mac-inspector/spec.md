# Feature Specification: An inspector on the right when the window is wide

**Feature Branch**: `019-mac-inspector`

**Created**: 2026-11-17

**Status**: Draft

**Input**: User description: "Plan a Mac-style right-side toolbar when the horizontal layout is wide enough", with a Keynote window as the model: slide list on the left, the slide in the middle, and on the right a Format inspector with Style / Text / Arrange tabs for the selection.

## Context

Today the editor spreads its controls over the slide: a slide bar above it
(background, transition, hidden), an element bar that floats over the
selected element (run, edit, markdown, appears on click, delete, ⋯), a
formatting bar while typing, and a Properties panel on the right that
opens on request. On a wide screen there is room to keep all of this in
one place beside the slide, as Keynote does, and leave the slide itself
clear.

Decisions made with the person:

- When the window is wide and the inspector shows, it replaces the floating
  bars (slide bar, element bar, formatting bar). In a narrower window the
  floating bars work as they do today.
- It is organised in tabs that follow the selection, like Keynote: Slide
  when nothing is selected; Style, Text and Arrange for an element. It
  takes over today's Properties panel.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Change the selected element from the inspector (Priority: P1)

On a wide window the presenter selects a box on the slide. Nothing floats
over the slide. On the right, the inspector shows the box's controls in
three tabs: Style (look, appears on click, classes), Text (the formatting
controls) and Arrange (position, size, order, delete). They make a change
there and the slide updates, as it would from the element bar.

**Why this priority**: it is the point of the feature: the controls in one
fixed place beside the slide, not on top of it.

**Independent Test**: open a deck in a window 1440 px wide, select a text
box, turn on "Appears on click" in the Style tab and set its left edge in
the Arrange tab; the file changes as with the element bar, and no floating
bar appeared.

**Acceptance Scenarios**:

1. **Given** a wide window and an element selected, **When** the presenter
   looks at the slide, **Then** no element bar or formatting bar floats
   over it, and the inspector shows that element's tabs.
2. **Given** the Style tab, **When** they toggle "Appears on click" or set
   a class, **Then** the deck changes exactly as the element bar's or
   menu's action would, with one undo step.
3. **Given** the Arrange tab, **When** they type a position or size, or
   move the element up or down, **Then** it moves as from today's
   Properties panel and menu.
4. **Given** they are typing in a text element, **When** they use the Text
   tab (bold, list, colour, alignment…), **Then** it formats the text as
   the formatting bar would, and the typing keeps its place.
5. **Given** an element kind with its own controls (a code cell, a
   diagram box, an arrow, a table), **When** selected, **Then** the tabs
   show those controls (Run for a cell; the diagram's and arrow's
   properties), and tabs that don't apply are left out.

---

### User Story 2 — The slide's own settings when nothing is selected (Priority: P2)

With nothing selected, the inspector shows the slide: background colour
and image, transition, hidden, layout, and the slide's other actions.
The slide bar above the slide is gone on a wide window.

**Why this priority**: it completes the move of the controls off the slide.

**Independent Test**: on a wide window, click an empty part of the slide;
the inspector shows the Slide tab; set the transition to fade; the file
changes as from the slide bar.

**Acceptance Scenarios**:

1. **Given** a wide window and nothing selected, **When** the slide shows,
   **Then** the inspector shows the Slide tab and no slide bar is above the
   slide.
2. **Given** the title slide is shown, **When** nothing is selected,
   **Then** the inspector shows the deck's title fields (title, subtitle,
   author, date) as today's title panel does.

---

### User Story 3 — It adapts to the window (Priority: P3)

The presenter narrows the window, or shows the source pane beside the
slide. When there is no longer room, the inspector closes and the
floating bars come back; when there is room again, it comes back. They
can also hide it with a toolbar button or a shortcut, as Keynote's Format
button does, and that choice is remembered.

**Why this priority**: the feature must not cost the slide its room on a
small screen.

**Independent Test**: at 1440 px the inspector shows; resize to 1000 px:
it hides and selecting an element shows the element bar; resize back: it
shows again with the same selection.

**Acceptance Scenarios**:

1. **Given** the window becomes too narrow, **When** it is resized,
   **Then** the inspector hides and the floating bars work as today, with
   the selection kept.
2. **Given** the inspector is hidden by the button, **When** the editor is
   reopened, **Then** it stays hidden until shown again, and the floating
   bars are used meanwhile.
3. **Given** the editor runs inside VS Code's panel, **When** the panel is
   wide, **Then** the inspector behaves the same.

---

### Edge Cases

- **A popup from a control** (colour picker, a class list, ⋯ menu) opens
  next to the inspector, not over the slide's centre.
- **Several elements selected**: the inspector shows only what applies to
  all of them (for example "Appears on click", Delete), or says nothing
  common if none applies.
- **A conflict or a render error**: messages show as today; the inspector
  keeps its place.
- **The History pane or the source pane on the right**: the inspector
  goes beside them only if the slide still keeps its minimum width;
  otherwise it hides as when the window is narrow.
- **Keyboard**: every control is reachable by Tab, and a shortcut moves
  focus to the inspector and back to the slide.
- **Dark mode and the VS Code theme**: the inspector follows the editor's
  colours, as the panels do today.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: When the editor has room beside the slide, it MUST show an
  inspector on the right; when it doesn't, the inspector MUST hide.
- **FR-002**: While the inspector shows, the slide bar, element bar and
  formatting bar MUST NOT float over or above the slide; their controls
  MUST be in the inspector.
- **FR-003**: The inspector's tabs MUST follow the selection: Slide (or
  the title fields on the title slide) with nothing selected; Style, Text
  and Arrange for an element, leaving out tabs that don't apply.
- **FR-004**: Every control in the inspector MUST do exactly what the same
  action does from the bars, menus or keys today: the same edit, the same
  undo step, the same `qd` command behind it.
- **FR-005**: The inspector MUST replace today's Properties panel; the
  Properties action MUST show the inspector (or its tab) on a wide window.
- **FR-006**: A toolbar button and a shortcut MUST show and hide the
  inspector; the choice MUST be remembered across sessions.
- **FR-007**: Below the width where it fits, the editor MUST behave as
  today (floating bars, Properties panel on request).
- **FR-008**: The inspector MUST be usable by keyboard and screen reader:
  labelled tabs, every control reachable, focus moves to it and back.

### Key Entities

- **Inspector**: the right-hand column; shown or hidden by width and by
  the person's choice; its tabs depend on the selection.
- **Tab**: Slide, Title, Style, Text or Arrange; each lists controls that
  map to existing actions.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: On a window 1440 px wide, selecting any element or typing
  shows no bar over the slide.
- **SC-002**: Every action of today's slide bar, element bar and formatting
  bar can be done from the inspector in at most two clicks.
- **SC-003**: Resizing across the width where the inspector appears never
  loses the selection, the typing place, or an unsaved edit.
- **SC-004**: The slide on a 1440 px window keeps at least 60% of the
  window's width with the slide list and inspector showing.

## Assumptions

- **Width**: the inspector shows when the window is at least 1200 px wide
  and the slide would keep at least 640 px; both numbers are defaults to
  tune by trying them on real screens.
- **Size**: the inspector is about as wide as today's Properties panel
  (320 px), not resizable at first.
- **What moves**: controls move, not change. No new edit is added by this
  feature; anything that can't be done today stays out.
- **Look**: a Mac-like column (tabs as a segmented control, grouped
  sections), drawn in the editor's own style and colours, with no Apple
  assets.
- **Out of scope**: a top toolbar of insert buttons like Keynote's
  (Insert stays in the menu), a resizable or floating inspector, and
  presenter-notes layout changes.
