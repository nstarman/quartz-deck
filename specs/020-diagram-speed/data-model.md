# Data model: Diagrams that keep up with the hand

Nothing is stored in the deck. One setting's default changes, and the
editor keeps a little page state.

## Setting

| Key | Kind | Default | Change |
|---|---|---|---|
| `diagrams.live-layout` | bool | `true` | was `false` (research R6) |

## Editor state

- **shown**: what the preview shows.
  - `version`: the version of the render loaded in the iframe.
  - `source`: the deck's text at that version, from `/api/source`. If
    the source has already moved on by the time it's fetched (its
    `version` differs), it's left unknown, and the next render reloads.
  - `drawn`: a map from diagram cell ref to the cell text the editor drew
    since that render. It is emptied on every reload.
- **Rule on a finished render** (research R4): skip the reload when the new
  source, with every `{dot}` body taken out, equals `shown.source` with
  the same bodies taken out, and each diagram body in the new source is in
  `drawn` or unchanged. Then `shown.version` becomes the new version and
  `shown.source` the new source. Otherwise reload.

## Instant drawing of a cell

**Input**: the cell's `qd dot show` result (`text`, `settings.layout`, and
each node's `image` and spacer).

**Output**, into the cell's existing `<svg>`:
- Graphviz's children and `viewBox`;
- the root's `width`, `height` and `style` kept from Quarto;
- `dot-image` pictures (R3);
- the editor's clickable arrow copies, highlight and handles.
