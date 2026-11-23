# Feature Specification: One properties column

**Feature Branch**: `023-properties-column`

**Created**: 2026-11-23

**Status**: Draft

**Input**: User description: "Unify the slide, element and deck settings, now spread over bars above and beside the slide and panels below it, into one properties column beside the slide, which can be moved to the left or below like the source pane, and which is also where slide transitions and element animations are set."

## Context

Today a slide's settings and an element's settings are reached in several
places:

- **The slide bar**, over the slide: background colour, background image,
  transition, hidden, and a menu.
- **The element bar**, floating beside a selection: run, edit, markdown,
  appears on click, delete, and a menu.
- **The text toolbar**, floating beside text being typed.
- **The Properties panel**: the slide's title, background, transition,
  classes and body source; an element's source and size; a diagram's boxes
  and arrows; an arrow's options.
- **The inspector** (spec 019), which gathers the bars and the panel into
  tabs on the right, but only when the window is wide.

A setting is found in a different place depending on the window's width
and on what is selected, and some settings (how a transition is timed, the
order things appear in, the deck's slide size) have no place at all. This
feature puts every setting in one column, always in the same place, with
three views:

- **Format**: the slide, or the selected element.
- **Animate**: how the slide changes to the next, and how elements appear
  and leave.
- **Deck**: the presentation as a whole.

Decisions made with the person (2026-11-23):

- The slide bar and the element bar go. Their settings and actions are in
  the column, and the element's actions stay in its right-click menu.
- The text toolbar stays floating beside text being typed, since that's
  where the hands are. Its controls are also in the column's Text tab.
- Animate covers transitions and builds that Quarto and reveal.js already
  support: a slide's transition, its speed, and morphing into the next
  slide; an element's appearing and leaving on click, with an effect; and
  the order of all of a slide's builds.
- The column is always shown, and the View menu hides it. It sits on the
  right by default. Its header can be dragged to the left of the slide
  list, beside the list or below the slide, as the source pane can (spec
  022). In a narrow window it goes below the slide. It replaces spec 019's
  wide-window-only inspector.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Every slide and element setting in one place (Priority: P1)

The presenter clicks a blank part of a slide. The column's Format view
shows the slide: background colour and picture, hidden, title, which
classes it has, and its source. They pick a background colour from the
column and the slide changes. They click a picture on the slide: the
column now shows the picture's settings in three tabs.

- **Look:** its style.
- **Text:** for text, the font, colour, alignment, spacing and lists.
- **Arrange:** size, position, layer order and alignment with others.

No bar floats over the slide or beside the selection.

**Why this priority**: it is the change the person asked for. It removes
the most-used floating bars, and it gives every existing setting a single
home that works at any window width.

**Independent Test**: on a made-up deck, every setting the slide bar,
element bar and Properties panel offer today is reachable from the column,
with nothing selected and with each kind of element selected. Each change
writes the same text to the deck as before. No slide bar or element bar
appears.

**Acceptance Scenarios**:

1. **Given** nothing is selected, **When** the presenter looks at the column, **Then** it shows the slide's settings: background colour, background image, hidden, title, classes, and the slide's source.
2. **Given** an element is selected, **When** they look at the column, **Then** it shows the element's tabs. Text appears only for elements that hold text. The tab last chosen for that kind of element is the one shown.
3. **Given** they change a setting in the column, **When** it applies, **Then** the deck's text changes exactly as the same setting changes it today, as one undo step.
4. **Given** any selection, **When** the slide is shown, **Then** neither the slide bar nor the element bar appears. Run, edit, markdown, delete and appears-on-click are in the element's column and its right-click menu.
5. **Given** they are typing on the slide, **When** text is selected, **Then** the floating text toolbar shows beside it as today, and the column's Text tab shows the same controls.
6. **Given** a diagram's box or an arrow is selected, **When** they look at the column, **Then** its settings (label, look, link, picture, size; an arrow's ends, style and label) are in the same tabs.

---

### User Story 2 — Animate: transitions and builds (Priority: P2)

With nothing selected, the presenter opens Animate. They set the slide's
transition, its speed, and whether it morphs into the next slide. They
select a bullet list and choose that it appears on click with a fade, and
a picture that it grows in and leaves with a fade. A list under the
effects shows the slide's builds in the order they happen. They drag the
picture above the list, and in the presentation the picture now comes
first.

**Why this priority**: transitions and "appears on click" exist today
only as a plain choice and a checkbox, with no effect, no speed and no
order. The column is the natural place for them.

**Independent Test**: on a made-up deck, set each of these from Animate
and check the deck's text and the presented slide:
- a transition and its speed;
- morphing into the next slide;
- an appear effect;
- a leave effect;
- a new build order.

**Acceptance Scenarios**:

1. **Given** nothing is selected, **When** they choose a transition and a speed, **Then** the slide's attributes say so, and presenting shows that transition at that speed.
2. **Given** nothing is selected, **When** they turn on morphing into the next slide, **Then** the slide and the next morph matching elements when presenting.
3. **Given** an element is selected, **When** they choose how it appears (none, or an effect such as fade, fade up, grow, shrink or highlight), **Then** it appears on click with that effect. Choosing none makes it always shown.
4. **Given** an element appears on click, **When** they also choose how it leaves, **Then** it leaves on a later click with that effect.
5. **Given** a slide with several builds, **When** they look at the build list, **Then** it lists every build in the order it happens, named by its element. Dragging one to a new place changes the order the deck presents them in.
6. **Given** a build is in a list item or a column, **When** they look at the build list, **Then** it is named so the presenter can tell which it is (its first words or its kind).

---

### User Story 3 — Deck settings (Priority: P3)

With the Deck view open, the presenter sees the presentation's theme, its
slide size, whether slide numbers show, the default transition for every
slide, and how slides follow each other (spec 018's navigation). They
change the slide size, and every slide's preview changes shape.

**Why this priority**: these are set rarely, and today only in the
deck's front matter or through other menus. Bringing them into the column
completes the one-place picture.

**Independent Test**: each deck setting, changed from the Deck view,
changes the deck's front matter as the existing `qd meta set` does, and
the preview shows it.

**Acceptance Scenarios**:

1. **Given** the Deck view, **When** they pick a theme from Quarto's built-in themes, **Then** the deck's theme changes and the preview shows it.
2. **Given** the Deck view, **When** they set the slide size (a preset ratio or a width and height), **Then** the deck's width and height change, and positioned boxes keep their place relative to the slide.
3. **Given** the Deck view, **When** they turn slide numbers on, choose a default transition, or change navigation, **Then** the front matter says so and the preview follows.

---

### User Story 4 — The column goes where the presenter wants it (Priority: P4)

The presenter drags the column's header. Drop zones show, as when moving
the source pane, and they drop it below the slide. On a narrow window it
goes below the slide by itself. The View menu hides and shows it, and the
place and visibility are remembered.

**Why this priority**: the person asked for it to move like the source
pane. It reuses spec 022's way of moving.

**Independent Test**: drag the column to each place, reload, and check it
stays. Narrow the window and check it goes below the slide. Hide it from
the View menu and check the slide takes the room.

**Acceptance Scenarios**:

1. **Given** the column on the right, **When** they drag its header and drop it on a zone, **Then** it moves there and stays there after a reload.
2. **Given** the column and the source pane both want the same side, **When** both are shown, **Then** both fit in a set order, as spec 022 orders the source pane and the inspector today.
3. **Given** a window 700 px wide or less, **When** the editor lays out, **Then** the column is below the slide, whatever its chosen place.
4. **Given** View ▸ Properties is turned off, **When** the editor lays out, **Then** the column is hidden and the slide takes its room. Turning it on brings it back in its place.

### Edge Cases

- Several elements selected: the column shows the settings they share.
  Changing one changes all of them, as one undo step.
- An element qd can't safely edit (opaque markdown): the column says so,
  and offers its source as today's panel does.
- The title slide: the Format view shows the deck's title, subtitle,
  author and date, as today's title panel does.
- A build on an element inside a list: the list's items appear one by one
  when the list builds, and the build list shows the list as one build
  with its items under it.
- A transition or effect the deck uses that the column's choices don't
  list (written by hand): it shows as written and is kept until changed.
- Speaker notes: stay where they are today, not in the column.
- Presenting, speaker view and exports: the column never shows.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The editor MUST show one properties column with three views, Format, Animate and Deck, chosen at its top.
- **FR-002**: Format with nothing selected MUST offer every slide setting that the slide bar and slide panel offer today (background colour, background image, hidden, title, classes, source), and the title slide's fields on the title slide.
- **FR-003**: Format with an element selected MUST offer its settings in Look, Text (for text elements) and Arrange tabs. Together they MUST cover everything the element bar, the element panel, the diagram and arrow panels, and the text toolbar offer today.
- **FR-004**: The slide bar and the element bar MUST no longer appear. The text toolbar MUST still float beside text being typed.
- **FR-005**: Animate with nothing selected MUST set the slide's transition, its speed, and morphing into the next slide.
- **FR-006**: Animate with an element selected MUST set whether and how it appears on click, and how it leaves. Effects MUST be ones Quarto's presentations support.
- **FR-007**: Animate MUST list the slide's builds in presentation order and let the presenter reorder them, the new order written to the deck.
- **FR-008**: Deck MUST set the theme (Quarto's built-in themes), slide size, slide numbers, default transition and navigation.
- **FR-009**: Every change MUST be written to the deck by a CLI op, as one undo step, with the same text the CLI writes. Where no op exists (an effect, a build order), one MUST be added, so that agents can do the same.
- **FR-010**: The column MUST be movable by dragging its header to the right of the slide, the left of the slide list, beside the list, or below the slide. Its place and visibility MUST be remembered.
- **FR-011**: In a window 700 px wide or less, the column MUST be below the slide.
- **FR-012**: The column MUST be reachable by keyboard: a key moves focus into it and back to the slide, its views and tabs follow the arrow keys, and every control is labelled.
- **FR-013**: Settings written by hand that the column's choices don't list MUST be shown as written and kept.

### Key Entities

- **Properties column**: the one place for settings; its view (Format,
  Animate, Deck), its tab within Format, its place and whether it shows.
- **Build**: an element's appearing or leaving on click, with an effect,
  and its place in the slide's order.
- **Slide transition**: how a slide changes to the next, its speed, and
  whether it morphs.
- **Deck settings**: theme, slide size, slide numbers, default transition,
  navigation.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: Every setting reachable today from the slide bar, element bar,
  Properties panel and inspector is reachable from the column, checked
  item by item, and none is reachable only from a floating bar except the
  text toolbar's (which the column also has).
- **SC-002**: A presenter can find any one setting by looking only in the
  column. In a check of 10 named settings, each is found without opening a
  menu.
- **SC-003**: Each setting changed from the column writes the same deck text
  as the CLI op for it, and is undone in one step.
- **SC-004**: A build order changed in the column is the order the presented
  slide shows its builds, for a slide of at least 5 builds.
- **SC-005**: The column keeps its place and visibility across a reload in
  each of its 4 places, and is below the slide in a 700 px window.
- **SC-006**: The person, on a copy of their own talk, sets a background, a
  transition and two builds from the column, and says the settings are
  where they expect them.

## Assumptions

- Effects and transitions are those Quarto's reveal.js presentations offer
  (fragment classes, transition names and speeds, auto-animate). No
  custom animation engine is added.
- The column's width is about the inspector's (320 px), and it can be
  resized as the source pane can.
- The right-click menus stay, with the same actions, so nothing is lost
  for people who reach for them.
- Look presets (a few ready-made styles for a box) are out of scope, unless
  they already exist for diagrams (spec 015's starter looks).
- Themes are Quarto's built-in ones and a custom `.scss` already named in
  the deck. Making a theme is out of scope.
- Audio, slide layouts and new kinds of content are out of scope.
