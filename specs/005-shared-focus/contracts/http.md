# HTTP contract additions (spec 005)

All three endpoints require 004's rules: loopback `Host` and `Origin`, and
`X-QD-Token`. Anything else → 403. Focus lives only in memory.

| Method + path | Body | Result |
|---|---|---|
| `POST /api/focus` | focus snapshot (data-model.md) | `204` |
| `GET /api/focus` | — | as `qd focus` |
| `POST /api/focus/set` | `{"ref"}` | as `qd focus --set` (400/404 for bad refs) |

## Event stream

- `GET /events?tab=<id>`: the `tab` query registers the tab as open for as long
  as the stream lasts (other events unchanged).
- New event `point {"tab", "anchor", "ref"}`: only the tab with that id acts —
  it goes to `anchor` and flashes `ref` (revealing its source lines), or, while
  editing, shows a dismissable "Go to …" offer.
