# Data model: Arrows on the slide (016)

Nothing is stored outside the `.qmd` and the deck's `_extensions/`.

## Arrow

One paragraph block whose whole text is one quarto-arrows shortcode, for
example:

```markdown
{{< arrow from="420,310" to="780,160" position="absolute" color="#b8513a" width="3" label="the peak" >}}
```

Identity: the block's ref (`SLIDE/i`), like any element.

| field | option | type, rule | default (not written when equal) |
|---|---|---|---|
| start | `from` | `x,y`, slide px, whole numbers | required |
| end | `to` | `x,y`, slide px, whole numbers | required |
| placed | `position` | must be `absolute` to be edited on the slide | (inline) |
| bend | `control1`, `control2` | `x,y` slide px; written by the middle handle | none |
| hand curve | `curve`, `bend`, `waypoints`, `smooth` | kept and drawn; not written by qd | none |
| colour | `color` | any CSS colour | `black` |
| width | `width` | number, px | `2` |
| dash | `dash` | `true` or `a,b` | none |
| line | `line` | `single`, `dot`, `double`, `triple` | `single` |
| head | `head` | `arrow`, `stealth`, `diamond`, `circle`, `dot`, `square`, `bar`, `barbed`, `none` | `arrow` |
| heads | `head-start`, `head-end` | `true` / `false` | `false` / `true` |
| head fill | `head-fill` | `true` / `false` | `true` |
| size | `size`, `head-size` | number | `10` |
| opacity | `opacity` | 0 to 1 | `1` |
| label | `label` | text | none |
| label place | `label-position` | `start`, `middle`, `end` | `middle` |
| label offset | `label-offset` | number, px | `10` |
| appears on click | `fragment` | `true` (qd writes this) or `draw`, both kept | none |
| click order | `fragment-index` | number; kept if written | none |
| access | `aria-label`, `alt`, `title` | text; kept | none |

Options qd doesn't know are kept, in place.

**Editable** when: the paragraph is exactly one `{{< arrow … >}}`,
`position="absolute"`, and every option parses (`key="…"`, `key='…'`,
`key=bare`). Otherwise read-only, with the reason (`qd arrow show`).

**Lifecycle:** added by `qd arrow add` (or Insert ▸ Arrow) after the slide's
last content block, before its notes; changed by `move` and `set`; removed by
`rm` (or Delete, which is `qd block rm` on the ref). Copy, cut, paste,
duplicate and move to another slide work as for any element.

## Arrows extension

quarto-arrows v0.3.0 (commit `957a61b`): six files, pinned by SHA-256
(research R3).
- **In qd's cache:** `<cache>/lib/quarto-arrows-0.3.0/`, downloaded once.
- **In the deck:** `_extensions/emilhvitfeldt/arrows/`, installed with the
  first arrow; version-aware (an older copy is replaced, an edited copy of
  the same version is kept with a warning).
