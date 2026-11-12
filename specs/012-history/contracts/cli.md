# CLI contract: history (012)

```text
qd history [--slide REF | --element REF] [--limit N] [--skip N]
qd history --init
qd diff REV [REV]
qd restore REV [--slide REF | --element REF] [--expect VERSION] [--dry-run]
```

REV is anything git accepts (`a1b2c3d`, `HEAD~2`, a tag, a branch). REF
resolves in the version named by REV.

## Output

- `qd history` → `{"kind": "history", "repo": "<top>", "path": "<deck rel>", "versions": [Version…], "more": bool}`
  (data-model Version; the uncommitted entry first when the deck differs from HEAD).
- `qd history --init` → `{"ok": true, "repo": "<dir>", "rev": "<first commit>"}`.
- `qd diff` → `{"kind": "diff", "from": "<rev>", "to": "<rev>|uncommitted", "slides": [SlideChange…]}`.
- `qd restore` → the usual edit result (`op`, `diff`, `version`), with op
  `restore slide methods from a1b2c3d`. With the setting on and no server,
  it also has `"commit": {"rev"} | {"error"}`.
- Any edit with the setting on and no server running gains `"commit"` the
  same way.

## Errors

| Case | Code | Hint |
|---|---|---|
| git not installed | `E_NO_GIT` | install git |
| deck not in a repository | `E_NO_REPO` | `qd history --init` |
| bad revision, or the deck isn't in it | `E_NOT_FOUND` | `qd history` lists versions |
| slide/element not in that version | `E_NOT_FOUND` | |
| element whose slide is gone now | `E_INVALID_ARG` | restore the slide instead |
| file changed since `--expect` | `E_CONFLICT` | existing |
| `--init` inside a repository | `E_INVALID_ARG` | already in a repository |
| commit refused (CLI, no server) | not an error: the edit succeeded, and `"commit": {"error"}` says why | |

`E_REPO_BUSY` (merge, rebase, detached head) appears only as a commit
`error`. It never fails an edit.
