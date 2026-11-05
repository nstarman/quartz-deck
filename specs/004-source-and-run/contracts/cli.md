# CLI contract additions (spec 004)

Common flags and JSON result shape as in specs 001–003.

## `qd replace`

```text
qd replace --old <TEXT> --new <TEXT>        (or --old-file/--new-file, `-` = stdin for one of them)
```

Replace the unique occurrence of `old` with `new`. `E_CONFLICT` if `old` is not
found (the text changed elsewhere; `details.hint` suggests re-reading),
`E_AMBIGUOUS` if found more than once (add context). Result as other edits
(`diff`, `version`, `target` = slide containing the change). Undoable.

## `qd env`

```text
qd env                         # {"tool","root","manifest","lockfile","lock_current"}
qd env init [--tool uv|pixi]   # create manifest + lock with the base packages (default uv)
qd env add <PKG>...            # add packages (uv add / pixi add --pypi), relock
qd env sync                    # install exactly the lockfile (e.g. on a new machine); relocks after a hand-edited manifest
```

Errors: `E_NO_ENV` (no environment; hint: `qd env init`), `E_ENV_TOOL`
(uv/pixi missing; hint: install command), `E_LOCK_STALE` (manifest newer than
lock; hint: `qd env sync` / `qd env add`), `E_ENV` (tool failed; details hold
its output).

## `qd run`

```text
qd run <CELL>... [--fresh] [--timeout SECS]
qd run --all [--fresh] [--timeout SECS]
qd outputs [<CELL>...]           # {"outputs":[run entry + "stale"]}
```

- `<CELL>`: element ref of a code cell (`results/1`, `#id`).
- With `qd serve` running: uses its live session (outputs appear in the
  editor). Without: a one-shot fresh session (cells 1…k run from the top).
- `--fresh`: restart and run from the top (through the last given cell).
- `--timeout`: interrupt a cell after SECS (status `stopped`).
- Result: `{"ok", "mode":"live|fresh", "status":"done|error|stopped", "runs":[{"cell","counter","status":"done|error|stopped","outputs":[{"kind":"text","text"}|{"kind":"html","html"}|{"kind":"image","path","mime"}|{"kind":"error","ename","evalue","traceback"}]}]}`.
  `ok` is false (exit 1, `E_RUN`) when a cell raised; `--all` stops at the
  first error.
- `E_NOT_CODE` for a non-code element; `E_ENGINE` for a non-Python cell
  ("cannot run here yet").

## `qd show` / `qd slides` (extended)

Slides and blocks gain `"lines": [start, end]` (1-based, inclusive). Code
blocks gain `"lang"`; `kind` is `"cell"` for executable cells (```` ```{python} ````)
and `"code"` for shown code; a cell's fingerprint is its existing `hash`.
Cells gain `"output": {"stale", "counter"}` when an output is stored.

## Render through the environment

`qd render`, `qd export`, `qd check --render` run Quarto through the deck's
environment when it has code cells (`uv run --locked quarto …` /
`pixi run --locked quarto …`); `E_LOCK_STALE` as above. A deck with code
cells and no environment renders with Quarto directly, as before (it never
needs one to start). Unless the deck names a kernel (`jupyter:`), cells run
in qd's `qd-python` kernel spec — the environment's own Python — so a Python
kernel installed elsewhere for the user is never picked up.
