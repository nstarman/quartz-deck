# CLI contract additions (spec 005)

Common flags and JSON result shape as in specs 001–004.

## `qd focus`

```text
qd focus                 # what the person has in focus in the editor
qd focus --set <REF>     # point the editor at a slide or element
```

Read result:

```json
{"open": true, "age": 2.4, "version": "<snapshot's deck version>", "deck_version": "<current>",
 "slide": "results", "elements": ["results/1"],
 "selection": {"text": "three things", "where": "slide", "truncated": false},
 "line": 18, "editing": false}
```

- No `qd serve` running, or no editor tab connected: `{"open": false}`, exit 0.
- Open but nothing reported yet (e.g. a tab opened without the editor link):
  `{"open": true, "age": null, "hint": "open the editor with the link from qd serve / qd url"}`.
- `slide: null` = the title slide / front matter. `elements: []` and
  `selection: null` when nothing is selected.
- Refs are the ones every other command accepts; refs that no longer resolve
  in the current deck are dropped. `version` ≠ `deck_version` means the deck
  changed since the focus was taken.

Set result: `{"ok": true, "result": "moved" | "deferred" | "no_editor", "ref": "<REF>"}`.
`deferred` means the editor reported the person typing at its last update
(the page checks again when the request arrives and offers "Go there" while
they type); element refs come back positional (`<slide>/<i>`).
`E_NOT_FOUND` / `E_AMBIGUOUS` for a bad ref, as other commands.

Reading never changes anything (deck, editor view).

## Skill

When the person says "this", "here", "the selected …", run `qd focus` first:
act on `elements`/`selection` when `open` is true and `age` is small; ask when
it is closed, empty or old. After a change the person should look at,
`qd focus --set <target>` shows it to them.
