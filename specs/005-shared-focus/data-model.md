# Data Model: Shared Focus

Session-only, in the server's memory. Nothing here is written to disk.

## Focus snapshot (sent by an editor tab)

| Field | Type | Rule |
|---|---|---|
| `tab` | string | random id the tab made at load; matches its event stream |
| `version` | string | deck version the refs were computed against |
| `slide` | slide ref or `null` | `null` = title slide / front matter |
| `elements` | element refs | `[]` when nothing is selected; several when a source selection spans blocks; first = where the selection starts |
| `selection` | `{text, where: "slide"\|"source", truncated}` or `null` | `text` ≤ 2,000 chars; `null` when nothing is selected |
| `line` | integer or `null` | 1-based source line of the caret, when the source pane has it |
| `editing` | bool | an on-slide edit is open, or a source keystroke within 3 s |

Validation on receipt (server): `tab` and `version` non-empty; text clamped to
2,000 chars (`truncated` set); at most 50 element refs.

## Server focus state

- `tabs`: open tab ids (added when a tab's event stream connects, removed when
  it ends).
- `latest`: the most recent snapshot whose tab is still open, with the time it
  arrived.

Derived on read (`qd focus`):

- `open` = `tabs` non-empty.
- `age` = seconds since `latest` arrived (`null` if none).
- refs in `latest` that no longer resolve in the current deck are dropped;
  `deck_version` (current) is reported next to the snapshot's `version`.

## Point request (`qd focus --set <ref>`)

| Field | Rule |
|---|---|
| `ref` | slide or element ref; must resolve (`E_NOT_FOUND` otherwise) |

Outcome: `moved` (target tab not editing), `deferred` (target tab editing: it
shows a "Go to …" offer), `no_editor` (no open tab). The target is the tab of
`latest`.

## State transitions (one tab)

```text
loaded ──stream connects──▶ open ──reports──▶ open + latest
open ──stream ends──▶ gone (its snapshot no longer counts)
```
