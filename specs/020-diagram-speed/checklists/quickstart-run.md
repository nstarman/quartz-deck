# Quickstart run: diagrams that keep up with the hand

Run on 2026-11-18 with a release build of main after #94 and #95 (plus
this PR's fix), on a machine under heavy load (load average 50–250).
Timings were taken in the editor page: `Date.now()`/`performance.now()` at
the release, at the drawing's change (a `MutationObserver` on the cell's
`<svg>`), and when the status turned "Up to date".

## Made-up deck (two slides, one diagram of 8 boxes)

- [x] §1 **Drop**: drawn **79 ms** after the release. The render landed
  3.9 s later, with no reload: the preview's `document` stayed the same
  object.
- [x] §2 **Drag again at once**: covered by `dot_drop_draws_at_once`, which
  makes two drops, an undo and a nudge, each drawn before a render slowed
  by 4 s.
- [x] §3 **Live routing**: on by default (#95). `dot_live_routing_keeps_up`
  covers it: 30 fast moves on a 20-box diagram, with the box within 1 px of
  the pointer at each.
- [x] §4 **Other diagram edits**: they go through the same `op()` →
  `dot.changed()` path. A nudge is checked in `dot_drop_draws_at_once`.
- [x] §5 **Undo**: `dot_drop_draws_at_once`.
- [x] §6 **Pictures**: `dot_image_js_matches_lua` (now with a linked box,
  see below).
- [x] §7 **Other edits unchanged**: `dot_other_edits_still_reload`. A title
  change and an outside undo both reload.

## The person's talk (a copy in a scratch folder, T014)

**The diagram:** 15 boxes, 15 arrows, 4 pictures, linked boxes.
**The deck:** a full render takes about 24 s.

- **First try: not drawn, and the preview reloaded after the render.** A
  box with both a link and a picture has its label inside an `<a>`, and the
  editor's picture step inserted the picture beside the label as if the
  label were a direct child of the box. The Lua filter inserts beside the
  label wherever it sits. Fixed in `dot.js`. `dot_image_js_matches_lua` now
  uses a linked box with a picture: it failed before the fix and passes
  after.
- **After the fix:** dragging a linked box with a picture was drawn **24 ms**
  after the release, with its picture. The render landed **23.8 s** later
  without a reload. Before spec 020 the editor showed stand-in lines for
  those 24 s, then reloaded.
- **Not checked here:** live routing during the drag. The browser pane was
  hidden, so the page didn't run animation frames, and live routing waits
  for one. The drag was driven by pointer events sent in the page. It
  remains for the person to try by hand.
