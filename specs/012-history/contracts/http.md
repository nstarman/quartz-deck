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

`history` (Save as git commit): `{"state": "pending", "pause"}` on each change (the panel counts down from `pause` seconds),
then after the pause `{"state": "committed", "rev"}`, `{"state":
"unchanged"}` (the deck equals HEAD), `{"state": "failed", "reason"}` (git's
message: a hook, signing), or `{"state": "skipped", "reason"}` (no
repository, or a merge, rebase or detached head). A failed or busy commit
stays pending and is tried again at the next pause and when the server stops.
