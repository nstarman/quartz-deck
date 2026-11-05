# quartz-deck for VS Code

Shows the quartz-deck slide editor in a panel beside the `.qmd` you are
editing (spec [006](../specs/006-vscode/spec.md)). Personal use, same licence
as quartz-deck.

- **Open in quartz-deck** (editor title button, or the palette) on a `.qmd`.
- Edits on the slides land in the VS Code buffer as ordinary unsaved edits
  (⌘Z undoes them); the file changes when you save. Typing in VS Code updates
  the slides. Agents' `qd` edits land in the buffer too.
- Needs `qd` and Quarto where the deck lives (local, or the SSH host).

## Build and install

```bash
cd vscode
npx @vscode/vsce package          # → quartz-deck-<version>.vsix
code --install-extension quartz-deck-*.vsix
```

Tests: `node --test vscode/test/*.test.js` (no npm packages).
