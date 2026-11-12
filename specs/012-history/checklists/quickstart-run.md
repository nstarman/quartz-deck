# Quickstart run: 012 history (T017)

**Date**: 2026-11-12
**Build**: branch `012-polish` (debug), `qd serve` in the Claude browser pane
**Setup**: a scratch repository with `talks/deck.qmd` and four commits: `first`,
`qd: edit one (person)`, a `git mv` rename away and back, and a commit by the
person. Signing was off in that repository. qd's config and cache were in
scratch folders (`QD_CONFIG_DIR`, `QD_CACHE_DIR`), so the real settings
weren't touched.

## §1 Look back — pass

- [x] `qd history` lists the four commits newest first, with `uncommitted`
  first while the file had an unsaved change. Only the `qd:` commit has
  `qd: true`. The rename commit has its old path, `talks/old.qmd`.
- [x] `--slide one` lists only the two commits that changed slide one.
- [x] View ▸ History lists the same versions, with the `qd` badge.
  Choosing `first` renders it beside the current deck.

## §2 Save as git commit — pass (one fix to this guide)

- [x] The guide said `qd settings set history.commit true history.pause 5`,
  but `qd settings set` takes one key at a time. The guide now has two
  commands.
- [x] With `qd serve` running, three `qd title` edits within a second gave one
  commit, `qd: title one ×3 (agent)`, about 5 s later. It contained
  `talks/deck.qmd` only, and `other.txt` stayed staged (`M  other.txt`).
- [x] The History panel's top line counted down ("Saving a version in 4 s").
- [x] With no server, `qd title three Drei` committed at once, and its JSON had
  `"commit": {"rev": "3988f54…"}`.
- [x] During a conflicted `git merge`, `qd title two Zwei` succeeded with
  `"commit": {"error": {"code": "E_REPO_BUSY", …}}`, and no commit was made.

## §3 Compare — pass (one fix)

- [x] Compare with `first` listed "changed · One again" and "changed · Two".
  Opening the first showed its diff, with the removed and added lines in red
  and green, and both previews moved to slide One.
- [x] Fixed: each slide's diff ended in "\ No newline at end of file",
  because slides are compared trimmed. The trimmed text now ends in a
  newline, and the unit test checks for it.

## §4 Restore — pass

- [x] `qd restore 6af5d5a --slide one` changed only that slide, with op
  `restore slide one from 6af5d5a`.
- [x] `qd undo` gave back the file byte for byte (`cmp`).
- [x] With the setting on and the server running, the restore was committed as
  `qd: restore slide one from ed8d994 (agent)`. `git rev-list` went from 5
  commits to 6 and still had all the old ones.

## §5 Outside a repository — pass

- [x] `qd history` on a copy outside git gives `E_NO_REPO` with the hint
  "start a history with `qd history --init`". The editor's **Start a history
  here** is covered by the browser test `history_panel`. It wasn't clicked
  here, so no repository was created in a real folder.

## Seen in passing (not 012)

- While rendering, Quarto writes `talks/.gitignore` (`/.quarto/`) and a
  transient `deck.qd-preview.qmd` beside the deck. Both predate 012, and qd's
  commits never include them.
