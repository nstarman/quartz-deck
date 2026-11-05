# Security check (T035, SC-006, 2026-11-04)

**Goal.** No other web page or link can make the editor's server run code,
change the deck or its environment, or read the session token (FR-015).

## Automated: `tests/onslide.rs` `another_origin_cannot_run_code_or_edit`

A deck with an environment (`qd env init`), the editor open in Chrome. In the
same browser, a page from **another local origin** (`127.0.0.1:<other port>`,
the realistic attacker: any local web page or a page a link opens) tries, for
each of `/api/run` (`{"all":true}`), `/api/env/init`, `/api/env/add`
(`six`), `/api/op` (`replace`) and `/api/shutdown`:

| Attempt | What stops it |
|---|---|
| `fetch` with `Content-Type: application/json` and a guessed `X-QD-Token` | CORS preflight, never granted; Origin check; token |
| `fetch` `mode: 'no-cors'`, `text/plain` body | JSON-only extractor (415); Origin check; token |
| `<form method=post enctype=text/plain>` into a hidden frame | JSON-only extractor; Origin check; token |
| `fetch('/')` to read the page for the token | CORS (opaque); and since #11 the page never holds the token |

**Result: PASS.** The deck file is byte-identical, the environment manifest
unchanged, `qd outputs` is empty, no kernel was started, the server still
answers, and the attacker could not read the editor page.

## Covered elsewhere

| Threat | Defence | Test |
|---|---|---|
| Wrong / missing token, foreign Origin, non-JSON | `authorized()` = loopback Host + loopback Origin + `X-QD-Token` | `tests/serve.rs` `run_cells_over_http` |
| `run` smuggled through the general edit endpoint | `run_op` refuses `run` | same |
| DNS rebinding (a public name resolving to 127.0.0.1) | every route requires a loopback `Host` (#10) | `tests/serve.rs` (`GET /api/source` with a foreign Host → 403) |
| Another local user / sandboxed process reading `GET /` for the token | token only in the editor link's fragment, kept in `sessionStorage` (#11) | `run_cells_over_http` (page has no token), `run_cells_in_the_editor` (URL and page clean, reload still runs) |
| Token sent elsewhere by `qd run` / `qd stop` | posts only to `127.0.0.1:<port>`, redirects off (#10) | review |
| `qd stop` signalling a reused pid | token-guarded `/api/shutdown`, no `kill` (#11) | `run_cells_over_http` |
| Session file readable by other users | created `0600` | `run_cells_over_http` |

**Note.** Even without qd's own checks, a browser would block most of these
cross-origin attempts (CORS, JSON content type); qd's Host/Origin/token checks
matter for DNS rebinding, non-browser clients on the machine, and defence in
depth. The automated test asserts the end result either way.
