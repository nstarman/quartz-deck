# Feature Specification: Usability — the editor made easy to use

**Feature Branch**: `008-usability`

**Created**: 2026-11-06

**Status**: Draft

**Input**: User description: "Usability (roadmap 008): turn the design audit in specs/008-usability/audit/README.md into the editor's next round of work. The quartz-deck editor has the features; make them easy and pleasant to use, against Apple's Human Interface Guidelines as the reference (our own design). Stories from the audit's priorities: (P1) the slide list — add a slide anywhere (a + between slides on hover and from the keyboard, after the last slide; new slides start untitled with a placeholder, not "New slide" text in the file), drag with a visible handle, a translucent drag image and an insertion marker where the slide will land, thumbnails in the list instead of the hover pop-up, the title slide listed first; (P1) the canvas — bars never cover the slide's content, reveal.js's own menu button/controls/progress hidden while editing, no jump when the slide bar appears; (P1) contrast and size — every control and text meets 4.5:1 (3:1 large/bold) in light, dark and high contrast (themed form controls and source colours), controls at least 20 pt (28 pt default); (P1) a Present button (from the current slide, and from the beginning); (P1) our own small sheets instead of the browser's prompt/confirm/alert (image picker with files beside the deck plus drag-and-drop of an image file, link field with validation, colour menu, package field, About); (P2) header shows the deck's title and plain-language status, tab title is the deck; one top bar; plain names instead of jargon (Render, Open preview, Form, On click, + Package, Create environment, Source for two things); slide bar controls one height with a None colour choice; Insert offers code cell, callout, columns, speaker notes, table, quote; formatting bar with the common few up front and the rest in a pop-over; source pane run bar simplified with session state in words; narrow windows (list collapses to a slide picker below ~600 px, properties into ···); VS Code panel puts ☰ in the header row; (P3) multi-select in the list, Settings layout, high-contrast selection colour, 'Saved' feedback near the element. Every action stays in the 007 action list with its key, every edit stays a qd command, no new features beyond usability (layouts/free placement are 009, Run on the slide is 012)."

## Overview

Specs 001–007 gave the editor its features and an application shell. The 008
audit ([`audit/README.md`](audit/README.md), 71 screenshots, measured
contrast and sizes, Apple's Human Interface Guidelines as the reference)
found what makes them hard to use: slides can be added in one place only,
dragging gives no feedback, bars cover the slide, some text can't be read in
the dark theme, presenting is hidden, plain browser boxes ask for typed
paths, and the words are the program's, not the person's. This feature fixes
those, in the audit's order of priority. It adds no new editing power; each
finding it addresses is named by its audit id (L1, C1, …).

## User Scenarios & Testing *(mandatory)*

### User Story 1 — The slide list works like a slide app's (Priority: P1)

The person sees every slide as a small picture in the list, title slide
first. They add a slide exactly where they want it, from a + that appears
between slides or with a key, and the new slide is waiting for its title. They
drag a slide by a visible handle and see where it will land before they let
go.

**Why this priority**: the list is how a deck is built and reordered; today
adding goes through one button whose position misleads (L1), dragging gives
no feedback (L2), and slides are recognised only through a pop-up that covers
the canvas (L3, L4). The person named all three.

**Independent Test**: In the audit deck, add a slide between slides 2 and 3
using only the list, give it a title, drag slide 6 to second place watching
where it will land, and find a slide by its picture; each takes one or two
actions and nothing covers the slide being edited.

**Acceptance Scenarios**:

1. **Given** the list, **When** the person hovers between two slides,
   **Then** a + appears there; clicking it adds a slide at that place and
   shows it, ready for a title.
2. **Given** a slide focused in the list, **When** the person presses Enter
   (or uses Insert ▸ Slide after), **Then** a slide is added after it; a + after
   the last slide adds one at the end.
3. **Given** a new slide, **When** it appears, **Then** its title area shows a
   placeholder ("Click to add title") that is not written into the deck, and
   typing replaces it.
4. **Given** the list, **When** the person hovers a slide, **Then** a drag
   handle shows; **When** they drag, **Then** a translucent picture of the
   slide follows the pointer, the slides part to show a gap where it will
   land, Escape cancels, and dropping moves it as one undoable step.
5. **Given** a long list, **When** a drag nears its top or bottom edge,
   **Then** the list scrolls.
6. **Given** the list, **When** it shows, **Then** each slide is a thumbnail
   with its number and title, the title slide first, and the slide on the
   canvas is marked; the hover pop-up over the canvas no longer appears.
7. **Given** the keyboard only, **When** a slide is focused, **Then** ⌥↑/⌥↓
   move it and Delete removes it (undoable).

---

### User Story 2 — Nothing covers the slide (Priority: P1)

Bars that belong to a selection never sit on top of the slide's other
content; reveal.js's own menu button, arrows and progress bar don't show
while editing; and moving between slides doesn't make the canvas jump.

**Why this priority**: today the element bar and the formatting bar cover the
slide's title (C1), reveal's ☰ looks like ours (C2), and the canvas moves 34 px
when the slide bar appears (C3). These break direct manipulation on every
slide.

**Independent Test**: Select and edit the first element under a title on
each slide of the audit deck: no bar overlaps another element; no reveal
controls are visible; the canvas's position is the same on the title slide
and on content slides.

**Acceptance Scenarios**:

1. **Given** an element with another element directly above it, **When** it
   is selected or edited, **Then** its bar appears where it covers no other
   element (below it, or in a fixed strip), never on top of content.
2. **Given** the editor, **When** any slide is shown, **Then** reveal.js's
   menu button, navigation arrows and progress bar are hidden (they still show
   when presenting).
3. **Given** the title slide and a content slide, **When** the person moves
   between them, **Then** the canvas does not move.

---

### User Story 3 — Everything can be read and reached (Priority: P1)

Every piece of text and every control meets the contrast and size minimums in
light, dark and high contrast, including form controls and the source pane's
colours.

**Why this priority**: in the dark theme the Transition menu measures 1.18:1
and several source colours fail (T1, T2); some controls are 8–17 px (T3).
Unreadable or unhittable controls stop people outright.

**Independent Test**: An automated check walks every visible control and
text in each theme and measures contrast and size against the minimums.

**Acceptance Scenarios**:

1. **Given** any theme, **When** any text is shown, **Then** its contrast is
   at least 4.5:1 (3:1 for 18 pt and larger, or bold).
2. **Given** any theme, **When** a form control (select, checkbox, colour
   swatch, text field) is shown, **Then** it uses the theme's colours and its
   value is readable.
3. **Given** any interactive control, **When** measured, **Then** its target
   is at least 20×20 pt, and buttons in bars use one height (28 pt).
4. **Given** high contrast, **When** an element is selected or edited,
   **Then** its outline uses the theme's accent (T5).

---

### User Story 4 — Present is one click (Priority: P1)

A Present button in the top bar plays the deck from the current slide; a
choice beside it plays from the beginning.

**Why this priority**: presenting is what a deck is for, and today it is File ▸
Open preview ↗ (H2).

**Independent Test**: From slide 4, one click starts the presentation at
slide 4; the other choice starts at the title slide.

**Acceptance Scenarios**:

1. **Given** any slide, **When** the person clicks Present (or presses its
   key), **Then** the deck plays from that slide, full screen.
2. **Given** the Present menu, **When** the person picks "From the beginning",
   **Then** the deck plays from the title slide.

---

### User Story 5 — The editor's own dialogs (Priority: P1)

Choosing an image, a link, a colour, packages or confirming a reset happens in
the editor's own small sheets that look like the rest of it, check what is
entered, and offer choices instead of asking for typing.

**Why this priority**: nine plain browser boxes remain (D1), including typing
an image's path by hand.

**Independent Test**: Insert an image from a list of the deck's images and by
dropping an image file on the slide; add a link and see a bad address refused
with a reason; pick a background colour from a menu with None; no browser
`prompt`, `confirm` or `alert` box appears anywhere.

**Acceptance Scenarios**:

1. **Given** Insert ▸ Image…, **When** it opens, **Then** it lists the images
   beside the deck with small previews and accepts a path or address too.
2. **Given** an image file dragged from the desktop onto a slide, **When** it
   is dropped, **Then** it is copied beside the deck and inserted where it was
   dropped (one undoable step); a name already taken is not overwritten.
3. **Given** a link, background colour, background image, custom class or
   packages to add, **When** the person enters one, **Then** an inline sheet
   takes it, says what is wrong with an invalid value, and offers None where
   removing makes sense.
4. **Given** any action, **When** it runs, **Then** no browser `prompt`,
   `confirm` or `alert` box appears.

---

### User Story 6 — Plain words and one top bar (Priority: P2)

The top of the window is one bar: the menus (or ☰), the deck's title, its
status in words, Present and Source. The browser tab is named after the deck.
Labels say what happens.

**Why this priority**: the header names the app and shows a version hash
(H1); three bars take 104 px (H3); labels are the program's words (H4); VS
Code's panel spends a row on ☰ (N1).

**Independent Test**: With two decks open, the tabs and headers tell them
apart; every menu item, button and status is in plain words a presenter
understands without knowing Quarto.

**Acceptance Scenarios**:

1. **Given** a deck, **When** it is open, **Then** the top bar and the tab
   title show its title (and file name), and the status reads "Saved",
   "Rendering…", "Up to date" or a plain error, the version only in a tooltip.
2. **Given** the editor, **When** shown, **Then** there is one bar above the
   slide area (the slide's own properties excepted), including in VS Code's
   panel, where ☰ sits in that bar.
3. **Given** a label from the audit's jargon list (Render, Open preview,
   Export offline HTML, Export with PDF, + Package, Create environment, Form,
   On click, Edit source), **When** shown, **Then** it reads in the person's
   terms, and "Source" names one thing only.

---

### User Story 7 — Tidier bars and a fuller Insert (Priority: P2)

The slide's properties use controls of one height with clear labels; the
formatting bar shows the common few and keeps the rest one click away; the
source pane's bar shows Run all, Stop and the session state in words, with
the rest in a menu; Insert offers the deck's building blocks.

**Why this priority**: C4, C5, C6, E1, S1, S2 and B1 in the audit: crowded,
unlabeled and mixed-size controls slow every edit.

**Independent Test**: Count and measure the controls in each bar against the
audit's numbers; insert each new building block from Insert and see it on the
slide.

**Acceptance Scenarios**:

1. **Given** the slide bar, **When** shown, **Then** every control is one
   height, the colour control offers None, and every control has a label or a
   tooltip naming it.
2. **Given** text being edited, **When** the formatting bar shows, **Then** it
   holds bold, italic, underline, strikethrough, link, list and colour; the
   rest is in one pop-over; colours are named.
3. **Given** the source pane, **When** shown, **Then** its bar shows Run all,
   Stop and the session state in words; Restart, packages and environment are
   in one menu.
4. **Given** Insert, **When** opened, **Then** it offers code cell, callout,
   columns, speaker notes, table and quote, besides today's items.
5. **Given** a positioned box, **When** selected, **Then** its resize handles
   are easy to grab (at least 20 pt target).

---

### User Story 8 — A narrow window still works (Priority: P2)

Below about 600 px wide, the slide list becomes a slide picker in the top
bar, the slide's properties move into a ··· menu, and the slide keeps its
shape without an empty band.

**Why this priority**: N2: at 420 px the list is a strip cut off at the edge
and the slide bar wraps.

**Independent Test**: At 420 px, reach every slide, change a property, and
edit text, with nothing cut off or wrapped.

**Acceptance Scenarios**:

1. **Given** a window under 600 px, **When** shown, **Then** the list is a
   picker in the top bar, the properties are in ···, nothing is cut off, and
   every action stays available.

---

### User Story 9 — Finishing touches (Priority: P3)

The list takes several slides at once; Settings lines up; a short "Saved"
shows by the element after an edit; undoing a duplicate returns to a
neighbouring slide.

**Why this priority**: L5, D2, E2 and the 007 quickstart note; each is small.

**Independent Test**: Select three slides with ⇧/⌘-click and move or delete
them together; the Settings controls share one width with Theme first; edit
text and see "Saved" by it; undo a duplicate and stay near it.

**Acceptance Scenarios**:

1. **Given** the list, **When** the person ⇧-clicks or ⌘-clicks, **Then**
   several slides are selected and move, copy or delete together as one undo
   step.
2. **Given** Settings, **When** opened, **Then** Theme is first and the
   controls share one width.
3. **Given** an edit on the slide, **When** it saves, **Then** a short
   "Saved" shows near the element and fades.
4. **Given** an on-screen duplicate, **When** it is undone, **Then** the
   canvas shows the slide it was made from.

---

### Edge Cases

- A deck with no title in its front matter: the list's first item is the
  first slide; the top bar shows the file name.
- A deck with 100+ slides: thumbnails appear without slowing the list
  (rendered lazily), and dragging scrolls.
- Dropping an image file whose name exists beside the deck: the new file gets
  a free name; nothing is overwritten.
- Dropping something that isn't an image (a PDF, a folder): refused with a
  message, nothing changes.
- An element at the very top of a slide with no room above or below for its
  bar: the bar goes to the fixed strip, still never over content.
- Present while an edit is open: the edit saves first.
- The person's own theme overrides (VS Code passes its theme): the contrast
  rules still hold for the editor's own surfaces; the deck's own slide
  colours are the deck's.
- A slide with no title: the list shows its first words, then "Untitled".

## Requirements *(mandatory)*

### Functional Requirements

**Slide list**

- **FR-001**: The list MUST show each slide as a thumbnail with its number
  and title, the title slide first, the current slide marked; the hover
  pop-up over the canvas MUST be removed.
- **FR-002**: The person MUST be able to add a slide at any position from the
  list: a + between slides on hover, a + after the last slide, and Enter on a
  focused slide; each adds one slide as one undoable edit.
- **FR-003**: A new slide MUST start untitled, showing a placeholder that is
  not written into the deck.
- **FR-004**: Dragging a slide MUST offer a visible handle, a translucent
  picture of the slide, a gap at the landing place, scrolling near the list's
  edges and Escape to cancel; the move MUST be one undoable edit.
- **FR-005**: Moving and deleting MUST work from the keyboard (⌥↑/⌥↓, Delete)
  and from the menus, as alternatives to dragging.

**Canvas**

- **FR-006**: No bar or handle belonging to a selection MAY cover another
  element of the slide.
- **FR-007**: reveal.js's menu button, navigation arrows and progress bar MUST
  be hidden in the editor and shown when presenting.
- **FR-008**: The canvas's position MUST NOT change when moving between
  slides.

**Contrast and size**

- **FR-009**: All text MUST meet 4.5:1 contrast (3:1 for 18 pt and larger, or
  bold) against its background, in light, dark and high contrast.
- **FR-010**: Form controls and the source pane's colours MUST follow the
  theme.
- **FR-011**: Interactive targets MUST be at least 20×20 pt; buttons in a bar
  MUST share one height (28 pt by default).
- **FR-012**: Selection and editing outlines MUST use the theme's accent.

**Present**

- **FR-013**: A Present control in the top bar MUST play the deck from the
  current slide; a second choice MUST play it from the beginning; both MUST be
  in the action list with keys.

**Dialogs**

- **FR-014**: No action MAY use the browser's `prompt`, `confirm` or `alert`.
- **FR-015**: Choosing an image MUST list the images beside the deck and
  accept a typed path or address; dropping an image file on a slide MUST copy
  it beside the deck under a free name and insert it where dropped.
- **FR-016**: Entering a link, colour, background image, class or packages
  MUST happen in the editor's own sheet, which validates the value and says
  what is wrong.

**Words and top bar**

- **FR-017**: The top bar and the tab title MUST show the deck's title (else
  file name); the status MUST be in words, the version only on request.
- **FR-018**: There MUST be one bar above the slide area, holding the menus
  (or ☰), the deck's title, status, Present and Source, including in VS Code's
  panel.
- **FR-019**: Labels MUST name what happens in the person's terms; one word
  MUST NOT name two different things.

**Bars and Insert**

- **FR-020**: The slide's property controls MUST share one height, each MUST
  have a label or tooltip, and colour choices MUST include None.
- **FR-021**: The formatting bar MUST show the most common formats directly
  and the rest in one pop-over; colours MUST be named.
- **FR-022**: The source pane's bar MUST show Run all, Stop and the session
  state in words, with the other run and environment actions in one menu.
- **FR-023**: Insert MUST offer code cell, callout, columns, speaker notes,
  table and quote, each producing ordinary Quarto markdown.

**Narrow windows and polish**

- **FR-024**: Below about 600 px wide, the list MUST become a slide picker in
  the top bar and the slide's properties MUST move into a ··· menu, with no
  action lost.
- **FR-025**: The list MUST support selecting several slides (⇧/⌘-click) for
  move, copy and delete as one undo step.
- **FR-026**: After an edit on the slide saves, a short "Saved" MUST show near
  the element.
- **FR-027**: Undoing an on-screen duplicate MUST show the slide it was made
  from.

**Unchanged rules**

- **FR-028**: Every new or renamed action MUST be in the action list (007)
  with its menu, key and place; every deck edit MUST remain a `qd` command;
  nothing in this feature MAY write editor state into the deck.

### Key Entities

- **Audit finding**: id (L1, C1, …), surface, what is wrong, principle,
  evidence, proposed change, priority. Each requirement above traces to one
  or more findings.
- **Thumbnail**: a small picture of one slide, made from the same render the
  canvas shows, refreshed when that slide changes.
- **Sheet**: the editor's own small dialog for a value (image, link, colour,
  class, packages, confirmation), with validation and a message.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Adding a slide at a chosen position takes at most two actions
  from the list (today: select, then a button at the end of the list).
- **SC-002**: In a usability check, the person predicts where a dragged slide
  will land before dropping, every time, in 10 drags.
- **SC-003**: In the audit deck, no bar covers another element when each
  element is selected or edited (0 overlaps, from 2 today).
- **SC-004**: Every text and control in light, dark and high contrast passes
  the contrast minimums (0 failures, from 6 measured today).
- **SC-005**: Every interactive target is at least 20×20 pt (0 below, from 4
  kinds today: ▶ Run, box handles, the Hidden checkbox, the Transition menu).
- **SC-006**: Presenting from the current slide takes one action (from two,
  in a menu).
- **SC-007**: No browser `prompt`, `confirm` or `alert` box remains (from 9).
- **SC-008**: Adding an image takes no typing when the file is beside the
  deck or on the desktop.
- **SC-009**: The space above the slide area is one bar (from three, 104 px at
  800 px tall).
- **SC-010**: Rerunning the audit's screenshot script and measurements after
  this feature shows every P1 and P2 finding resolved, with the evidence
  recorded beside the audit.

## Assumptions

- Apple's Human Interface Guidelines are the reference for principles and the
  numbers used (sizes, contrast); the design is our own (constitution I).
- Thumbnails come from the renders the editor already makes; no new render
  path.
- Images dropped from the desktop are copied into the deck's folder (an
  `images/` folder beside the deck, made if missing); this is the one place
  this feature writes outside the `.qmd`, and only on the person's drop.
- Present opens the existing presentation view; it gains a "from the
  beginning" choice and full screen.
- A new slide's placeholder title is shown by the editor only; the deck gets
  an untitled slide (a heading the person fills, or a slide break), as `qd add`
  without a title makes today.
- The source pane's highlighting palette and every form control get light,
  dark and high-contrast colours; the deck's own slide colours are not
  changed.
- Layout templates and free placement (009) and Run on the slide (012) are out
  of scope; where this feature touches the same surfaces (Insert, the source
  pop-up), it leaves room for them.
- VS Code's panel keeps 006/007 behaviour; only its top bar changes (☰ in the
  one bar).
