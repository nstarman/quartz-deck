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
| `POST /api/op` | body `{"op": "<cli command>", "args": {…}, "expect": "<version>"}` → same result/error JSON as the CLI |
| `GET /events` | SSE stream: `changed {version}`, `rendering`, `rendered {version, html}`, `render_error {message}`; on connect the current `rendered`/`render_error` state is replayed |

`op` names and `args` are exactly the CLI commands/flags (e.g.
`{"op": "box move", "args": {"box": "#b-1a2b", "at": [200, 100]}}`), so the
GUI exposes nothing the CLI lacks (FR-010).

Preview DOM contract: each slide `<section id="<anchor>">` (slide id, or
`qd-N` for rule/implicit slides); each
top-level block carries `data-qd="<slide>/<i>"` (or its own `id`);
boxes are `.absolute` elements with inline `top/left/width/height` px.
