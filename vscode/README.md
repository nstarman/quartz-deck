# quartz-deck for VS Code

Shows the quartz-deck slide editor in a panel beside the `.qmd` you are
editing (spec [006](../specs/006-vscode/spec.md)). Personal use, same licence
as quartz-deck.

- **Open in quartz-deck** (editor title button, or the palette) on a `.qmd`.
- Edits on the slides land in the VS Code buffer as ordinary unsaved edits
  (⌘Z undoes them); the file changes when you save. Typing in VS Code updates
  the slides. Agents' `qd` edits land in the buffer too.
- The cursor and the slide follow each other: move the cursor and the panel
  shows that slide; click an element on a slide and the cursor goes to its
  source. ⌃⌥Q moves the keyboard to the panel, Esc in the panel back.
- Palette: **Render**, **Export (offline HTML)**, **Export with PDF**, **Check
  offline** run the matching `qd` command; the result (or what went wrong,
  with **Show log**) is shown. With the panel open, Export and Check use the
  unsaved buffer; Render always renders the saved file, so it asks first.
- With the Quarto extension installed, run cells with its Run Cell; the panel
  hides its own Run.
- The panel's menus are one **☰** button. Keys VS Code uses while the panel
  has focus stay VS Code's: ⌘, (settings), ⌘S, ⌘B (sidebar), ⌘K (chords),
  ⇧⌘X, ⇧⌘N and ⇧⌘D. Those editor actions (Settings, Bold, Link,
  Strikethrough, Slide after, Duplicate slide) are in the ☰ menu instead; **?**
  lists the keys that work in the panel. The list of VS Code's keys is
  `default-keys.json`, checked by `test/keys.test.js`.
- Needs `qd` and Quarto where the deck lives (local, or the SSH host).

## Build and install

```bash
cd vscode
npx @vscode/vsce package          # → quartz-deck-<version>.vsix
code --install-extension quartz-deck-*.vsix
```

Tests: `node --test vscode/test/*.test.js` (no npm packages).
