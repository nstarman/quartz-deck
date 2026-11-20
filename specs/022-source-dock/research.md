# Research: Dock the source pane by dragging it

## R1. Clean room

- **Decision**: drop zones are plain translucent rectangles in the accent
  colour with our own labels ("Before the slide list", "Beside the slide
  list", "Right of the slide", "Below the slide").
- **Rationale**: dragging a panel onto highlighted zones is a general idea
  shared by many IDEs; `CLEANROOM.md` allows general ideas.
- **Alternatives**: a compass-style dock target in the middle of the
  window. More to draw and closer to specific products.

## R2. How the four places are laid out

Today `<main>` is a flex row: nav, history, source pane, splitter, stage,
panel. `main.source-right` moves the splitter, pane and both asides after
the stage with `order`.

- **Decision**: `editor.js` sets `main.dataset.source` from `source.side`.
  - `left`: no rule, the DOM order (today's default).
  - `far-left`: pane `order: -2`, splitter `order: -1`, before the nav.
  - `right`: stage, then `#panel` (`order: 2`), splitter (`3`), pane (`4`).
  - `below`: `main` becomes a grid only in this place: columns
    `auto auto 1fr auto` (nav, history, stage column, panel), rows
    `1fr auto auto` (stage, splitter, pane); nav, history and panel span
    all rows. A hidden column is `display: none` and takes no width.
- **Rationale**: the side places are pure `order`; the pane's element
  never leaves the DOM, so the textarea keeps its state (FR-004). Only
  `below` needs two rows.
- **Alternatives**: moving the pane's element with `append`. It would keep
  the textarea too, but the layout would live in two places (JS and CSS)
  and `order` already does it. A grid for every place: more change to the
  tested default layout.

History (spec 012) stays beside the nav in every place. Today it moves to
the right with `source-right`, which made it sit away from the list it
belongs to; with the inspector now between the slide and the pane on the
right, keeping it left is simpler.

## R3. The splitter

- **Decision**: one `#splitter`. In side places it is vertical and sets
  `--srcw` (as today). Below the slide (and on a narrow window) it is
  horizontal: `aria-orientation="horizontal"`, `cursor: row-resize`, Up
  and Down keys, and sets `--srch`, remembered in a new `ui.source_height`.
  The drag direction follows the pane's side: for `right` the width grows
  leftwards (today's `onRight`), for `below` the height grows upwards.
- **Rationale**: a width that suits a side column is wrong as a height;
  two values let each place come back as it was (US3 scenario 1).
- **Alternatives**: one size as a fraction for both. Loses the person's
  pixel choice when switching back.

## R4. The drag

- **Decision**: `dock.js` listens for `pointerdown` on `.srcbar` when the
  target is not inside a `button, input, select, a`. After 4 px of
  movement it captures the pointer, adds `main.docking` (user-select off)
  and shows four zone elements positioned from the current rects: a strip
  at the window's left edge, one at the stage's left edge, one at the
  window's right edge, one along the stage's bottom. The zone under the
  pointer gets `.over`. `pointerup` over a zone writes `source.side`;
  anywhere else, Esc, `pointercancel` or `lostpointercapture` cancels. The
  current place's zone is shown but a drop on it changes nothing.
- **Rationale**: the existing `settings.set` path already applies and
  broadcasts the change (US2 scenario 3), so the drop is one call.
- **Alternatives**: HTML drag and drop (`draggable`). Its ghost image and
  the textarea's own drag handling get in the way; pointer events are
  what the splitter and the slide list (spec 018) already use.

## R5. The right side with the inspector

- **Decision** (with the person): slide, inspector, source pane. The pane
  is at the window's edge. `inspector.fits()` measures the stage, so it
  needs no change: the pane's width is already taken off the stage.
- **Rationale**: the inspector edits the slide; it stays next to it.

## R6. A narrow window

- **Decision**: inside the existing `@media (max-width: 700px)` block,
  where `<main>` is already a column, the pane and splitter get
  `order` after the stage whatever `data-source` says, the grid of `below`
  is turned off, and the splitter is horizontal. `dock.js` shows only the
  `below` zone when `matchMedia('(max-width: 700px)')` matches. The
  setting is not written by the window size.
- **Rationale**: one media query; nothing to undo when the window widens.
- **Alternatives**: writing `below` to the setting when narrow. It would
  lose the person's place.
