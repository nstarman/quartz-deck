# HTTP Contract: `qd serve`

Bound to `127.0.0.1` only. Used by the editor UI; agents should use the CLI
(the browser is for looking).

| Method & path | Purpose |
|---|---|
| `GET /` | editor shell (slide list, preview iframe, properties panel) |
| `GET /preview/…` | latest good Quarto render of the preview copy + assets |
| `GET /present/` | redirect to the preview HTML (deck alone, no editor chrome) |
| `GET /api/deck` | same JSON as `qd slides` |
| `GET /api/slide/{ref}` | same JSON as `qd show` |
| `POST /api/op` | body `{"op": "<cli command>", "args": {…}, "expect": "<version>"}` → same result/error JSON as the CLI (`400` bad args, `404` not found, `409` conflict/no undo, `422` unsafe edit) |
| `GET /events` | SSE stream: `changed {version}`, `rendering`, `rendered {version, html}`, `render_error {message}`; on connect the current `rendered`/`render_error` state is replayed |

`op` is a CLI command (`"title"`, `"meta set"`, …); `args` keys are its
positional names and long flags (arrays repeat a flag, `true` sets a switch),
e.g. `{"op": "set", "args": {"slide": "intro", "attr": ["transition=fade"], "class": ["center"]}}`.
The server rebuilds the command line and runs it through the CLI's own parser,
so the GUI exposes nothing the CLI lacks (FR-010). Allowed ops: read ops,
edits, `undo`/`redo`; not `serve`/`stop`/`new`/`skill`; no `file` or stdin
input.

Security: `/api/op` requires `Content-Type: application/json` (cross-site
pages cannot send it without a CORS preflight, which is never granted) and a
loopback `Host` (and `Origin`, if present) — `403` otherwise, which blocks DNS
rebinding. Editor assets are served `Cache-Control: no-cache`.

Preview DOM contract: each slide `<section id="<anchor>">` (slide id, or
`qd-N` for rule/implicit slides); each
top-level block carries `data-qd="<slide>/<i>"` (or its own `id`);
boxes are `.absolute` elements with inline `top/left/width/height` px.
