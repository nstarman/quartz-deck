# HTTP contract (011 changes)

No new routes. The editor goes through `/api/op` as before.

## `POST /api/op`

- `{"op": "export", "args": {"to": "pptx", "out"?: "…"}, "dry_run"?: true}`
  → the export result (data-model.md).
- `{"op": "export", "args": {"list": true}}` → the format list.
- **Changed:** ops `render`, `export` and `pdf` require the `X-QD-Token`
  header (same origin + token, `authorized()`), because they run code.
  Without it they return `403`, as the other token-checked routes do. Other ops are
  unchanged (same origin).
- **Changed:** the response's `href` is set for an output of any
  extension inside the deck folder (it was only set for html and pdf), so
  the editor can link the file.

## Show the file

If the server has no way to reveal a file yet:
`POST /api/reveal {"path": "…"}`, token required, path inside the deck
folder only. It runs `open -R` on macOS and `xdg-open <folder>` on Linux.
`204` on success.
