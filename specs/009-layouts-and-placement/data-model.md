# Data model: Layouts and placement (009)

Nothing here is stored outside the deck except saved layouts (files).

## Layout (a file)

`src/layouts/<name>.md` (built-in), `<config>/layouts/<name>.md` (the person's),
`_layouts/<name>.md` beside the deck (the deck's):

```markdown
---
name: two-columns
description: A title and two columns of text
---

## {#title}

::: {.absolute .qd-ph ph=text left=60 top=170 width=450 height=470}
<!-- left column -->
:::

::: {.absolute .qd-ph ph=text left=540 top=170 width=450 height=470}
<!-- right column -->
:::
```

| Field | Rule |
|---|---|
| name | `[a-z0-9-]{1,40}`; unique within its place |
| description | one line |
| slide | one slide's markdown in 1050×700 px; boxes get fresh ids when added |
| placeholder comment | an HTML comment inside an empty box: its purpose, for `qd layouts` and agents; dropped when the slide is added |

## Placeholder (in the deck)

An element whose attributes carry `.qd-ph` and `ph=title|text|image` and
whose content is empty (or the untitled heading for the title). Filling it
drops both markers in the same patch. Never rendered with content.

## Placed element (in the deck)

A `BlockKind::Box`: a div `::: {#b-… .absolute left top width [height]}`
around one block, or an image with those attributes. Position and size in px
of the deck's slide. `unplace` applies only when the attributes are exactly
these (plus the id).

## Settings (007's `DEFS`)

| Key | Values | Default |
|---|---|---|
| `placement.snap` | bool | true |
| `ui.last_layout` | layout name | `title-content` |
