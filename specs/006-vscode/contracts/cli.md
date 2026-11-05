# CLI contract additions (spec 006)

- While a VS Code panel is attached (`session.json` `"buffer": true`), every
  `qd` command reads and edits the VS Code buffer (the overlay), not the file
  on disk; the change shows in VS Code as an unsaved, undoable edit. Results
  are unchanged.
- `qd focus`: `selection.where` may be `"text"` (the VS Code text editor).
