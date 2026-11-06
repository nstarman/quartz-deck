# Security check (T023, 2026-11-05)

**Goal.** Only the person changes their settings: through the editor's own
Settings window, or `qd settings` on this machine. No other web page and no
other user on the machine can change or read them.

## Automated

| Threat | Defence | Test |
|---|---|---|
| Another local web page posts to `POST /api/settings` (JSON with a guessed token, `no-cors` text, a form post) | JSON only (a cross-origin page can send only `text/plain` without a CORS preflight, and CORS is never granted), then `authorized()`: loopback `Host` + loopback `Origin` + `X-QD-Token` | `tests/onslide.rs` `another_origin_cannot_run_code_or_edit` (spec 004's attack page, extended): `qd settings` shows every value unchanged afterwards |
| A request without the token | `authorized()` | `tests/serve.rs` `settings_over_http` (403) |
| A bad value or an unknown key | checked against the definitions before anything is written (`E_INVALID_SETTING`, 400) | `settings_over_http`; `src/settings.rs` unit tests |
| Another user on the machine reads or edits the file | `settings.json` is written mode 0600, atomically (temp file + rename) | `src/settings.rs` `set_checks_every_value_and_keeps_unknown_keys` (mode check, Unix) |

Checked by mutation: with the token check removed from `POST /api/settings`,
the attack page still fails (JSON-only stops it first). With it removed, the
no-token request in `settings_over_http` is what catches it (it got 200).

**Result: PASS.**

## By design

- Settings hold preferences only (layout, theme, confirmations, hints). None
  of them runs code or widens what the server allows.
- `GET /api/settings` has no token, like the other read endpoints; a
  cross-origin page can't read the response (no CORS), as the attack test
  checks for `/` and `/api/focus`.
- The file lives in qd's per-user config folder (`$QD_CONFIG_DIR`, else
  `~/Library/Application Support/quartz-deck` or XDG config), never in the
  deck or its folder.
