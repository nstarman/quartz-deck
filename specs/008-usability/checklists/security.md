# Security check (T022, 2026-11-06)

**Goal.** Only the person adds files beside their deck: by dropping an image on
the slide in the editor, or with `qd image add` on this machine. No other web
page can write a file, and what is written is only an image, under `images/`,
never over an existing file.

## Automated

| Threat | Defence | Test |
|---|---|---|
| Another local web page posts to `POST /api/images` (JSON with a guessed token, `no-cors` text, a form post) | `authorized()`: loopback `Host` + loopback `Origin` + `X-QD-Token`; the token header can't be sent cross-origin without a CORS preflight, which is never granted | `tests/onslide.rs` `another_origin_cannot_run_code_or_edit`: no `images/` folder afterwards |
| A request without the token, or with another `Origin` | `authorized()` (403) | `tests/serve.rs` `images_over_http` |
| A name with folders (`../../x.png`) or a hidden name | only the base name is kept, reduced to `[A-Za-z0-9._-]`, leading dots dropped; the file always goes in `images/` | `src/images.rs` `names_are_cleaned` |
| A file that isn't an image (an `.exe`, or a `.png` holding something else) | the extension must be png, jpg, jpeg, gif, svg or webp, and the first bytes must match it (`E_INVALID_ARG`, 400) | `refuses_wrong_types_bytes_and_size`; `images_over_http` |
| A huge upload | the route takes at most 20 MB + 1 byte (larger: 413); `images::add` refuses over 20 MB (400) | `refuses_wrong_types_bytes_and_size` |
| Overwriting a file | `create_new`: a taken name moves on to `x-1.png`, `x-2.png`, … | `free_names_and_no_overwrite`; `images_over_http` (the first file's bytes are unchanged) |

Checked by mutation: with the token check removed from `POST /api/images`,
`images_over_http` fails at its no-token request (it got 200). In that run the
browser attack page did not land a file either; the reason wasn't established,
so the HTTP test is the one that holds this defence.

**Result: PASS.**

## By design

- `GET /api/images` has no token, like the other read endpoints; a
  cross-origin page can't read the response (no CORS).
- **SVG can hold script** (research R9). The editor shows images only through
  `<img>`, where scripts don't run. But `/preview/` serves the deck's files on
  the editor's origin, so an SVG opened there in the editor's tab (or embedded
  by the deck with `<object>`/`<iframe>`) could run script on that origin and
  read the token from the tab's session storage. That is no more than the
  deck's own rendered HTML can already do (it runs in the same-origin preview
  frame): files beside the deck are trusted as the deck is. What this feature
  adds is only that a dropped SVG lands there by the person's own drop or
  `qd image add`; nothing from another site can put one there (above).
- The undo journal covers the inserted `![](images/…)` line; the copied file
  stays (removing it is the person's choice).
