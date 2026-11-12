# HTTP contract: history (012)

Reads are same-origin, like `/api/deck`. Anything that writes outside the
deck file or runs Quarto needs the session token as well.

| Route | Auth | Body / query | Reply |
|---|---|---|---|
| `GET /api/history` | same-origin | `?slide=&limit=&skip=` | `qd history` JSON |
| `GET /api/history/diff` | same-origin | `?from=REV&to=REV` (`to` empty means the current file) | `qd diff` JSON |
| `POST /api/history/render` | token | `{"rev"}` | `{"href": "/history/<rev>/<deck>.html"}` after rendering (cached) |
| `GET /history/<rev>/…` | same-origin | | the rendered old version's files, read-only from the cache |
| `POST /api/history/init` | token | `{}` | `qd history --init` JSON |
| `POST /api/op` `restore` | same-origin, like every edit | `{"op":"restore","args":{"rev","slide"?,"element"?},"expect"}` | the edit result |

## Events (SSE `/events`)

`history`: `{"state": "pending", "due": <secs>}`, `{"state": "committed",
"rev", "message"}`, `{"state": "failed", "error"}` (sent once per reason),
or `{"state": "skipped", "reason"}`.
