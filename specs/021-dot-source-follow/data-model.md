# Data model: Diagram source follows the cursor

Nothing is stored in the deck or on disk. The model is what `qd dot at`
answers, and the little page state the editor keeps.

## Statement at the cursor (`qd dot at` → `at`)

| Field | Meaning |
|---|---|
| `kind` | `node`, `edge`, `cluster`, `setting` or `defaults` |
| `node` | the box's name (`node`) |
| `edge` | the arrow's ref, `from->to#k` as `dot show` and the editor name it (`edge`): the link under the cursor in a chain |
| `edges` | every arrow ref of the statement (`edge`): the whole chain when the cursor is not on one link |
| `cluster` | the subgraph's id (`cluster`) |
| `span` | `[start, end]` byte offsets of the statement in the given text |
| `alone` | DOT text drawing just this element (`node`, `edge`), else absent |

`setting` and `defaults` have no outline and no preview (FR-009).

## Follow state (editor, per page)

| Field | Meaning |
|---|---|
| `cell` | the cell being followed, or null |
| `at` | the last good answer |
| `from` | `caret` or `hover`: a hover wins while it rests, and leaving it gives `at` back to the caret |
| `stale` | the text has changed since `at`, and it doesn't parse: the preview shows as out of date |

## Transitions

- Caret or hover stops in a `{dot}` fence: ask, then outline and preview.
- Caret leaves the fence, Esc, or the pane loses focus: clear both.
- A preview reload: the outline goes with the page. The next follow
  re-applies it.
- A parse stop: keep `at` and set `stale`.
