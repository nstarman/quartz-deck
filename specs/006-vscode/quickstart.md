# Quickstart: validating the VS Code integration

Prerequisites: VS Code, `qd` + Quarto on PATH, `cargo build --release`,
`cd vscode && npx @vscode/vsce package && code --install-extension quartz-deck-*.vsix`.

## 1. Panel (US1, SC-001)

Open a `.qmd`, run **Open in quartz-deck**: slides beside the text within 5 s,
in VS Code's theme; switch theme → panel follows; close panel → `qd focus`
says `{"open": false}`.

## 2. Same deck, no lost changes (US2, SC-002, SC-003)

- Type in the text editor without saving → the panel re-renders (≤ 3 s).
- Edit a title on a slide → the text editor shows it, dirty marker on, file
  on disk unchanged (`git diff` empty) until ⌘S; ⌘Z in VS Code undoes it.
- With unsaved typing in slide 2, edit slide 4 in the panel → both kept.
- From a terminal: `qd edit results/0 --md "…"` → appears in the buffer.
- 15 minutes of mixed edits: nothing lost.

## 3. Cursor ↔ slide (US3, SC-004)

Cursor into slide 3's text → panel shows slide 3; click a list in the panel →
cursor at its source; `qd focus` reports each (`where: "text"` for VS Code).

## 4. Run, render, export (US4)

Without the Quarto extension: Run in the panel. With it: no Run in the panel;
Run Cell from VS Code. Palette: Render / Export / Check offline.

## 5. Remote-SSH (SC-005) and safety (SC-006)

Repeat §1–2 on an SSH host. Security test: another page/process cannot open,
update or read the overlay or the focus without the token.
