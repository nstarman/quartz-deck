# Security notes: saved layouts (009 US3)

`POST /api/layouts` is the one 009 route that writes outside the `.qmd`.

- **Who can call it:** the same origin with the session token (`authorized()`), like
  `/api/images`. Another origin gets 403; `another_origin_cannot_run_code_or_edit`
  sends it a save from a hostile page and checks no `_layouts/` folder appears.
- **Where it writes:** only `_layouts/<name>.md` beside the deck or
  `layouts/<name>.md` in qd's config folder (`where` is `deck` or `mine`, nothing else).
- **Names:** `[a-z0-9-]{1,40}`, checked before any path is built, so no `/`, `..`
  or dot files.
- **Overwrites:** a taken name is 409 unless `replace` is sent; rename never
  overwrites. Built-in layouts can't be removed or renamed.
- **Content:** a saved layout holds no text from the slide: only an empty
  heading, placeholder boxes with px numbers, and an empty notes block.
- **Mutation check:** dropping the `authorized()` call makes `layouts_over_http`
  (403 without the token) fail.
