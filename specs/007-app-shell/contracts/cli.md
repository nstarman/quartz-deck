# CLI contract additions (007)

| Command | Output (JSON) | Errors |
|---|---|---|
| `qd actions` | `[{id, label, menu, keys: {mac, other, vscode}, where, toggle, runs_code, qd}]` | — |
| `qd settings` | `{values, defs, path}` | — |
| `qd settings set KEY VALUE` | `{ok, values}` (VALUE parsed as JSON if it parses, else string) | `E_INVALID_SETTING` (unknown key or bad value, with allowed values in the hint) |
| `qd settings reset` | `{ok, values}` | — |
| `qd paste [--after S \| --before S] (--md TEXT \| --md - \| --file PATH)` | the usual op result (`diff`, `target`, `assigned_ids`) | `E_PARSE` if the text has no slide |

`qd schema` and the agent skill list the new commands.
