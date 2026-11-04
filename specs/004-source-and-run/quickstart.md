# Quickstart: validating source and run

Prerequisites: Quarto ≥ 1.4, **uv** (or pixi), Chrome, `cargo build --release`.
Network for the first `qd env init` (packages are locked and cached after).

## 1. A reproducible environment per deck (FR-017–020, SC-007)

```bash
cd talk/ && qd env init            # pyproject.toml + uv.lock with the base packages
qd env add numpy matplotlib        # manifest and lock updated
qd env                             # tool uv, lock_current true
git add pyproject.toml uv.lock     # the environment travels with the deck
```

On another machine: `qd env sync` then `qd render` — same package versions.
Edit `pyproject.toml` by hand without relocking: `qd run --all` refuses with
`E_LOCK_STALE`.

## 2. Source pane (US1, US2)

```bash
qd serve --open
```

Click **Source** (split view). Edit a word in a paragraph and a value in the
YAML: `git diff` shows only those changes; the slides re-render; ⌘Z undoes to
identical bytes. Move the cursor into another slide's text: the preview
follows. Click a list on a slide: the source scrolls to it.

## 3. Run (US3, US5)

Press **Run** on a cell that prints and one that plots: output under each cell
and on its slide; `.qmd` unchanged (`git diff` empty). Run a cell with
`1/0`: traceback shown. `while True: pass` → **Stop**. Restart the editor:
outputs are still there; edit a cell: its output is marked stale.

## 4. Agents (US4)

```bash
qd run results/1                   # JSON outputs; figure saved as a file
qd run --all                       # stops at the first failing cell
qd outputs                         # stored outputs with "stale"
```

With the editor open, the agent's outputs appear in the source pane.

## 5. Safety (FR-015, SC-006)

From another local page, `fetch('http://127.0.0.1:<port>/api/run', …)` without
the token → 403; no code runs.
