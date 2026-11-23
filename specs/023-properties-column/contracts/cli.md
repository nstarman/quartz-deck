# Contract: new and changed CLI ops

All of these edit the deck through the store, as one undo step each. Each
supports `--dry-run` and `--expect`, and is in `qd schema` and `/api/op`.

## `qd build set REF [--appear EFFECT|none] [--leave EFFECT|none]`

Sets how element `REF` builds.
- `--appear`: `fade-in` (written as just `.fragment`), `fade-up`, `fade-down`, `fade-left`, `fade-right`, `grow`, `shrink`, `strike`, `highlight-red`, `highlight-green`, `highlight-blue`, `highlight-current-red`, `highlight-current-green`, `highlight-current-blue`; or `none` (not a build: always shown).
- `--leave`: `fade-out`, `semi-fade-out`, or `none`. With `--appear fade-in`, a leave is written as `fade-in-then-out` / `fade-in-then-semi-out`.
- Other classes and attributes on the element are kept.
- An element that only leaves (`--appear none --leave fade-out`) is written as `.fragment .fade-out`.
- Output: `{"ok": true, "target": REF, "appear": …, "leave": …, "diff": …}`.
- `E_INVALID_ARG` for an unknown effect, and `E_NOT_FOUND` for no such element.

## `qd build list SLIDE`

```json
{"ok": true, "slide": "results", "builds": [
  {"ref": "results/1", "name": "Fades up on the next click", "appear": "fade-up", "leave": null, "index": 1},
  {"ref": "results/2", "name": "Picture", "appear": "grow", "leave": "fade-out", "index": 2},
  {"ref": "results/0", "name": "List", "appear": "fade-in", "leave": null, "index": 3, "items": ["First", "Second"]}]}
```

The builds come in presentation order: by `fragment-index` where set,
then source order.

## `qd build order SLIDE REF…`

Puts the slide's builds in the given order. Every build of the slide must
be named once.
- An order equal to source order removes every `fragment-index`.
- Any other order writes `fragment-index=1…n`.
- `E_INVALID_ARG` when a build is missing, repeated, or not a build.

## `qd meta set KEY VALUE` / `qd meta unset KEY` (changed)

`KEY` may be dotted: `format.revealjs.theme`. Missing maps on the path
are created. On unset, maps that become empty are removed.
- A top-level key works as today.
- Setting through a non-map value (e.g. `format: html` as a string) is
  `E_UNSAFE_EDIT`.

## `qd meta size WIDTH HEIGHT [--keep-places]`

Sets `format.revealjs.width` and `.height`. With `--keep-places` (what the
editor uses), every positioned box's and arrow's coordinates on every
slide are scaled with the slide, in the same patch.

## Tests

- `tests/cli.rs`:
  - each appear and leave pair, and back to none;
  - classes kept;
  - `build list` order with and without `fragment-index`;
  - `build order` to source order removes indexes;
  - errors.
- `ops.rs` unit tests:
  - dotted `meta set` and `unset` on a deck with and without `format:`, and with `format: html`;
  - `meta size --keep-places` scaling boxes and arrows.
- `tests/serve.rs` or `onslide.rs`: a slide with `fragment-index` presents its builds in that order (reveal.js's `Reveal.getIndices` through clicks).
