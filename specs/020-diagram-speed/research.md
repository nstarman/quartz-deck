# Research: Diagrams that keep up with the hand

All measurements are on a made-up two-slide deck with a seven-box `{dot}`
cell, on 2026-11-17, with the machine under heavy load.

## R1. Where the time goes

**Finding**: one drop of a box, timed in the editor:

| Step | Time |
|---|---|
| `qd dot move` writes the file | 0.01 s |
| the server waits before rendering | 0.15 s, by design (`render_loop`) |
| Quarto renders the whole preview copy | 2.8 s |
| the preview iframe reloads | 0.06 s |

`quarto render` of the same deck alone took 5.0–5.6 s, against 2.9 s for a
deck with no diagram, so Quarto's Graphviz step costs about 2 s on top of
Quarto's own start. The render grows with the deck. Renders run one at a
time; edits made during a render are batched into one more render after
it.

**Conclusion**: the drag isn't slow; the wait for Quarto after the drop
is. The editor has to show the result without that wait.

## R2. The same engine gives the same picture

**Spike** (2026-11-17), in the editor's page:
- loaded Quarto's `graphviz-wasm.js` with the existing Deno shim (spec 015 R6);
- laid out the cell's text from `qd dot show` with the cell's layout engine;
- compared it with the SVG Quarto had rendered into the preview.

**Finding**:
- **Same picture:** every node shape, label position, edge path and
  arrowhead is identical, and so is the `viewBox`.
- **What Quarto adds:** it keeps Graphviz's SVG and only sets `width`,
  `height` and a `style` on the root, for the figure's size.
- **What the editor adds:** clickable copies of each arrow
  (`path.qd-dot-hit`).
- **Speed:** a layout takes under 1 ms for this cell and 2–6 ms for
  20 boxes and 25 arrows, after the engine loads once (about 10 ms).

**Decision**: the instant drawing is Graphviz's SVG from the browser. The
editor puts its children and `viewBox` into the cell's existing `<svg>`,
keeping Quarto's `width`, `height` and `style`. It then re-adds its
clickable arrow copies, the selection highlight and the handles, as after
a render.

**Alternatives**:
- **Render the cell on the server:** it would need Graphviz in qd (a new
  dependency) or Quarto's Deno. Rejected.
- **Move only the dragged box's group and leave the arrows:** that is
  today's look. Rejected.

## R3. Pictures in boxes

**Decision**: port `dot-image`'s `draw` step (about 25 lines of Lua) to
`dot.js`. For each box of `qd dot show` with an `image`:
- find its node by `<title>`;
- take the shape's bounds;
- place an `<image class="dot-image">` at the spacer (left of a
  start-anchored label, else the box's left), with the same size rule;
- put it after the shape and before the label.

The path is the box's `image` as written, which the preview serves
relative to the deck, as Quarto's output references it.

**Test**: the plugin's own example deck, rendered by Quarto, and the same
cell drawn by `dot.js` give the same `<image>` positions, within 0.01.

**Rationale**: `dot-image` is qd's own plugin (spec 015 US3), so mirroring
it is allowed and has a test to keep the two in step. Without it, a box
with a picture would lose it until the render arrived.

## R4. When a finished render needn't reload the preview

**Decision**: the editor remembers:
- the deck's source for the render the preview shows (fetched from
  `/api/source` when that render loads);
- the text of each diagram cell it has drawn since.

When a newer render arrives, the editor fetches the source again and
compares the two with every `{dot}` cell's body taken out. The reload is
skipped (status "Up to date", nothing moves) when both hold:
- the rest is identical;
- every diagram cell now has the text the editor last drew for it, or the
  same text as before.

Otherwise it reloads as today.

**Rationale**: it is one rule for every source of change: a drag, a
nudge, undo, redo, a restyle. It can't skip a reload that something else
needed, because any change outside drawn cells forces one. R2 shows
the drawing equals the render, so skipping is invisible.

**Alternatives**:
- **Swap the cell from the new render's HTML:** fetch the page and replace
  the cell. That is exact, but it fetches a large page per drop (decks
  with embedded images reach tens of MB). It is kept as the fallback
  idea if R2 ever stops holding.
- **Trust the op name** (`dot …` ops change one cell): undo and redo don't
  say what they changed. Rejected.

## R5. Which edits draw instantly

**Decision**: after any op the editor sends (the `dot` ops, nudges, undo
and redo), `dot.js`:
- re-reads `qd dot show` for each editable diagram on the shown slide (one
  quiet request each, as `mark()` already does);
- redraws each cell whose text changed.

Edits from outside the editor (the CLI, an agent) arrive with the render,
as today.

## R6. Live routing on by default

**Decision**: `diagrams.live-layout` defaults to `true` (in
`crates/qd/src/settings.rs`; the settings test changes with it).

**Rationale**: the existing `route()` already lays out at most once a
frame and asynchronously, and it skips frames while a layout is still
running, so the box keeps the pointer's rate (SC-004). At 2–6 ms a layout
fits in a frame. The setting stays for anyone who wants straight stand-in
lines.

## R7. When the instant drawing can't be made

- **No engine** (an old Quarto, a failed load) or a layout error: nothing
  is drawn and the render reloads as today. The error goes to the
  console.
- **A render error:** the drawing stays, and the banner shows as today.
- **The person typing:** a render is already held back (`pendingRender`).
  Instant drawings happen only from diagram ops, which never run while
  text is being typed.

## R8. Not in this feature

A faster Quarto render for every edit, by rendering only the changed slide
or keeping Quarto running between renders. It is worth its own spec: it
changes the render loop and the preview's loading for every kind of
element.

## Licences

No new dependency. The Graphviz engine is Quarto's, already used in spec
015.
