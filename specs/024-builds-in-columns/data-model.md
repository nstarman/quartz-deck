# Data model: Builds inside columns

## Block (changed)

| Field | Change |
|---|---|
| `id` | A child's auto id is `<parent id>/<i>`; explicit `{#id}` as before |
| `children: Vec<Block>` | New. Non-empty only for a `.columns` / `.column` div; spans are absolute byte offsets in the deck text, inside the parent's span |

`Slide.blocks` and `Deck::blocks()` stay top-level only, so cells,
outputs, export and the kernel see what they saw before.

## Ref

`SLIDE` `/` `N` (`/` `M`)* — or `#id` / `id`. See [contracts/refs.md](contracts/refs.md).

## Validation

- A path index past the end at any level, or into a block with no
  children: `E_NOT_FOUND`.
- A nested ref given to an op that rearranges the slide: `E_INVALID_ARG`.
- An explicit id found twice at any depth: `E_AMBIGUOUS`, as today.
