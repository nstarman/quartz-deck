# Quickstart run (T023, 2026-11-12)

Run in the Claude app's browser pane (628 × 753 px, so the menus fold into ☰) on
a fresh copy of `specs/008-usability/audit/deck/`, with `qd serve` from the
`009-polish` branch. Changes were checked with `git diff` on the copy.

| § | Step | Result |
|---|---|---|
| 1 | + after Highlights → gallery (Two columns preselected from last time) → Add slide; type a title and both columns | Pass. "Click to add title" and two "Click to add text" boxes on the canvas; the file holds the heading and two positioned boxes with the text, no `.qd-ph`. |
| 2 | `qd layouts`; `qd add --layout image-text --after growth --title Chart`; `qd edit chart/1 --md "Users doubled."`; render | Pass. 12 built-in layouts; the add returns two placeholder refs (image, text: the title was given). Rendered with Quarto (a copy without the code slide, which needs Jupyter): no "Click to add", "Drop an image" or purpose words, and the filled text is there. |
| 3 | Place the Highlights list freely, drag it to the lower right, resize from a corner, ← and ⇧↑, then Arrange ▸ Put back in the flow | Pass. Wrapped with its bytes unchanged; the title stayed; after Put back, `git diff` shows nothing for the slide. The pane's drag tool can't hold ⌥, so "Place freely" came from the right-click menu; ⌥-drag itself is covered by `place_freely`. |
| 4 | Right-click the columns on "Two views" | Pass. "Place freely" greyed out; tooltip "A div (columns, callout…) can't be placed: place the elements inside it". |
| 5 | List placed in the corner; Slide ▸ Save slide as layout… "list-corner", with this deck; gallery; rename and delete there; a slide from it | Pass after a fix (below). `_layouts/list-corner.md` holds the heading, a text placeholder at the list's place and an empty notes block; the gallery lists it under "This deck's"; a slide from it has the placeholder in the corner. |
| 6 | Guides and snapping | Covered by `guides_and_snapping` (a drag near the centre snaps and shows a guide; ⌘ held and the setting off don't). Not repeated by hand: the pane can't hold ⌘ while dragging. |
| 7 | Bring to front | Covered by `front_and_back`. The Arrange menu showed the four items greyed out for a lone placed box, as it should. |

## Found and fixed in this run

- **A placed element saved as a stacked placeholder.** An element placed
  without a height (the usual case) has no full px place, and the save sheet
  only measured flow elements, so the layout put it under the title. The sheet
  now measures every element; a box with a full px place still uses its own.
  Unit test: `a_placed_box_without_a_height_uses_where_it_shows`.
- **"Forbidden" with no hint.** Saving a layout or dropping an image from an
  editor opened without the session link said only "Forbidden". It now says
  the editor link from `qd serve` or `qd url` is needed, as running code does.
- **The save sheet's "Keep it" radios** sat on their own line, away from their
  labels. They now sit beside them.

## Seen, not changed

- After an edit, the canvas shows the first slide for a moment while the new
  render loads. PR #41 (browser-test reliability) works on that swap.
- The preview's off-slide outline showed on the list placed near the bottom:
  that list ran past the slide's lower edge, as the marker says.
