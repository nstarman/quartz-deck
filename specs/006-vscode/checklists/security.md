# Security check (T020, SC-006, 2026-11-05)

**Goal.** Only the person's own VS Code extension can attach a buffer, type
into it or detach it. No other web page or local program can make qd read a
planted text as the deck, and no page can steer the extension (FR-017).

## Automated

| Threat | Defence | Test |
|---|---|---|
| Another local web page posts to `/api/buffer/open`, `/api/buffer` or `/api/buffer/close` | JSON only (a cross-origin page can send only `text/plain` without a CORS preflight, and CORS is never granted), then `authorized()`: loopback `Host` + loopback `Origin` + `X-QD-Token` | `tests/onslide.rs` `another_origin_cannot_run_code_or_edit` (spec 004's attack page, extended): `qd slides` never shows the planted text |
| A request without the token, or with a foreign Origin | `authorized()` | `tests/serve.rs` `buffer_overlay_over_http` (403 on all three paths) |
| A page framed by something other than the panel sends `theme`/`follow` to the editor | the editor takes messages only from `window.parent`, and only `theme` and `follow` with checked fields | `framed_editor_takes_theme_and_messages` (a child frame's message is ignored) |
| The editor (or a page inside it) sends the extension a message | the webview relays only from its own iframe; the extension accepts only `focusText` and `reveal {lines: [int, int]}` | `vscode/test/panel.test.js` (`fromEditor`) |
| The webview loads anything else | CSP `default-src 'none'; frame-src <qd origin>; script-src 'nonce-…'` | `vscode/test/panel.test.js` |
| A stale or crossed edit overwrites typing | `POST /api/buffer` is a compare-and-swap on the version (409 with qd's text) | `buffer_overlay_over_http`, `vscode/test/sync.test.js` |

Checked by mutation: with the token check removed from `/api/buffer/open`,
the attack page still fails (JSON-only stops it first). With it removed, the
no-token request in `buffer_overlay_over_http` is what catches it.

**Result: PASS.**

## By design

- The buffer overlay lives in qd's per-user cache. It is removed when the
  panel closes or the server stops.
- The token reaches the extension on `qd serve`'s stdout, a pipe only it
  reads. It goes to the webview inside the editor link's fragment, as in the
  browser.
- Remote-SSH: the extension and qd run on the remote host. The panel reaches
  qd through VS Code's own port forwarding (`asExternalUri`), and qd still
  binds to 127.0.0.1 there.
