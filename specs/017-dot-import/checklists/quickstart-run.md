# Quickstart run: import a drawing

Run on 2026-11-20 with a release build of main after #107 and #109, in a
scratch folder with the fixtures.

- [x] §1 **Dry run**: 4 boxes, 3 arrows; `left_out` lists the stray text
  ("outside every box"), the stray line ("its end at x=60 y=480 is near no
  box") and the polygon ("not a box, line, text or picture qd reads"); a
  `diff`; the file unchanged.
- [x] §2 **Import**: `qd dot show` reads it as editable and pinned. The
  ellipse is `shape=ellipse`, `tool` has its link and picture, and the
  two-headed arrow has `dir=both`. `dot-image` is installed and named in
  `filters:`.
- [x] §3 **Render**: covered by `import_renders_where_drawn`. Boxes sit
  within 5 px of `hand.svg`, at one unit to one pixel, with the picture
  drawn. A real drawing was also rendered (see below).
- [x] §4 **Undo**: one `qd undo` takes the cell out.
- [x] §5 **Graphviz**: `kind: graphviz`, the six names of the source DOT,
  5 arrows.
- [x] §6 **Replace**: `--replace` on the slide's `![](hand.svg)` swaps it
  for the cell.
- [x] §7 **Errors**: the deck itself gives `E_INVALID_ARG`, and only lines
  give `E_INVALID_ARG` ("no boxes in this drawing…"). The file is unchanged.

## The person's own drawings (T017)

Two box-and-arrow pipeline drawings from the person's talk were imported as
copies, in a scratch folder. Nothing about them is in the repository.
- **Result:** both came in whole, with 4 boxes, 3 arrows and nothing left
  out.
- **Changes from trying them:**
  - Names now come from a label's first line (`1background`, not the whole
    two-line label run together).
  - A drawing wider than the slide (these are 1480 units wide and shrunk by
    CSS in the talk) is scaled down to fit. It rendered at 1050 × 84 px.
  - The drawing's font (set on its root `<svg>`) now carries over as
    `fontname`.
- **Left for the person:** the talk's original hand-drawn diagram from
  spec 015 (SC-005) went to the Trash with the unused files, so it wasn't
  tried. Try it if it's still there, and give the boxes' look a glance on
  the slide.
