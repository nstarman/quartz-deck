# Data model: Diagrams you edit on the slide (015)

There is no stored state: everything is the cell's DOT text, read into this
model each time.

## Diagram (one `{dot}` cell)

| Field | Meaning |
|---|---|
| `cell` | the block ref (`slide/i`) |
| `kind` | `digraph` or `graph` (edge operator `->` or `--`) |
| `strict` | bool |
| `settings` | graph-level `key=value` and `graph [ … ]`, with spans |
| `defaults` | `node [ … ]` and `edge [ … ]` statements, with spans |
| `nodes` | Box list |
| `edges` | Arrow list |
| `subgraphs` | spans, read-only |
| `editable` | `true`, or `{reason, at}` when the reader stopped |
| `pinned` | `layout` is `neato` or `fdp`, and every node has `pos` ending in `!` |
| `body` | the span between the cell's fences, so patches are relative to the deck |

## Box (node)

| Field | Meaning |
|---|---|
| `name` | the node ID (stable; edges refer to it) |
| `label` | the `label` attribute, else the name; for a spacer label (image), the text cell |
| `look` | the style attributes present: `shape`, `style`, `fillcolor`, `color`, `fontcolor`, `fontname`, `penwidth` |
| `url`, `image` | `URL`, `image` |
| `pos` | `[x, y]` in inches, and whether pinned (`!`) |
| `size` | `width`, `height`, `fixedsize` |
| `span` | the node statement; for a node with no statement of its own (only in edges), none, and setting an attribute adds a statement |
| `attrs` | every attribute: key, value, value span, quoted or not |

## Arrow (edge)

| Field | Meaning |
|---|---|
| `from`, `to` | node names, with ports kept |
| `index` | 1-based among edges with the same `from` and `to`, in source order (`A->B#2`) |
| `statement` | span of the edge statement; `chain` position when part of `a -> b -> c` |
| `attrs` | `dir`, `arrowhead`, `arrowtail`, … |

## Look

A set of the style attribute keys above with values. "Looks in this
diagram" are the distinct sets among its boxes. The two starters:
- **tool:** `shape=box, style="rounded,filled", fillcolor="#3d4a5c", color="#3d4a5c", fontcolor="white"`;
- **artifact:** `shape=box, style="rounded,filled", fillcolor="white", color="#c8ccd2", fontcolor="#1f2933"`.

## Edit (a patch)

`(span, replacement)` pairs relative to the deck, as `ops::Patch`, with
`op` (`dot-move`, `dot-add`, …) and `target` (`cell` or `cell:node`).

## Spacer label (for images)

```
label=<<TABLE BORDER="0" CELLPADDING="0" CELLSPACING="4"><TR><TD WIDTH="W" HEIGHT="H" FIXEDSIZE="TRUE"></TD><TD>TEXT</TD></TR></TABLE>>
```

W = H = the node's height in points minus 8 (36 − 8 = 28 at the default
0.5 in). `rename` replaces TEXT only.

## Setting

`diagrams.live-layout`: boolean, default `false`.
