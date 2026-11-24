# Contract: nested element refs

## Form

```text
results/1          top-level block 1 of slide `results` (as before)
results/1/0        column 0 of the columns block results/1
results/1/0/2      element 2 of that column
#fig-a, fig-a      an element with {#fig-a}, at any depth
```

## `qd show <slide>`

Each block in `slide.blocks` may carry `"children": [<block>, ...]`, the
same object shape, recursively. Only `.columns` and `.column` divs have it.

```json
{"id": "results/1", "kind": "div", "attrs": {"classes": ["columns"], ...},
 "children": [
   {"id": "results/1/0", "kind": "div", "children": [
     {"id": "results/1/0/0", "kind": "list", "text": "- one\n- two", ...}]},
   {"id": "results/1/1", "kind": "div", "children": [
     {"id": "results/1/1/0", "kind": "image", ...}]}]}
```

## Ops

| Takes a nested ref | Refuses it (`E_INVALID_ARG`) |
|---|---|
| `build set`, `build order` (its element list), `block class`, `edit`, `show`, `focus --set` | `block move`, `block dup`, `block rm`, `block add --before/--after`, `block place`, `block unplace`, `box *`, `restore --element`, `run` |

`qd build list SLIDE` returns builds at every depth, each with its `ref`,
ordered by `fragment-index` and then source order (parent before child).

Refusal message: `'<ref>' is inside columns: <verb> it in the source for now`.
