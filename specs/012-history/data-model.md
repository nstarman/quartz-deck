# Data Model: History (012)

Nothing new is stored, apart from rendered old versions in qd's cache and a
`by` field on undo journal entries. git holds the history.

## Version

| Field | Type | Notes |
|---|---|---|
| `rev` | string \| null | full commit hash; `null` for the uncommitted entry |
| `short` | string | 7-character hash, or `"uncommitted"` |
| `time` | integer | commit time, Unix seconds (the file's mtime for uncommitted) |
| `author` | string | commit author name |
| `message` | string | subject line |
| `qd` | bool | subject starts with `qd:` |
| `path` | string | the deck's path at that commit, relative to the repository top (renames) |

## SlideChange

| Field | Type | Notes |
|---|---|---|
| `kind` | `added` \| `removed` \| `moved` \| `changed` | `moved` means same text, new position |
| `id` | string | the slide's id in the newer version (older for `removed`) |
| `title` | string \| null | |
| `from` / `to` | integer \| null | 1-based positions in the old and new version |
| `diff` | string | unified line diff of the slide's text (`changed` only) |

Matching (research R7): explicit id, then a title unique in both, then
position.

## Pending commit (server, in memory)

| Field | Notes |
|---|---|
| `due` | the time of the last change plus `history.pause` |
| `state` | `idle`, `pending`, `committed <rev>`, `failed <reason>`, `skipped <reason>` (busy repository, no repository) |

## Settings (007)

| Key | Kind | Default | Notes |
|---|---|---|---|
| `history.commit` | Bool | `false` | Save as git commit |
| `history.pause` | Number | `30` | seconds of quiet before committing; 5–600 |

## Journal entry (store.rs), extended

`by`: `"person"` (an edit through the server's `/api/op`) or `"agent"` (a
CLI process). Old entries without it read as `"agent"`.

## Commit message

```text
qd: <op> [×n], <op> … (<person|agent|person, agent>)
```

Ops in order of first appearance, with repeats folded. A pause with no
journal entries (typing in the source pane) gives `qd: edit source (person)`.
Subjects longer than 72 characters end in `, …`.
