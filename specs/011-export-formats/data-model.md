# Data Model: Export Formats (011)

Nothing here is stored. These are the values `formats.rs` works with and
the JSON it prints.

## Format

| Field | Type | Notes |
|---|---|---|
| `name` | string | command-line name: `pptx`, `beamer`, `docx`, `odt`, `html`, `pdf`, `typst`, `gfm`, `commonmark`, `ipynb`, `epub`, `revealjs`, `revealjs-pdf`, or any Pandoc writer |
| `label` | string | menu text: "PowerPoint", "Beamer PDF", "Word", … |
| `group` | `slides` \| `documents` \| `other` | menu group |
| `ext` | string | output extension without the dot (`pptx`, `pdf`, `md`, …) |
| `route` | `quarto` \| `pandoc` \| `offline` | `offline` = the 002/010 export (`revealjs`, `revealjs-pdf`) |
| `needs` | `tex` \| `browser` \| null | tool checked before rendering |
| `ready` | bool | `needs` is null or the tool was found |

Rules: names match `^[a-z0-9_+-]+$` (as `render_deck` already checks). An
unknown name is `E_INVALID_ARG` with "see `qd export --list`".

## Feature (a deck feature that a format may not keep)

`notes`, `pause`, `fragment`, `columns`, `placed`, `background-image`,
`background-media`, `interactive`, `theme`.

How each is found in the deck:

| Feature | Found by |
|---|---|
| notes | a `Notes` block on the slide |
| pause | a `Pause` block |
| fragment | a div or block with class `fragment` |
| columns | a div with class `columns` |
| placed | a `Box` block (`.absolute`) |
| background-image | slide attr `background-image` |
| background-media | slide attr `background-video` or `background-iframe` |
| interactive | a cell on a deck that uses plotly, or a recorded output whose only rich type is `text/html` or a widget |
| theme | `format.revealjs.theme` in the front matter (deck-wide, `slides: []`) |

## Keep (one cell of the carry-over table)

`kept`, `flow` ("in the flow"), `all-shown` ("all steps shown at once"),
`hidden` ("in the file but not shown"), `lost` ("not in the file").
Only `kept` produces no loss.

## Loss

| Field | Type | Notes |
|---|---|---|
| `feature` | Feature | |
| `keep` | Keep | never `kept` |
| `what` | string | plain sentence: "placed boxes are in the flow" |
| `slides` | [ref] | slide refs in deck order (`#id` or `slide/i`); empty for `theme` |

## Export result (stdout of `qd export --to`)

```json
{
  "ok": true,
  "kind": "export-format",
  "format": "pptx",
  "route": "quarto",
  "out": "/abs/export/talk.pptx",
  "files": [],
  "losses": [
    {"feature": "placed", "keep": "flow", "what": "placed boxes are in the flow", "slides": ["#placed"]}
  ],
  "notes": [],
  "changed_during": false
}
```

On failure: `{"ok": false, "error": {"code", "message", "hint", "slide"?}}`
on stderr, with a non-zero exit, as for every qd error. `slide` is set
when Quarto's log names a cell (as `qd render` already reports it).

`--dry-run` returns the same object with `"dry_run": true`, without `out`
being written and without rendering. The editor uses this to show losses
before the export starts.

## Format list (stdout of `qd export --list`)

```json
{"kind": "export-formats", "formats": [
  {"name": "pptx", "label": "PowerPoint", "group": "slides", "ext": "pptx", "route": "quarto", "needs": null, "ready": true},
  {"name": "beamer", "label": "Beamer PDF", "group": "slides", "ext": "pdf", "route": "quarto", "needs": "tex", "ready": false, "hint": "install TeX: quarto install tinytex"}
]}
```
