# UI contract: Run on the slide (013)

## ▶ on a cell (US1)

- A cell element in the preview (an executable cell's code block) shows a ▶
  at its top right when hovered, when selected, or while it runs.
- The ▶ is a `<button>` in the editor's overlay, not in the slide's HTML. Its
  accessible name is "Run cell REF" ("Stop cell REF" while it runs). It's in
  the tab order while its cell is selected.
- **States**: the cell element gets `qd-running`, `qd-error`, `qd-stopped` or
  `qd-stale`. The ▶ shows a spinner while running and a ■ to stop.
- **Keys**: ⇧Enter runs the selected cell (the `run` action). Esc during a run
  doesn't stop it; ■ or Run ▸ Stop does.
- When a run ends, the cell's `qd-output` block is replaced in place, or
  inserted after the cell if it had none. The slide and selection don't
  change.

## Run in the source pop-up (US2)

- The pop-up on a code cell shows **Run ⇧Enter** in its bar, between the save
  hint and Cancel.
- ⇧Enter or Run saves the edit (if the text changed), then runs the cell. The
  pop-up stays open and keeps the caret. A refused save shows its message in
  the pop-up's error line and runs nothing.
- Other pop-ups (math, callouts, raw HTML) have no Run.

## Actions (007)

| Id | Label | Key | Where |
|---|---|---|---|
| `run` | Run cell | ⇧Enter | element (a cell) |
| `run-stop-cell` | Stop cell | — | element (a running cell) |

The source bar's Run all, Stop and Restart are unchanged. Its badge shows the
engine (`R · idle`, `Julia · busy`).

## Observable JS (US4)

`{ojs}` cells get the same ▶ and states. Running one evaluates it again in the
page, together with the cells that depend on it. No token is needed: they
already run on load.
