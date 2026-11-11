# Research: Layouts and placement (009)

Decisions for the plan, checked against the code on `main` (04d63a0).

## R1. What a layout writes: a heading and positioned placeholder boxes

**Decision**: a built-in layout writes a slide heading (or none, for blank and
full-bleed image) and its content areas as **positioned boxes**: the same
`::: {#b-… .absolute left= top= width= height=}` blocks text boxes use (001,
`ops::box_add`). "Two columns" is two boxes side by side; "image + text" an
image box and a text box. Speaker notes ("title + notes") are a `::: notes`
block.

**Rationale**: the deck model is flat: a slide's top-level blocks are its
elements (`Deck::element`, refs `<slide>/<i>` or `#id`). A `:::: {.columns}`
block is one element, so the content inside a column has no ref of its own,
can't be typed into on the canvas as a separate element, and can't be
returned to an agent as a placeholder (FR-005, SC-005). Positioned boxes are
top-level elements with explicit ids: each placeholder gets a ref, the
editor already selects, types into, moves and resizes them (001 US4, 003),
and Quarto renders them in place (FR-003). It is also how slide apps' layouts
behave: placeholders are boxes that can be moved and resized at once (US2).

**Alternatives**: `.columns` layouts with nested element refs
(`<slide>/<i>/<col>/<j>`) through the parser, the preview's `data-qd`
attributes, the editor's typing and every op — a large change to the core for
a structure that boxes give directly. Kept for later if decks need text that
reflows across columns.

## R2. Placeholders: empty, marked, and never in the output

**Decision**:
- **Text and image placeholders** are positioned boxes with no content and
  two editor markers in their attributes: the class `.qd-ph` and `ph=text` or
  `ph=image` (Pandoc writes unknown attributes as `data-ph`).
- **The title placeholder** is the untitled heading 008 already writes
  (`## {#id}`).

An empty fenced div renders as an empty `<div>`, so nothing shows when the
deck is rendered or presented (FR-004, SC-002). The editor's preview CSS
shows the prompt on the canvas only: `.qd-ph[data-ph=text]:empty::before {
content: "Click to add text" }`, and "Drop an image" for images, as it shows
"Click to add title" on an empty heading.

**Filling one:** typing into a text placeholder, dropping an image on an image
placeholder, or `qd edit REF --md …` replaces its content. The op that fills
a block marked `.qd-ph` drops the two markers in the same patch, so a filled
placeholder is an ordinary box. Deleting a placeholder deletes the box.

**Rationale**: no placeholder words are ever written into the file, and no
side file is needed (FR-003). The markers say "still empty" to the editor and
to agents (`qd show` lists them).

## R3. Layout definitions

**Decision**: each built-in layout is a small Quarto markdown file embedded
in the binary (`src/layouts/<name>.md`, `include_str!`), with a front matter
`name`, `description` and the slide below it, in a 1050×700 slide's px. Adding
a slide from a layout (`ops::add_layout`) reads it, scales every `left`,
`top`, `width` and `height` to the deck's slide size (`Deck::width`/`height`),
gives each box a fresh id (`ctx.fresh("b")`), sets the title if given, and
inserts it like `ops::add` (one patch). The result lists each placeholder's
ref, kind (`title`, `text`, `image`) and purpose (the layout file's comment for
it, e.g. "left column").

**Saved layouts** (US3) are files of the same form, in two places (FR-015):
- the person's own: `<qd config dir>/layouts/<name>.md` (as 007's settings);
- the deck's: `_layouts/<name>.md` beside the deck. Quarto skips files and
  folders starting with `_` when rendering a project, so these never become
  pages.

The lookup order is the deck's, then the person's, then built-in. `qd layouts`
reports where each one lives.

**Save slide as layout:** the slide's text is turned into a layout. Its title
becomes the title placeholder. Each positioned box becomes a placeholder of
its kind (text or image) at its position and size. Each block in the flow
becomes a placeholder box at the place it shows on the canvas, measured by
the editor and passed with the request. Notes are kept as an empty notes
block.

## R4. Placing an element freely

**Decision**: `qd block place REF --at X,Y [--size W,H]` (`ops::block_place`)
handles two kinds of element:
- **An image** (`![](…){…}`) gets `.absolute left= top= width= [height=]` in
  its own attributes, the form image boxes already use.
- **Any other block** is wrapped:
  `::: {#b-… .absolute left= top= width=}` + newline + the block's bytes
  unchanged + newline + `:::`. The height is left out unless `--size` gives
  it, so text keeps its natural height.

Both are boxes from then on (`BlockKind::Box`): `qd box move` and `qd box
resize` work unchanged.

`qd block unplace REF` (`ops::block_unplace`) reverses exactly what `place`
wrote:
- A box div whose attributes are only an id, `.absolute` and position or
  size, around one block, is replaced by that block's bytes.
- On an image, those attributes are removed (and the `{}` if it empties).

`unplace` on a box made as a text box (001) unwraps its text the same way.
Place followed by unplace is byte for byte the original (SC-003). The id `place`
gives the wrapper is dropped again by `unplace`.

**Refused, with the reason (FR-012):**
- a `.columns` or other div (callout etc.);
- speaker notes (`::: notes`);
- an element that is already a box;
- a slide's title, which is not a block.

Elements inside containers are not top-level blocks, so they have no ref to
place.

**Units (FR-011):** px of the deck's slide, the numbers boxes already use;
unitless numbers in Quarto's `.absolute` are px. The editor converts pointer
positions with `Reveal.getScale()`.

**Fragments (FR-013):** the `.fragment` class stays on the element inside the
wrapper, and reveal.js shows a fragment inside a positioned div on click as
usual.

## R5. Dragging, nudging, guides and snapping (editor)

**Decision**:
- **⌥-drag on an element in the flow** starts placement: a translucent copy
  follows the pointer (as 008's slide-list drag does). The drop runs `block
  place` at the copy's top-left with the element's rendered width, then the
  new box is selected.
- **Placed elements** reuse the box overlay (`attachOverlay`): drag, eight
  handles, and arrow keys. The arrow keys are new actions `box-nudge-*`
  (1 px, ⇧ 10 px) in the action list (FR-020). Each one runs `qd box move
  --by`.
- **Guides:** while dragging or resizing, the candidate lines are the slide's
  left, centre and right (and top, middle, bottom), plus the same for every
  other top-level element's box. The nearest within 6 screen px of the dragged
  box's edge or centre wins: the box snaps to it and a 1 px line is drawn over
  the canvas. This is pure arithmetic on rectangles, done on each pointer
  move.
- **Turning snapping off:** a new setting, `placement.snap` (on by default, in
  007's `DEFS`), and holding ⌘/Ctrl during one drag.

## R6. Stacking

**Decision**: positioned elements stack in source order (later is on top),
so stacking is a reorder.
- **Bring to front:** the box moves to after the slide's last box.
- **Send to back:** it moves to before the first.
- **Forward and backward:** it swaps with the next or previous box.

These run `qd box front|back|forward|backward REF`, each one `block_move`
patch. Moving a box past flow elements doesn't change the flow, because boxes
take no space in it. A fragment's click order follows source order too, which
the menu item's tooltip says.

## R7. The gallery

**Decision**:
- **Where:** the sheet module (008) gains a gallery field. Insert ▸ Slide,
  the list's + and ⇧⌘N open it beside what was clicked. ⇧⌘N stays "Slide
  after" (007), now opening the gallery.
- **Previews:** each layout's preview is drawn from its own boxes as small
  rectangles in a 3:2 frame: a bar for the title, outlined boxes for text, a
  hatched box for an image. No render is needed.
- **Choosing:** Enter or a double click adds the slide, and Esc adds nothing.
- **The last layout used** is remembered as a settings value
  (`ui.last_layout`).

`GET /api/layouts` lists the layouts with their boxes for the previews. The
add itself is `qd add --layout` through `/api/op`. Saving, renaming and
deleting a layout write files outside the `.qmd`, so they get their own
endpoint, `POST /api/layouts` (token, same origin), like 008's image upload.

## R8. Following the source pane and VS Code

Each change is one patch through the store, so the source pane (004), shared
focus (005) and VS Code's buffer (006) follow as they do for any op; nothing
new is needed.

## Licences

No new dependencies.
