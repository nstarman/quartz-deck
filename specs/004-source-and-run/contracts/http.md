# HTTP contract additions (spec 004)

Existing rules stay: loopback only, Host/Origin checks, JSON only.

## Session token (FR-015)

`qd serve` generates a random token per session; it is embedded in the editor
page and written to the session file (user-only permissions) for the CLI.
Every run endpoint requires `X-QD-Token: <token>`; a missing or wrong token
→ 403. Run endpoints are not reachable through `/api/op` (`run_op` refuses
`run`).

## Endpoints

| Method + path | Body | Result |
|---|---|---|
| `POST /api/run` | `{"cells":[ref…] \| "all":true, "fresh":bool, "timeout":secs?, "wait":bool}` | `202 {"run":id}`; progress via SSE. With `wait` (`qd run`): `200 {"run","status":"done|error|stopped|failed","error"?,"runs":[as qd run]}` when the run ends |
| `POST /api/run/interrupt` | `{}` | `200` |
| `POST /api/run/restart` | `{}` | `200` (session restarts) |
| `GET /api/outputs` | — | `{"cells":{ref:{"stale","counter","items":[…]}}, "order":[{"cell","lines","runnable"}…]}` (image items carry `/outputs/<file>` URLs; `order` lists every code cell) |
| `POST /api/env/init` | `{}` + token | as `qd env init` (uv) |
| `GET /outputs/<file>` | — | cached figure files |
| `GET /api/env` | — | as `qd env` |
| `POST /api/env/add` | `{"packages":[…]}` + token | as `qd env add` |
| `POST /api/op` `replace` | `{"old","new"}` | as `qd replace` (normal op rules) |

## Events (SSE, `/events`)

`kernel {"state":"starting|idle|busy|stopped"}`,
`run_started {"run","cells"}`, `cell_output {"run","cell","counter","items"}`,
`run_done {"run","status"}`. The preview re-renders after `run_done` so slides
show the new outputs (research R7).

Every route answers only to a loopback `Host` (`127.0.0.1` / `localhost`); others get 403 (DNS rebinding).
