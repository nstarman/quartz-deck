# Editor contract: arrows on the slide (016)

The overlay is `ui/arrows.js`. It changes the file only through the
`qd arrow` commands (POST /api/op), and the preview's re-render draws the
result.

| gesture | where | command |
|---|---|---|
| Insert ▸ Arrow, then drag on the slide | menu, slide | `arrow add SLIDE --from --to` |
| Insert ▸ Arrow, then click | menu, slide | `arrow add` with a 200 px arrow pointing right |
| click the line | an arrow | selects it (no command) |
| drag the start or end handle | selected arrow | `arrow move REF --from` / `--to` |
| drag the line | selected arrow | `arrow move REF --by` |
| drag the middle handle | selected arrow | `arrow bend REF --through` (or `--straight` near the line) |
| arrow keys (⇧ for 10 px) | selected arrow | `arrow move REF --by` |
| Delete / ⌫ | selected arrow | `block rm REF` |
| double-click | an arrow | the label sheet, then `arrow set REF label=…` |
| Properties panel | selected arrow | `arrow set REF …` (colour, width, dash, head, heads, label position) |
| element bar "Appears on click" | selected arrow | `arrow set REF fragment=true` / `fragment=` |

**While drawing or dragging:**
- A dragged end, or each end of a moved arrow, snaps to the slide's and the
  elements' edges and centres within 6 screen px, with a guide line, when
  "Snap to guides" (`placement.snap`) is on. ⌘/Ctrl places freely.
- Moving the whole arrow shows its rendered SVG translated. Dragging an end
  or drawing shows a dashed provisional line with a small head (research
  R8).
- Escape cancels; nothing is written.

**Read-only arrows** (in the flow, or unreadable): a click selects the
element as usual; the source opens on request. `qd arrow show` gives the
reason.

**Accessibility:** the handles are buttons named "Arrow start", "Arrow end"
and "Bend arrow"; every gesture has a menu item or a key; the arrow keys
nudge.

**Actions** (`actions.json`): `insert-arrow` (Insert), and the element
menu's existing Delete, Properties and "Appears on click", which act on an
arrow through the commands above.
