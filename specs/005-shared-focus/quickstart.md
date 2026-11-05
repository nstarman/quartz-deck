# Quickstart: validating shared focus

Prerequisites: Quarto, Chrome, `cargo build --release`; any deck.

## 1. Reading the focus (US1, SC-001)

```bash
qd serve --open          # open the printed editor link
qd focus                 # {"open": true, "slide": ..., "elements": [], ...}
```

- Click a paragraph on a slide → `qd focus` shows that slide and element.
- Select two words while typing in it → `selection.text` is those words,
  `where: "slide"`.
- Open **Source**, select a few lines spanning two blocks → both elements,
  `where: "source"`, `line` is the caret's line.
- Edit the deck with `qd title …` → `version` ≠ `deck_version`; a deleted
  element's ref is dropped.
- `qd stop` → `qd focus` says `{"open": false}`.

## 2. Pointing the editor (US2, SC-002, SC-003)

```bash
qd focus --set results/1 # "moved": the editor shows Results, element 1 flashes
```

- Start typing in a paragraph, then run `qd focus --set intro` from a terminal
  → `"deferred"`; nothing moves, no character is lost; a "Go to Intro" offer
  appears; clicking it goes there.
- `qd focus --set nope` → `E_NOT_FOUND`, the editor does not move.

## 3. An agent and "this" (SC-004)

Select a sentence, then ask an agent "make this shorter": it runs `qd focus`
and edits exactly that element.

## 4. Safety (SC-005, SC-006)

100 × `qd focus` → deck file and editor view unchanged. From another local
page, `fetch('http://127.0.0.1:<port>/api/focus')` → refused (see 004's
security test, extended).
