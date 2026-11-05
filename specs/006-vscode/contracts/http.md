# HTTP contract additions (spec 006)

All endpoints: 004's `authorized()` (loopback Host/Origin + `X-QD-Token`).

| Method + path | Body | Result |
|---|---|---|
| `POST /api/buffer/open` | `{"text"}` | `200 {"version"}`; overlay created from the buffer |
| `POST /api/buffer` | `{"base", "text"}` | `200 {"version"}`, or `409 {"version", "text"}` if the overlay moved |
| `POST /api/buffer/close` | `{}` | `204`; overlay removed |
| `POST /api/focus` | as 005, `where` may be `"text"` | as 005 |

Event stream:

- `buffer_edit {base, version, edits}`: after any overlay change not made by
  `POST /api/buffer` (panel ops, CLI edits, undo/redo).

While attached, every existing endpoint (`/api/source`, `/api/op`, …) works
on the overlay.
