# Security check (T019, SC-006, 2026-11-05)

**Goal.** No other web page or local program can read what the person has
selected (it is deck content), plant a fake focus for an agent to act on, or
move the person's editor (FR-009, FR-010).

## Automated

| Threat | Defence | Test |
|---|---|---|
| Another local web page posts a fake focus, points the editor (`/api/focus/set`), or reads `/api/focus` | `authorized()`: loopback `Host` + loopback `Origin` + `X-QD-Token`; JSON only; CORS never granted | `tests/onslide.rs` `another_origin_cannot_run_code_or_edit` (spec 004's attack page, extended): the planted selection never appears in `qd focus`, the editor never moves, the focus is unreadable |
| Missing / wrong token, foreign Origin | same | `tests/serve.rs` `focus_over_http` (403 for each) |
| A focus report from a tab that has no open event stream (a stale or forged tab id) | reports are kept only for connected tabs | `focus_over_http` ("accepted, not kept") |
| A huge selection or ref list | clamped server-side (2,000 chars, 50 refs) | `src/focus.rs` unit test |
| DNS rebinding | every route requires a loopback `Host` (spec 004 #10) | `tests/serve.rs` |

**Result: PASS.**

## By design

- The focus lives only in the server's memory: never written to disk, gone
  when the server stops (FR-010).
- It contains only refs and the selected text; the selected text is the same
  deck content the token already protects.
- A tab opened without the editor link (no token) cannot report a focus;
  `qd focus` then says so (`hint`).
