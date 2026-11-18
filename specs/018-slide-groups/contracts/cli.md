# CLI contract: slide groups (018)

Every command takes the global `--deck`, `--expect VERSION`, `--dry-run`
and `--human`, prints JSON, and is one undo step. `SLIDE` is any slide ref
`qd show` accepts.

| command | does | prints |
|---|---|---|
| `qd indent SLIDE` | makes the slide a sub-slide of the group above (research R3); may also make the slide above a head | `{"ok","slide","changed":[ids],"diff","version"}` |
| `qd outdent SLIDE` | makes a sub-slide a group head; the sub-slides after it become its own | `{"ok","slide","changed":[ids],"diff","version"}` |
| `qd move SLIDE… (--after X \| --before X) [--indent \| --outdent]` | as today; with a flag, the first moved slide then lands indented or outdented, in the same patch (R4) | as today, plus `"changed"` |
| `qd navigation (linear \| vertical \| grid)` | sets the deck's `navigation-mode`; `linear` removes it (R6) | `{"ok","navigation_mode","diff","version"}` |
| `qd show` | as today; each slide gains `head` and `in`, the deck gains `group_level` and `navigation_mode` (data-model.md) | |

`changed` lists every slide whose heading level changed, so a caller can
tell when the slide above also became a head.

A whole group is moved, removed or duplicated by naming the head and its
sub-slides (`qd show` lists them by `in`): `qd move part-a a1 a2 --after
part-b`. There is no separate group command.

## Errors

| code | when |
|---|---|
| `E_INVALID_ARG` | the indent or outdent isn't possible: nothing above, already a sub-slide, already a head, `slide-level: 1`, the slide above has no heading. The message gives the reason the editor shows. |
| `E_UNSAFE_EDIT` | the slide, or the slide above it that would change, is opaque |
| `E_NOT_FOUND` | `SLIDE` doesn't exist |
| `E_CONFLICT` | `--expect` doesn't match the file |

All four new commands are in `API_OPS` (the editor calls them through
`api.op`), `qd schema` and the skill.
