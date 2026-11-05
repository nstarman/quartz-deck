# Data Model: Source and Run

The `.qmd` gains nothing. New state lives in standard environment files (the
person's, versioned) and in qd's cache (derived, disposable).

## Environment (files next to the deck / at the Quarto project root)

| Field | Notes |
|---|---|
| tool | `uv` (default) or `pixi` — detected from the files (research R1) |
| root | folder holding the manifest and lockfile |
| manifest | `pyproject.toml` (uv), `pixi.toml` or `pyproject.toml` `[tool.pixi]` (pixi) |
| lockfile | `uv.lock` / `pixi.lock` |
| lock_current | false when the manifest changed after the lock → runs/renders refuse (FR-019) |
| base packages | `ipykernel`, `nbclient`, `nbformat`, `pyyaml` (R2) + the deck's own |

## Code cell (derived from the deck)

| Field | Type | Notes |
|---|---|---|
| ref | `<slide>/<i>` | element ref as in specs 001–003 |
| lang | string | from the fence (`{python}`); only `python` runnable (FR-012) |
| options | map | `#|` lines (`eval`, `echo`, `fig-cap`, …) |
| source | string | the cell's code (without the fence) |
| fingerprint | 16 hex | SHA-256 of fence + options + source |
| lines | [start, end] | 1-based line range in the `.qmd` (R10) |

## Run (cache, plus live state in `qd serve`)

| Field | Notes |
|---|---|
| id | increasing per session |
| by | `person` or `agent` |
| mode | `live` (default) or `fresh` (re-run from the top) |
| cells | refs executed, in order |
| status | running → done \| error \| stopped |
| counter | per-session execution counter, stored on each output (FR-008a) |

## Output (cache: `<cache>/outputs/<deck-id>/<fingerprint>/`)

| Field | Notes |
|---|---|
| fingerprint | of the cell when produced → stale if the cell's fingerprint differs (FR-011) |
| counter | execution counter of the run that produced it |
| items | ordered: `text` (stream/plain), `html` (tables, Plotly), `image` (file path, mime), `error` (name, value, traceback) |
| truncated | true when text was cut to 2,000 lines (full text in a side file) |

## Kernel session (in `qd serve`, one per deck)

`stopped → starting → idle ⇄ busy → stopped`; `restart` returns to
`starting`; `interrupt` stops the current execution (status `stopped` for
that run). The bridge process exits with the server.

## Source pane (browser only)

text, base text (last saved), cursor/scroll, pending save, linked slide/block
(from line ranges).
