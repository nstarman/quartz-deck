# Contract: `qd dot at`

A read-only op: it changes nothing and needs no deck. It is in the CLI,
`/api/op` and `qd schema`.

```text
qd dot at --text BODY --at OFFSET
```

- `BODY`: a `{dot}` cell's body (the text between the fences), as the
  person has it, saved or not. `-` reads it from stdin on the CLI. The
  API takes the text itself, never a file.
- `OFFSET`: a byte offset in `BODY`, at the cursor.

## Answers

At a box:

```json
{"ok": true, "at": {"kind": "node", "node": "data", "span": [41, 112],
  "alone": "digraph { node [shape=box]; data [label=Data, shape=box, style=\"rounded,filled\"] }"}}
```

At an arrow (in `a -> b -> c`, the cursor on `b -> c`):

```json
{"ok": true, "at": {"kind": "edge", "edge": "b->c#1", "edges": ["a->b#1", "b->c#1"],
  "span": [130, 142], "alone": "digraph { b [...]; c [...]; b -> c }"}}
```

The cursor elsewhere in the same statement (`a` or a `[...]` list at its
end): `edge` is null and `edges` lists both.

Elsewhere:
- At a cluster: `{"kind": "cluster", "cluster": "cluster_x", "span": [...]}`, with no `alone`.
- At a whole-diagram line (`layout=neato;`, `node [shape=box];`): `{"kind": "setting" | "defaults", "span": [...]}`.
- Outside any statement (blank line, comment, `digraph {`): `{"ok": true, "at": null}`.
- Text that doesn't parse: `{"ok": true, "at": null, "stopped": {"reason": "...", "at": {"line": 3, "column": 9}}}`.
  Line and column are within `BODY`.

`alone` never holds `pos`, `width`/`height` from the graph's own `size`,
`layout`, or `splines`. It holds the cell's other graph settings and the
`node`/`edge` defaults in force at the statement.

## Errors

- `E_INVALID_ARG` when `OFFSET` is past the end of `BODY`, or not on a
  character boundary.

## Tests (`tests/dot.rs`, `dot.rs` unit tests)

- A box, an arrow, each link of a chain, a cluster, a setting, a default,
  a blank line and a comment, each at the start, middle and end of its
  statement.
- Quoted names with spaces, HTML labels, ports.
- Defaults declared after a box don't reach its `alone`.
- Unclosed quote: `stopped`, with line and column.
- `alone` draws with Graphviz (`dot -Tsvg`, skipped without it) and has
  the box's label.
