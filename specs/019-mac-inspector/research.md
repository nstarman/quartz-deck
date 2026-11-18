# Research: An inspector on the right when the window is wide

## R1. Clean room

**Decision**: take only the general idea: a properties column docked on the
right, with tabs that follow the selection. The tab names (Slide, Title,
Element, Text, Position), the layout of the sections and the look are our
own, drawn with the editor's existing styles. No other app's screenshots,
icons or wording go into the repository, the tests or the docs.

**Rationale**: the constitution (Principle I) and `CLEANROOM.md` forbid
reproducing a proprietary slide editor's UI layout or wording, and Keynote
is one. `CLEANROOM.md` allows "general, non-proprietary UI ideas common to
many editors (… a properties panel)". A docked properties column whose
content follows the selection is common: LibreOffice Impress's sidebar
(open source), PowerPoint's and Google Slides' format panes, and design
tools all have one.

**Alternatives**: Keynote's tab names and look, as the person's screenshot
showed. Rejected by Principle I.

## R2. Move the existing controls, don't rebuild them

**Decision**: the inspector holds the existing controls, not copies:
- the slide bar's controls (background, image, transition, hidden, ⋯) go
  in the Slide tab, beside today's slide panel fields;
- the element bar's buttons (Run, Edit, Markdown, Appears on click,
  Delete, ⋯) and today's element panel (markdown, the diagram and arrow
  fields) go in the Element tab;
- the formatting bar (`toolbar.js`) goes in the Text tab;
- the element panel's position and size fields, plus Move up/down, go in
  the Position tab.

Where a control is a DOM node today (`#slidebar`, `#elbar`, `#fmtbar`), the
inspector moves that node into its tab when it shows and puts it back when
it hides. Their handlers stay as they are. The panel content comes from the
existing `renderPanel` / `renderElementPanel` / `renderTitlePanel`, split
into tabs by their sections.

**Rationale**: FR-004 (every control does exactly what it does today)
then holds by construction: there is one copy of each control. It is also
the smallest change.

**Alternatives**: build the inspector's controls from `actions.json`. That
would make a second copy of every control that could drift. Rejected.

## R3. When it shows

**Decision**: one class on `<main>`, `inspector`, set by a small function.
It runs on load, on a window resize and when the source or History pane
opens, closes or is resized. The class is on when all of these hold:
- the person hasn't hidden it (R5);
- the window is at least 1200 px wide;
- the slide's column would keep at least 640 px with the inspector beside
  it.

The CSS keys off the class: with it, the bars are inside the inspector;
without it, they float as today.

**Rationale**: a CSS media query alone can't see the source pane's width,
and the existing `ResizeObserver` on the preview already re-places the bars.
Both numbers are the spec's starting values, kept as two constants.

**Alternatives**: a container query on `<main>`. It can't move nodes or set
the selected tab, so JS is needed anyway. Rejected.

## R4. Formatting while typing

**Decision**: the Text tab shows the formatting bar's buttons while a typing
session is open, and a short note ("Click into text to format it")
otherwise.

**Rationale**: the formatting buttons already live in the editor page,
outside the preview, and never take focus. They act on the selection in the
preview through the session, so they work from the inspector unchanged.
`toolbar.place` / `placeNear` skip positioning while the bar is in the
inspector.

## R5. Showing, hiding and remembering

**Decision**:
- **Setting**: a new setting `ui.inspector` (bool, default true), in
  `settings.rs`, so it is in `qd settings` like the other `ui.*` keys.
- **Toggle**: a new action `inspector`, "Inspector" in the View menu, with
  ⌥⌘I. It is a general convention for an inspector, and free in
  `actions.json`. It also has a button at the right end of the header.
- **Properties**: the existing `form-panel` action ("Properties panel")
  shows the inspector on a wide window. Below the width it opens the panel
  as today (`ui.form_panel`).

**Rationale**: the person's choice is per person, not per deck, like the
other layout settings (spec 007). It is a view setting, not an edit, so
Principle III doesn't call for a CLI command beyond `qd settings`.

## R6. Tabs, keyboard and screen readers

**Decision**: the WAI-ARIA tabs pattern:
- a `tablist` with `tab` buttons and `tabpanel`s;
- arrow keys move between tabs (roving tabindex);
- Tab goes into the panel;
- F6 moves focus between the slide and the inspector, as F6 moves between
  regions in many apps (added to `keys.js` with the list allowance).

Tabs that don't apply are left out, not disabled: Text and Position for an
element that has neither, and Element/Text/Position on the title slide.

The selected tab follows the selection:
- nothing selected: Slide (or Title on the title slide);
- an element selected: the tab last used for elements, Element at first;
- typing: Text.

## R7. Several elements selected

**Finding**: the editor has no multiple selection of elements today; only
slides in the list can be picked together. The spec's edge case applies
when that exists. Until then the inspector shows one element's tabs. Noted
in the spec's edge case; nothing to build now.

## R8. VS Code panel and the narrow layouts

**Decision**: no special case. The VS Code panel serves the same page, so R3's
width check applies there as it does in a browser. Below 1200 px the existing
narrow layouts (spec 008 US8) are untouched.

## Licences

No new dependency.
