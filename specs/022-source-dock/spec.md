# Feature Specification: Dock the source pane by dragging it

**Feature Branch**: `022-source-dock`

**Created**: 2026-11-20

**Status**: Draft

**Input**: User description: "Let me drag the source pane to dock it in one of three places: left of the slide list column, right of the slide stage (where it is today, or the inspector side), or beneath the stage. Dragging its header shows the drop zones; the chosen place is remembered as a setting and survives reloads. Full width mode and the inspector (spec 019) must keep working in each place."

## Context

Today the source pane sits on one side of the slide, chosen in Settings
(`source.side`: left or right of the slide, spec 007). Left puts it between
the slide list and the slide; right puts it after the slide, before the
inspector (spec 019) when that shows. A splitter beside it sets its width,
and Full width hides the slide so the pane takes the room.

The presenter wants to move the pane by dragging it, as in an IDE: grab its
title bar, see where it can go, drop it there. Below the slide is new: a
wide, short pane suits a wide screen where the slide should keep its full
width.

Clean room (constitution, Principle I): docking a panel by dragging its
title bar onto highlighted drop zones is common to IDEs and many editors.
No other product's look or wording is copied.

Decisions made with the person (2026-11-20):

- Four places, not three: today's default (between the slide list and the
  slide) stays, and stays the default.
- With the pane on the right and the inspector showing: slide, inspector,
  source pane. The inspector stays next to the slide it edits.
- On a narrow window the pane goes below the slide.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Drag the source pane to another place (Priority: P1)

The presenter has the source pane open. They press on the pane's title bar
(not on one of its buttons) and drag. Drop zones appear over the window,
one for each place the pane can go, and the one under the pointer is
highlighted. They release over "below the slide": the pane moves under the
slide, as wide as the slide, and the slide gets the full height above it.

**Why this priority**: it is the feature.

**Independent Test**: open a deck with the source pane open, drag its title
bar to each drop zone in turn; after each drop the pane is in that place,
its text, caret and scroll are unchanged, and the slide still shows.

**Acceptance Scenarios**:

1. **Given** the source pane is open, **When** the presenter presses on an
   empty part of its title bar and moves the pointer a few pixels, **Then**
   the drop zones appear, each labelled with its place, and the one under
   the pointer is highlighted.
2. **Given** a drag in progress, **When** they release over a drop zone,
   **Then** the pane docks there and the zones disappear.
3. **Given** a drag in progress, **When** they release outside every zone or
   press Esc, **Then** the pane stays where it was.
4. **Given** a press on a button in the title bar (Run all, Full width…),
   **When** they click it, **Then** the button works as before and no drag
   starts.
5. **Given** the pane docked anywhere, **When** they type, run a cell or
   follow the slide's selection into the source, **Then** it works as it
   does today.

### User Story 2 — The place is remembered (Priority: P1)

The presenter docks the pane below the slide and reloads, or opens another
deck, or opens the editor in VS Code. The pane is below the slide again.

**Why this priority**: without it the drag would have to be redone every
time.

**Independent Test**: dock below, reload; the pane is below. The Settings
window shows the same place, and choosing another place there moves the
pane at once.

**Acceptance Scenarios**:

1. **Given** the pane docked in a place, **When** the editor reloads,
   **Then** the pane opens in that place, with the size it had there.
2. **Given** the Settings window, **When** the presenter picks a place in
   the source pane setting, **Then** the pane moves there, as a drop would
   (the keyboard way to move it).
3. **Given** two editors open on the same machine, **When** one moves the
   pane, **Then** the other follows, as other settings do today.
4. **Given** a settings file from before this feature (`left` or `right`),
   **When** the editor opens, **Then** the pane is where it was.

### User Story 3 — Resize, Full width and the inspector in each place (Priority: P2)

In every place the splitter sits on the pane's edge facing the slide and
resizes it: sideways for a side place, up and down below the slide. Full
width works in every place. The inspector keeps working when the pane is on
the right.

**Why this priority**: the existing controls must not break; it is not new
value on its own.

**Independent Test**: in each place, drag the splitter and use the arrow
keys on it; toggle Full width; select an element with the inspector on.

**Acceptance Scenarios**:

1. **Given** the pane on a side, **When** the presenter drags the splitter,
   **Then** the pane's width changes; **given** it below, the pane's height
   changes. Each is remembered separately.
2. **Given** the splitter focused, **When** they press the arrow keys along
   its axis, **Then** the pane grows or shrinks.
3. **Given** Full width on, **When** the pane is in any place, **Then** the
   slide stage is hidden and the pane takes its room, as today; turning it
   off restores the place and size.
4. **Given** a window wide enough for the inspector and the pane on the
   right, **When** an element is selected, **Then** the inspector shows its
   tabs between the slide and the pane: slide, inspector, source pane,
   the pane at the window's right edge.

### Edge Cases

- A drag while History is open: History keeps its own column; the drop
  zones are the same.
- A drag starts and the window loses the pointer (another app, a dialog):
  the drag ends as a cancel.
- The source pane closed: nothing to drag; the remembered place applies when
  it opens.
- Full width on during a drag: the zones still show and the drop changes
  the remembered place; the pane stays full width.
- A narrow window (the width at which the editor already stacks its
  columns, 700 px): the pane shows below the slide whatever its place, and
  only the "below" zone shows during a drag. The remembered place is not
  changed, and comes back when the window is wide again.
- The inspector doesn't fit (narrow window, or a wide pane): it hides as
  today (spec 019) and the Properties panel works as before.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Pressing on an empty part of the source pane's title bar and
  moving the pointer starts a drag that shows one labelled drop zone per
  place; a press on a control in the bar does not.
- **FR-002**: There are four places: before the slide list (the window's
  left edge), between the slide list and the slide (today's left, still the
  default), right of the slide, and below the slide.
- **FR-003**: Releasing over a zone docks the pane there; releasing
  elsewhere or Esc leaves it where it was.
- **FR-004**: Moving the pane keeps its text, unsaved typing, caret,
  selection, scroll position and cell outputs.
- **FR-005**: The place is stored in the person's settings (the existing
  source pane setting gains the new places), applies on every reload and in
  every editor, and can be chosen in the Settings window.
- **FR-006**: Older settings files keep their meaning: `left` and `right`
  still put the pane where they did.
- **FR-007**: The splitter sits between the pane and the slide in every
  place, resizes along the right axis by pointer and arrow keys, and the
  pane's width (side places) and height (below) are remembered separately.
- **FR-008**: Full width works in every place.
- **FR-009**: With the pane on the right, the order is slide, inspector,
  source pane (the pane at the window's edge); this replaces today's slide,
  source, inspector order. The inspector keeps working in every place.
- **FR-010**: On a narrow window (700 px or less) the pane shows below the
  slide whatever its place, without changing the remembered place.
- **FR-011**: The drop zones and the dragged pane are visible in light,
  dark and high-contrast themes, and the drag does not select text on the
  page.

### Key Entities

- **Source pane place**: one of the dock places; stored as the value of the
  source pane setting. Its size in side places and its height below are
  separate remembered values.

## Success Criteria *(mandatory)*

- **SC-001**: Moving the pane to any place takes one drag, under 2 seconds.
- **SC-002**: After a reload the pane is in the same place and size every
  time.
- **SC-003**: Every existing source pane, splitter, Full width and inspector
  browser test passes with the pane in its default place; new tests cover
  each place.
- **SC-004**: No unsaved typing is lost by a move.

## Assumptions

- The default place for a new person stays as today.
- The keyboard way to move the pane is the Settings window choice; the drag
  itself is pointer only.
- History (spec 012) and the old Properties panel keep their columns; only
  the source pane moves.
- The pane can't float free of the window; it is always docked.
