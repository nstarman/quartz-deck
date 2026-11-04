# CLI contract additions (spec 003)

All commands keep the common flags (`--deck`, `--human`, `--dry-run`,
`--expect`) and the JSON result shape of spec 001
(`{"ok","op","diff","version","target","assigned_ids"}`).

## `qd block`

```text
qd block add <SLIDE> (--after <EL> | --before <EL> | --start | --end) (--md <MD> | --file <F>)
qd block move <EL> (--before <EL> | --after <EL>)
qd block rm <EL>
qd block dup <EL>
```

- `<EL>`: `<slide>/<index>` or `#id`. Both elements of `move` must be on the
  same slide, else `E_INVALID_ARG`.
- `add`: the markdown must be one or more blocks and must not contain a slide
  heading (`E_INVALID_ARG`). Separating blank lines are inserted as needed.
- `move`/`dup`: the element's source is copied byte-for-byte.
- `rm`: removes the block and one adjoining blank line; boxes may also be
  removed with `qd box rm` (unchanged).
- `target` in the result is the new/moved element's id.

## `qd edit` (extended)

```text
qd edit <EL> (--md <MD> | --file <F>) [--expect-text <HASH>]
```

- Writes only the bytes that differ between the element's source and the new
  markdown (minimal patch).
- `--expect-text`: refuse with `E_CONFLICT` unless the element's current
  source hashes to `HASH`; the error's `details` include the current source
  and hash. Independent of `--expect` (whole file); both may be given.

## `qd show` (extended)

Each block gains `"hash": "<16 hex>"`.

## Errors

| Code | When |
|---|---|
| `E_CONFLICT` | `--expect-text` mismatch, or the element no longer exists |
| `E_INVALID_ARG` | cross-slide move, slide heading in `block add`/`edit` source, unbalanced fence |
| `E_NOT_FOUND` | unknown slide/element (unchanged) |

## HTTP

`/api/op` accepts the new ops (`block add|move|rm|dup`) and the
`expect_text` key for `edit` (mapped to `--expect-text`), under the existing
rules (JSON only, loopback, no paths). `/api/slide/<ref>` blocks include
`hash`.
