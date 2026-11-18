# Contract: instant diagram drawing in the editor

No CLI command changes. The file and the rendered deck are exactly as
before; only what the editor shows between an edit and its render changes.

## After an edit the editor makes

1. **The op returns** (any `dot` op, a diagram nudge, undo or redo).
2. **Re-read**: `dot.js` reads `dot show` for each editable diagram on the
   shown slide.
3. **Redraw** each cell whose `text` changed:
   - Graphviz lays it out in the page;
   - its SVG replaces the cell's drawing, keeping Quarto's `width`,
     `height` and `style`;
   - pictures are drawn as `dot-image` draws them;
   - the clickable arrow copies, highlight and handles are added again;
   - the cell's text goes into `shown.drawn`.
4. **The dragged box**: its `transform`, the stand-in lines and the live
   copies of its arrows are removed, because the new drawing has the box in
   its place.

**Timing**: under 100 ms from the release to the new drawing (SC-001).
The next drag can start at once; nothing is `aria-busy`.

## When a render finishes (`rendered` event)

- **Skip** the reload when the rule of data-model.md holds. The status
  reads "Up to date" and nothing on the slide changes.
- **Otherwise** reload as today. That means any other change, an unknown
  shown source, or a cell that couldn't be drawn.

## While dragging

With `diagrams.live-layout` on (now the default), the arrows of the
dragged box are routed live, at most once a frame. The box always follows
the pointer.

## Failure

- **No engine, or a layout error:** nothing is drawn; the render reloads
  as today, and the reason goes to the console.
- **A render error:** the drawing stays, and the banner shows as today.

## Tests

`crates/qd/tests/onslide.rs`. The render is slowed on purpose, so "before
the render" can be seen: `QD_QUARTO` points at a small script that
sleeps, then runs Quarto (`Ed::open_with`'s env).
- **`dot_drop_draws_at_once`:** routed arrows within 100 ms of the release,
  and a second drag starts before the render;
- **`dot_render_lands_without_reload`:** the iframe's document stays the
  same object when the render arrives;
- **`dot_instant_matches_render`:** the drawing and Quarto's render have the
  same shapes, paths and `viewBox` (SC-003);
- **`dot_instant_pictures`:** a box with a picture keeps it in place;
- **`dot_undo_draws_at_once`:** undo of a move;
- **`dot_other_edits_still_reload`:** a text edit, or adding a picture
  (installs `dot-image`), reloads as before.

`crates/qd/tests/plugins.rs` (or `dot.rs`):
- **`dot_image_js_matches_lua`:** the plugin's example drawn by `dot.js`
  matches Quarto's render.
