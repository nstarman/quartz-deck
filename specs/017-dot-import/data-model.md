# Data model: Import a drawing as a diagram

Nothing new is stored. The drawing is read, the cell is written, and a
report is returned.

## Drawing (read from the SVG)

- **Shape**: a `rect`, `circle` or `ellipse` with its transformed bounding
  box, fill, stroke, rounded or not, and the `a` around it if any.
- **Text**: the anchor point (transformed), the string (its `tspan`s
  joined), the fill and the font size.
- **Line**: start and end points, the direction at each end, and markers
  at the start and end.
- **Picture**: an `image` with its href and bounding box.
- **Other**: anything else, kept only to report it.

## Box (one per Shape with text, or Graphviz node)

| Field | From |
|---|---|
| `name` | Graphviz `<title>`; else the label lowercased to letters, digits and `_`, made unique (`data`, `data_2`) |
| `label` | its Texts, top to bottom, joined with `\n`; with a picture, the spacer label |
| `pos` | centre ÷ 72, y up, normalised, pinned (`!`) |
| `width`, `height`, `fixedsize` | bounding box ÷ 72 (`fixedsize=true`) |
| `shape` | `box` (rect), `ellipse`, `circle` |
| `style` | `filled`, or `rounded,filled` |
| `fillcolor`, `color`, `fontcolor`, `fontsize` | fill, stroke, the text's fill and size |
| `URL` | the surrounding `a` |
| `image` | the Picture inside it |

A Shape with no text still becomes a box, labelled with an empty string and
named `n1`, `n2`, ….

## Arrow (one per Line joining two different boxes)

- **`from`, `to`:** the boxes at the two ends (research R4), in marker order.
- **`dir=both`:** when there are markers at both ends.
- **`arrowhead=none`:** when there are no markers.

## Report (the command's JSON)

```json
{
  "ok": true, "cell": "s/2", "kind": "hand" | "graphviz",
  "boxes": [{"name": "data", "label": "Data"}],
  "arrows": [{"from": "data", "to": "tool"}],
  "left_out": [{"what": "line", "at": "x=412 y=88", "reason": "its end at x=480 y=88 is near no box"}],
  "notes": ["picture face.png not found beside the deck"],
  "extension": "dot-image"
}
```

`extension` appears only when a picture brought in `dot-image`. `left_out`
and `notes` are empty arrays when there's nothing to say.

## Errors

- **`E_IO`:** the file can't be read.
- **`E_INVALID_ARG`:** it isn't SVG (no `<svg>` root, or bad XML), or it has
  no boxes. Nothing is changed in either case.
