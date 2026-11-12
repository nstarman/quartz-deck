# Data model: Run on the slide (013)

Nothing new is stored in the deck. Stored outputs (004) are unchanged.

## Engine

The answer to "how does this deck run its cells" (research R2, R3).

| Field | Type | Notes |
|---|---|---|
| `kind` | `jupyter` \| `knitr` \| `ojs` \| `julia` \| `none` | from `quarto inspect` `engines[0]` (`markdown` → `ojs` if the deck has `{ojs}` cells, else `none`) |
| `kernel` | string \| null | Jupyter only: the named kernel, else qd's `qd-python` for a Python deck with an environment, else chosen by language in the bridge |
| `language` | string | the first executable cell's language (`python`, `r`, `julia`, …) |
| `fig` | `{width, height, dpi}` | the revealjs `execute` options (inches, dpi), passed to the session so plots match a render |
| `label` | string | what the editor's badge shows: `Python`, `R`, `Julia`, or the kernel's display name |

Cached at `<cache>/engine/<hash>.json`, where the hash covers the front matter
and the set of cell languages. A front-matter or language change asks Quarto
again.

## Session (extended from 004)

One live process per deck while `qd serve` runs (or per `qd run` without a
server). Its `engine` is fixed at start. A change of engine (front matter
edited) ends the session, and the next run starts the right one. States are
the same as 004: `starting`, `idle`, `busy`, `stopped`.

## Run (extended from 004)

| Field | Type | Notes |
|---|---|---|
| `run` | integer | increasing per server; the editor keeps the newest it started per cell and ignores older `cell_output`s (FR-007) |
| `cell` | ref | the cell's block id |
| `status` | `done` \| `error` \| `stopped` | as 004 |
| `html` | string | NEW in `cell_output`: the block the preview shows (`<div class="cell-output qd-output" data-qd-cell="REF">…</div>`) |

## Cell state on the slide (editor only)

`idle` (no class), `running`, `error`, `stopped`, `stale` (stored output's
fingerprint ≠ the cell's text). It is shown as a class on the cell's element
in the preview and on its ▶. It's never saved.
