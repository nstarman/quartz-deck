# CLI contract additions (009)

## `qd layouts`

```json
{"layouts": [{"name": "two-columns", "description": "A title and two columns of text",
  "from": "built-in|mine|deck", "placeholders": [{"kind": "title|text|image", "purpose": "left column"}],
  "boxes": [{"kind": "text", "left": 60, "top": 170, "width": 450, "height": 470}]}]}
```

`boxes` are in the deck's slide px (scaled), for previews.

## `qd add --layout NAME [--after S | --before S] [--title T]`

Adds one slide from the layout (one patch, one undo step). Output is `add`'s,
plus `"placeholders": [{"ref": "s-ab12/0", "id": "b-cd34", "kind": "text", "purpose": "left column"}]`.
`E_NOT_FOUND` for an unknown layout. `--layout` with `--md`/`--file` is
`E_USAGE`.

## `qd layout save SLIDE NAME [--mine | --deck] [--replace] [--at REF=X,Y,W,H …]`

Saves the slide's structure as a layout (R3). `--deck` (default) writes
`_layouts/NAME.md` beside the deck, `--mine` the config folder. A taken name
is `E_CONFLICT` unless `--replace`. `--at` gives a flow element's place on the
canvas (the editor sends them; without it a flow element becomes a full-width
text placeholder stacked under the title).

## `qd layout rm NAME [--mine | --deck]` and `qd layout rename OLD NEW [--mine | --deck]`

Built-in layouts can't be removed or renamed (`E_INVALID_ARG`).

## `qd block place REF --at X,Y [--size W,H]` and `qd block unplace REF`

R4. `place` refuses (`E_INVALID_ARG`, with the reason) a div, notes, a box.
`unplace` refuses an element that isn't a box, or a box whose attributes hold
more than its id, `.absolute` and position/size (it says which).

## `qd box front|back|forward|backward REF`

R6. One `block_move` patch; `E_INVALID_ARG` on a non-box.

## Placeholders in `qd show`

Blocks carry `"placeholder": "text|image"` while marked. `qd edit` on a
placeholder drops its markers in the same patch.
