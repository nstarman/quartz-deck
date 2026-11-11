# HTTP contract additions (008)

## `GET /api/images`

The images beside the deck, newest first.

```json
{"images": [{"path": "images/chart.png", "kind": "png", "size": 18342, "modified": 1793900000}]}
```

No token (read-only, like `/api/deck`); a cross-origin page can't read it (no
CORS).

## `POST /api/images?name=<file name>`

Body: the file's bytes. Headers: `Content-Type: application/octet-stream`,
`X-QD-Token`. Same-origin and token as every write (`authorized()`).

- 200 `{"path": "images/chart-1.png"}`: written under a free name.
- 400 `E_INVALID_ARG`: no or bad name, a type not in the list, bytes that
  don't match the type, or more than 20 MB.
- 403: no token, or another origin.

The editor then inserts it with `POST /api/op` `block add` as usual (one undo
step is the insert; the file stays).
