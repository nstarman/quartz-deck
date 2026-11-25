# Data model: Arrange elements inside columns

No new stored data. Spec 024's `Block.children` is the container tree.

| Term | Meaning |
|---|---|
| list | the blocks an element sits among: `slide.blocks`, or its parent's `children` |
| parent | the columns or column div holding an element; none at the top level |
| target | an element (`--before/--after`) or a column (`--into`) on the same slide |

Validation: target on another slide → `E_INVALID_ARG` (as today); target inside the moving element → `E_INVALID_ARG`; `--into` a block that is not a columns/column div → `E_INVALID_ARG`.
