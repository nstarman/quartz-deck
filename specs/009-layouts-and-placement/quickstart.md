# Quickstart: Layouts and placement (009)

Prerequisites: `cargo build --release`, Quarto, a copy of
`specs/008-usability/audit/deck/` somewhere writable, `qd serve` there.

1. **A slide from a layout (US1).** Click the + after "Highlights": the
   gallery shows each layout's preview. Pick "Two columns": a slide with
   "Click to add title" and two "Click to add text" boxes. Type a title and
   text in both. The file holds the heading and two positioned boxes with
   that text, no `.qd-ph`. ⌘Z removes the slide in one step.
2. **CLI (US1).** `qd layouts` lists 12 built-in layouts with placeholders.
   `qd add --layout image-text --after growth --title "Chart"` returns three
   placeholder refs; `qd edit <text ref> --md "Users doubled."` fills one.
   `quarto render talk.qmd`: no placeholder words in the output.
3. **Place freely (US2).** ⌥-drag the bullet list on "Highlights" to the
   lower right: it stays there; the title stays where it was. Drag a corner
   to resize; arrow keys nudge (⇧ by 10). Present: same spot. "Put back in
   the flow": `git diff` shows nothing.
4. **Refused (US2).** Right-click the columns on "Two views": "Place freely"
   is greyed out, its tooltip says why.
5. **Own layouts (US3).** On the slide from step 3, Slide ▸ Save slide as
   layout…, name "list-corner", keep it with the deck: `_layouts/list-corner.md`
   appears; the gallery lists it under "This deck's"; a slide from it has the
   list's placeholder in the corner.
6. **Guides (US4).** Drag a placed box near the slide's centre: a centre
   line shows and it snaps. Hold ⌘ while dragging: no snap. View ▸ Snap to
   guides off: no snap.
7. **Front and back (US5).** Overlap two placed boxes; Arrange ▸ Bring to
   front on the lower one: it's on top on the canvas and when presenting.
