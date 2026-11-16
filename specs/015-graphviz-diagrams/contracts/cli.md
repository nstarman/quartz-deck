# CLI contract: `qd dot` (015)

All commands take the global `--deck`, `--expect`, `--dry-run` and `--human`
flags. Edits print `{"ok","op","version","diff","target",…}` as other edits
do. Errors go to stderr as `{"error":{"code","message","hint"}}`.

`CELL` is a block ref (`slide/i`, `#id`) to a `{dot}` cell. `NODE` is a node
name. `EDGE` is `A->B` or `A--B`, optionally `#k` for the k-th such edge.

| Command | Does | Output beyond the edit fields |
|---|---|---|
| `qd dot show CELL` | reads the diagram | `{"ok","kind":"dot","cell","directed","pinned","editable":true\|{"reason","at"},"text","settings":{…},"nodes":[{"name","label","look":{…},"url","image","pos":[x,y],"pinned","size":{…}}],"edges":[{"from","to","index","attrs":{…}}]}` |
| `qd dot move CELL NODE (--at X,Y \| --by DX,DY) [--no-snap]` | sets `pos="X,Y!"` (inches, snapped to 0.25) | `"pos":[x,y]` |
| `qd dot add CELL [--label L] [--at X,Y] [--look-of NODE \| --look tool\|artifact] [--from NODE] [--to NODE]` | adds a node, optionally connected | `"node":"n3"` |
| `qd dot connect CELL A B` | adds an edge A→B (or A--B) | `"edge":"A->B#k"` |
| `qd dot rename CELL NODE LABEL` | sets the label (only the text cell of a spacer label) | |
| `qd dot set CELL (NODE\|EDGE) KEY=VALUE… [--look-of NODE \| --look NAME]` | sets attributes; `image=PATH` also writes a spacer label, `image=` (empty) removes it | |
| `qd dot rm CELL (NODE\|EDGE)` | removes a node and its edges, or one edge | `"removed":{"nodes":[…],"edges":[…]}` |
| `qd dot flip CELL EDGE` | reverses a directed edge | |
| `qd dot pin CELL [--positions JSON]` | `layout=neato`, `splines=ortho`, every node's `pos` pinned | `"pinned":n` |
| `qd dot new (--after REF \| --on SLIDE)` | inserts a starter diagram cell | `"cell":"slide/i"` |

**`pin` positions**, in order of preference:
1. `--positions {"NODE":[x,y],…}` in inches, as the editor sends;
2. the last preview render in qd's cache;
3. `dot -Tjson` on PATH;
4. otherwise `E_NOT_FOUND`, with a hint to open the deck in `qd serve` once or install Graphviz.

**Errors**

| Code | When |
|---|---|
| `E_NOT_FOUND` | no such cell, node or edge; the cell isn't `{dot}`; no positions for `pin` |
| `E_UNSAFE_EDIT` | the cell isn't readable as DOT, or the target is inside a subgraph; the message names the reason and where |
| `E_INVALID_ARG` | a bad coordinate, attribute key or value; `flip` on an undirected graph; `move` on an unpinned diagram (hint: `qd dot pin`) |
| `E_CONFLICT` | `--expect` is stale |

Every edit can be undone with `qd undo`.
