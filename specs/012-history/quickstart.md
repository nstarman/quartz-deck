# Quickstart: History (012)

A temp repository with a deck in a subfolder, `commit.gpgsign false` and a
user set locally (research R10).

## 1. Look back (US1)

```bash
qd --deck talks/deck.qmd history
```

Expected: every commit that changed the deck, newest first, following a
rename. `qd: …` commits have `"qd": true`. An uncommitted change appears
first as `uncommitted`. `--slide one` lists only the commits that changed
that slide.

In the editor, View ▸ History lists the same entries. Selecting one shows
that version beside the current deck, with changed slides marked. A deck
with code shows its cells' code, and none of it runs.

## 2. Save as git commit (US2)

```bash
qd settings set history.commit true
qd settings set history.pause 5
```

With `qd serve` running, make three edits within a few seconds and wait
5 s: one commit, `qd: … (person)` or `(agent)`, containing only the deck. A
file staged beforehand is still staged afterwards.

Without a server, `qd title one "New"` commits at once, and its JSON has
`"commit": {"rev": …}`.

During `git merge` with conflicts, no commit is made, and the editor says
why once.

## 3. Compare (US3)

```bash
qd diff HEAD~2
```

Expected: only the changed slides, each with `added`, `removed`, `moved` or
`changed`. In the editor, Compare shows each changed slide's source diff
and both renders.

## 4. Restore (US4)

```bash
qd restore HEAD~2 --slide one
```

Expected: only that slide changes, and `qd undo` takes it back. With the
setting on, a new commit appears and `git log` loses none.

## 5. Outside a repository

`qd history` in a folder outside git fails with `E_NO_REPO` and the hint
`qd history --init`. In the editor, History offers **Start a history
here**.
