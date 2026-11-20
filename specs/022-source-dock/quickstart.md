# Quickstart: check the source pane's places

Prerequisites: `cargo build --release`, any deck (`examples/` has some).

1. `target/release/qd serve <deck>.qmd`, open the editor link in the
   browser pane, open the source pane (View ▸ Source).
2. Drag the pane's title bar (on the word "Source"): four zones show.
   Drop on each in turn. The pane moves; the text and caret stay.
3. Press on "Run all" and move: no zones, the button works.
4. Start a drag, press Esc: the pane stays.
5. Dock below; drag the splitter up; reload: the pane is below, same
   height. Dock left; the width is the one it had.
6. Full width in each place: the slide hides, the pane fills its room.
7. Window 1440 px wide, pane on the right, select an element: slide,
   inspector, pane, left to right.
8. Window 650 px wide: the pane is under the slide whatever its place;
   widen again: back to its place.
9. `qd settings set source.side far-left`: the open editor follows.

Automated: `cargo test -p qd --test onslide source_dock` (the tests this
feature adds), then the whole suite.
