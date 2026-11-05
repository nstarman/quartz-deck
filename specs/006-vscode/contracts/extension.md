# VS Code extension contract (spec 006)

Package: `quartz-deck` (`vscode/`), personal-use `.vsix`, same licence;
`extensionKind: ["workspace"]`; activates on `.qmd`.

## Commands (palette; editor-title button on `.qmd`)

| Command | Does |
|---|---|
| `quartz-deck.open` — "Open in quartz-deck" | open (or reveal) the panel for the active `.qmd` |
| `quartz-deck.focusPanel` / `quartz-deck.focusText` | move keyboard focus between panel and text (default keys ⌃⌥Q / Esc in panel) |
| `quartz-deck.render` | `qd render`; result shown |
| `quartz-deck.export` / `quartz-deck.exportPdf` | `qd export [--pdf]`; result shown |
| `quartz-deck.checkOffline` | `qd check --offline`; problems shown |

## Settings

- `quartz-deck.qdPath` (default `qd` on PATH).

## Behaviour

- Missing `qd`/Quarto → an error message naming the install step; no panel.
- One panel per deck per window; closing it runs `qd stop` (overlay removed).
- Panels are restored after a restart.
- With `quarto.quarto` installed, the panel's Run UI is hidden (Run Cell
  comes from the Quarto extension).
