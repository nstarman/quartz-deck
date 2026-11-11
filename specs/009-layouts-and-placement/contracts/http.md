# HTTP contract additions (009)

## `GET /api/layouts`

Same JSON as `qd layouts`. No token (read-only, like `/api/deck`).

## `POST /api/layouts`

Token and same origin (`authorized()`). JSON body:

- `{"save": {"slide": "s-ab12", "name": "quote-left", "where": "deck|mine", "replace": false, "at": {"s-ab12/2": [60, 170, 450, 300]}}}`
- `{"rm": {"name": "…", "where": "deck|mine"}}`
- `{"rename": {"from": "…", "to": "…", "where": "deck|mine"}}`

200 with the updated list; 400 `E_INVALID_ARG` (bad name, built-in); 409
`E_CONFLICT` (name taken, no `replace`); 403 without the token.

Adding a slide from a layout, placing, unplacing, nudging and stacking are
`POST /api/op` (`add` with `layout`, `block place`, `block unplace`, `box
move`, `box front` …) like every other edit.
