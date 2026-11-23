# Data model: One properties column

## In the deck (written by CLI ops)

| Thing | Where it lives in the `.qmd` | Op |
|---|---|---|
| Slide transition | slide attrs `transition=`, `transition-speed=` | `qd set SLIDE --attr` / `--unset` |
| Morph into the next slide | slide attr `auto-animate=true` | `qd set` |
| An element's appear effect | classes `.fragment` + one effect class (none for fade-in) | `qd build set REF --appear E` |
| An element's leave | `fade-in-then-out` / `fade-in-then-semi-out` (with appear) or `fade-out` / `semi-fade-out` | `qd build set REF --leave E` |
| A list's items one by one | class `.incremental` on the list | `qd block class` (as today) |
| Build order | attr `fragment-index=N` on each build of the slide, or none for source order | `qd build order SLIDE REF…` |
| Deck theme, size, slide numbers, default transition, navigation | `format: revealjs:` keys | `qd meta set format.revealjs.KEY V`, `qd meta size W H`, `qd navigation` |

## Build (`qd build list` → `builds[]`)

| Field | Meaning |
|---|---|
| `ref` | the element (`slide/i`) |
| `name` | its first words, or its kind ("Picture", "Diagram") |
| `appear` | effect name, or `fade-in` |
| `leave` | `null`, `fade-out` or `semi-fade-out` |
| `index` | its place in the slide's order, from 1 |
| `items` | for an incremental list, its items' first words |

## In the editor (settings)

| Setting | Values | Default |
|---|---|---|
| `properties.side` | `right`, `far-left`, `left`, `below` | `right` |
| `ui.inspector` | shown or hidden (kept from spec 019) | `true` |
| `ui.properties_width` / `ui.properties_height` | px | the column's default width (320); below: about a third of the window |

## View state (page only)

- `view`: `format`, `animate` or `deck`. Set by the person, and kept while
  the selection changes.
- `formatTab`: `look`, `text` or `arrange`, kept per element kind as spec
  019 keeps `elementTab`.
