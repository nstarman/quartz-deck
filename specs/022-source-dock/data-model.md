# Data model: Dock the source pane by dragging it

Nothing is stored in the deck. Two settings in the person's settings file
(spec 007):

| Key | Kind | Values | Default | Shown in Settings |
|---|---|---|---|---|
| `source.side` | choice | `far-left`, `left`, `right`, `below` | `left` | yes ("Source pane place") |
| `ui.source_width` | number (px) | existing | none | no |
| `ui.source_height` | number (px) | new | none (CSS default 35% of the window) | no |

- `left` and `right` keep their meaning (FR-006); `right` now puts the
  inspector between the slide and the pane (FR-009).
- An unknown value is refused by `qd settings set` and by the server, as
  today for every choice.
- The window width never writes `source.side` (R6).

## Where the pane is shown

| `source.side` | Window > 700 px | Window ≤ 700 px |
|---|---|---|
| `far-left` | pane, splitter, slide list, (history), slide, (panel) | below the slide |
| `left` | slide list, (history), pane, splitter, slide, (panel) | below the slide |
| `right` | slide list, (history), slide, (panel), splitter, pane | below the slide |
| `below` | slide above, splitter, pane under it, in the stage's column | below the slide |

Full width hides the stage and splitter in every row; the pane takes the
stage's room (and the stage's whole column for `below`).
