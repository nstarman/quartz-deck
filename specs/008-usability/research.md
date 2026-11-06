# Research: Usability (008)

Decisions for the plan, each checked against the code on `main` (6f3cfbf) and
the audit ([audit/README.md](audit/README.md)).

## R1. Thumbnails: copies of the canvas's own slides

**Decision**: each list item shows a scaled copy of that slide's `<section>`,
taken from the preview frame that already holds every slide (reveal.js keeps
all sections in the DOM). Each copy lives in its own shadow root, with the
deck's stylesheets shared through one constructed stylesheet (fetched once per
render) and relative image and link URLs made absolute. The slide's
background (`data-background-color`/`-image`) is set on the thumbnail's box.
Copies are rebuilt when a render lands, only for slides whose source changed,
and only for items in view (IntersectionObserver).

**Rationale**: the same render the canvas shows (the spec's assumption), no
JavaScript runs in a thumbnail, and 100 slides cost DOM copies, not 100
reveal.js instances. Math (MathJax output is DOM) and figures copy as they
are.

**Alternatives**: one small iframe per thumbnail, as today's hover pop-up
does (each starts reveal.js: ~10–20 MB and a script start per slide; too heavy
past a few dozen); screenshots (no browser API for an iframe's pixels without
a server-side browser).

**Ceiling** (`ponytail:` in the code): interactive outputs (Plotly, widgets)
show as their static DOM; a thumbnail of a slide whose content needs a script
to draw is blank until 012-style work, noted in the list with its title.

## R2. The title slide in the list

**Decision**: when the deck has a title slide (`title_slide` in `qd slides`),
the list shows it first, unnumbered and not draggable. Clicking its title or
subtitle on the canvas edits the front matter (`qd meta set title …`), as
editing other titles edits headings.

## R3. Adding a slide anywhere; untitled slides

**Decision**: a + appears in the gap between two list items on hover and at
the end of the list; it runs `qd add --after <slide>` (or `--before` the first
slide). Enter on a focused list item adds after it (action `slide-after`,
which keeps ⇧⌘N). A new slide is added **untitled**: `qd add` without a title
writes `## {#s-xxxx}`, which renders as an empty `<h2>`. The editor's preview
CSS shows "Click to add title" on an empty slide heading
(`section > h2:empty::before`), editor-only; clicking it starts the title edit
that already exists (`qd title`).

**Rationale**: no placeholder text in the file (FR-003); reuses `qd add`, `qd
title` and the existing title editing.

## R4. Dragging slides: pointer events, our own drag image

**Decision**: the list's drag is rewritten with pointer events instead of
HTML5 drag and drop: a ⠿ handle on hover (the whole item stays draggable
after a few points of movement), a translucent copy of the thumbnail follows
the pointer, items slide apart (CSS transform with a short transition) to
open a gap at the landing index, the list auto-scrolls within 40 px of its
edges, Escape cancels, and the drop runs one `qd move`. ⌥↑/⌥↓ on a focused
item run `slide-move-up/down` (existing actions, new keys).

**Rationale**: HTML5 drag and drop gives no control over the drag image's
look while moving, can't animate the gap, and fires inconsistently in
Safari; pointer events give the continuous feedback the guidelines ask for.
The drop stays one op, one undo step.

## R5. Bars never cover content

**Decision**: one placement function for the element bar and the formatting
bar: try above the element, then below it; a position is rejected if the
bar's rectangle intersects any other `[data-qd]` element or the slide's
heading. If both are rejected, the bar docks into a strip at the top of the
canvas area (the slide bar's row, which becomes context-dependent while an
element is selected). The ResizeObserver that re-places the bar stays.

**Rationale**: covers C1 with the existing bars; no new UI surface beyond the
docked state.

## R6. reveal.js's own chrome; the canvas doesn't move

**Decision**: the editor's frame hook (where it already sets
`scrollActivationWidth` and disables Escape) also sets `controls: false,
progress: false, slideNumber: false` and hides `.slide-menu-button`, as the
hover pop-up already does. The slide bar keeps its row on every slide
(`visibility: hidden` with its controls disabled on the title slide), so the
canvas never moves (C3).

## R7. Present: full screen in place

**Decision**: Present makes the preview frame full screen
(`requestFullscreen` on the person's click or key) with reveal.js's keyboard,
controls and progress turned back on, starting at the current slide; "From
the beginning" goes to the first slide first. Leaving full screen restores
the editing configuration. The existing "open in a new tab" (`/present/`)
stays in File, renamed "Open in browser tab", for a second screen and
reveal's speaker view.

**Rationale**: one click, from the current slide, with the render already on
screen (no load); full screen needs a user gesture, which the click is.

**Alternatives**: open `/present/` in a new tab and ask the person to press
F (two steps, and the new tab can't go full screen by itself).

## R8. The editor's own sheets

**Decision**: one small module, `sheet.js`, renders an anchored sheet (a
`<dialog>` positioned by what opened it) from a description: title, fields
(text, colour swatches with None, file list with previews, checkboxes),
validation per field with an inline message, and OK/Cancel. It returns a
promise of the values. Every `prompt`, `confirm` and `alert` (nine, listed in
the audit) moves to it. A Rust test scans `src/ui/*.js` for `prompt(`,
`confirm(` and `alert(` (as 007's scan for stray key handling) so none come
back.

## R9. Images: a list beside the deck, and dropping a file

**Decision**:
- **Listing:** `GET /api/images` returns the image files under the deck's
  folder (png, jpg, jpeg, gif, svg, webp; up to 3 levels deep; hidden folders,
  qd's cache and the export folder skipped) as paths relative to the deck,
  newest first. CLI: `qd images` (constitution III).
- **Dropping:** a file dropped on the canvas is sent as its raw bytes to
  `POST /api/images?name=<file name>` with `Content-Type:
  application/octet-stream` and the token header, like other writes. The server checks the extension and the file's magic bytes,
  limits size (20 MB), keeps only the base name (safe characters), writes it
  to `images/` beside the deck (made if missing) under a free name (`x.png`,
  `x-1.png`, …), never overwriting, and returns the relative path; the editor
  then runs `block add` with `![](images/x.png)` at the drop point. CLI: `qd
  image add <file> [--slide S --after REF]` copies and inserts the same way.

**Rationale**: neither `application/octet-stream` nor the token header is
allowed in a cross-origin request without a CORS preflight, which qd never
grants, so this keeps the defence the JSON-only writes have, with no base64
and no new dependency. Writing only under
`images/`, with checked types and a free name, keeps it to what the person
dropped.

**Security note** (for the security checklist): SVG can hold script. Images
are shown through `<img>` (no script), but `/preview/` serves deck files on
the editor's origin; a dropped SVG opened directly there could run script on
that origin. The API needs the token, which is never in the page, so such a
script still can't edit or run code; the checklist records this and the
attack test gains a POST to `/api/images`.

## R10. Contrast, sizes and themes

**Decision**: CSS only, plus a test.
- `color-scheme` on `:root` per theme, and explicit colours for `select`,
  `input`, `textarea` and the colour swatch from the theme tokens (T1).
- The source pane's highlight colours become tokens with a light, dark and
  high-contrast value each, chosen to pass 4.5:1 on the pane's background (T2).
- One control height token (28 px) for bar buttons, selects and toggles;
  minimum 20 px targets for handles (larger hit area than the visible square),
  the gutter's ▶ and checkboxes (T3); the preview's outline colours from the
  theme accent (T5).
- Chosen (2026-11-06), contrast on the pane (white / #222225 / black) and on
  the page background: light — YAML #755b2d 6.38/5.90, fences #7d3c98
  7.07/6.53, code #17703d 6.14/5.67, options #806600 5.50/5.08; dark — #d9bc85
  8.68, #c9a0e8 7.31, #7fc99a 8.10, #e3c75f 9.52; high contrast — #ffd98a 15.5,
  #e0b8ff 12.5, #9cf0b8 15.6, #fff07a 18.0.
- The selection outline on the slide stays blue in every theme: the slide is
  white, so the high-contrast yellow would be 1.07:1 (T5 kept).
- A browser test (`audit_measures`) walks every visible control and text in
  each theme on the audit deck and fails on contrast below the minimum or
  targets below 20×20 (SC-004, SC-005), and checks that no bar overlaps
  another element as each element is selected and edited (SC-003).

## R11. One top bar; words

**Decision**: the menu bar and the header become one row: menus (or ☰),
the deck's title (file name smaller, both from `qd slides`/the path), the
status, Present, Source. The tab title is the deck's title. The status maps
the editor's states to words: "Saved", "Saving…", "Rendering…", "Up to date",
"Changed on disk", "Disconnected — is qd serve running?"; the version is the
tooltip. Labels change in `actions.json` (one place; 007), and in the bars:
Render → "Render with Quarto", Open preview → "Open in browser tab", Export
offline HTML → "Export ▸ Web page (works offline)", Export with PDF → "Export
▸ Web page and PDF", + Package → "Add Python package…", Create environment →
"Set up Python…", Form → "Properties", On click → "Appears on click", Edit
source (element) → "Edit markdown". Tests that click labels are updated with
them.

## R12. Bars and Insert

**Decision**:
- **Slide bar:** colour as a swatch menu (palette, None, Custom…), image via
  the image sheet, transition and hidden as before, one height; labels or
  tooltips on each (C4).
- **Formatting bar:** B, I, U, S, link, bullets, colour menu, and "Aa" (a
  pop-over with code, numbered list, indent, outdent, size, alignment as a
  segmented control); colour names in tooltips (E1).
- **Source bar:** ▶ Run all, ■ Stop, the session state in words ("Python ·
  idle/running/stopped"), and ··· (Restart, Restart & run all, Add Python
  package…, Set up Python…); Full width and Properties as two small toggles
  at the pane's top right (S1). Gutter ▶ at least 20 px (S2).
- **Insert:** adds Code cell, Callout, Columns, Speaker notes, Table, Quote,
  each a markdown snippet through `block add` (C5), so `qd block add` already
  covers them.

## R13. Narrow windows

**Decision**: under 600 px, a media query hides the list; a slide picker (a
button with the current slide's number and title that opens a menu of
thumbnails) sits in the top bar; the slide bar's controls move into a ···
menu (N2). Every action stays reachable through the menus or ☰.

## R14. P3 polish

- **Multi-select:** ⇧-click and ⌘-click select ranges and sets in the list;
  move, copy, cut and delete apply to all as one step. `qd move` and `qd rm`
  accept several slides and make one patch each (CLI first).
- **Settings:** Theme first, controls one width.
- **"Saved":** a small label next to the element for 1.5 s after a save.
- **Undo after duplicate:** when the slide on the canvas disappears through
  undo, show its neighbour that remains (the one it was copied from, if
  any), not the first slide.

## Licences

No new dependencies. Playwright is used only by the audit script (run with
`uv run`, not part of the build).
