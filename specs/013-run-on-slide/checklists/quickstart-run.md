# Quickstart run: 013 run on the slide (T018)

**Date**: 2026-11-13
**Build**: branch `013-polish` (debug), `qd serve` in the Claude browser pane
**Setup**: a scratch deck with a matplotlib cell (`plot`), a cell that prints
a random number (`count`) and two OJS cells (`n = 3`, `n * 2`); a uv
environment with matplotlib; qd's config and cache in scratch folders. This
machine has no R or Julia, so those decks were run in CI (`knitr_cells`,
`run_knitr_from_slide`, `julia_cells`; PRs #57 and #59).

## §1 Run from the slide (US1) — pass

- [x] Before running, the plot cell (code hidden by revealjs) showed "Code
  cell, not run yet".
- [x] Pointing at it showed the ▶, named "Run cell plot/0". Clicking it gave
  the cell `qd-running`, then the plot appeared in place. It was the same
  page (a mark on the frame's window survived) and the same slide, and the
  badge read `Python · idle`.

## §2 Change a plot from the pop-up (US2) — pass (one fix)

- [x] Right-click ▸ Edit markdown opened the pop-up with **Run ⇧Enter**.
  Changing `tab:blue` to `tab:red` and pressing ⇧Enter wrote the file,
  re-drew the plot and left the pop-up open.
- [x] After Esc, Edit ▸ Undo brought back `tab:blue`, and the plot (made
  with red) was marked stale.
- [x] Fixed: the ⇧Enter in that check was a synthetic event, and it ran
  code. The editor's own key actions refuse synthetic events for anything
  that runs code (004), but the pop-up's key handler and its button didn't
  check. Both now require a real key press or click (`e.isTrusted`). The
  browser tests use trusted input, so they still pass.

## §3 Another engine (US3) — pass (one fix)

- [x] An `{r}` deck with no R installed: `qd run a/0` gave `E_ENGINE`
  "cannot run r cells: R is not installed (https://cran.r-project.org)".
- [x] A named kernel is checked by `named_kernel` (a `qd-test` kernelspec in a
  temporary Jupyter folder), which passes here. R and Julia passed in CI.
- [x] Fixed: `qd run` on an `{ojs}` cell said "cannot run ojs cells in a
  deck that runs Python". It now says "cannot run ojs cells from here: they
  run in the editor's page (`qd serve`)", as the CLI contract says.

## §4 No Run without the token (FR-008) — pass

- [x] `POST /api/run` without the token: 403.
- [x] `/present/` loads no editor code (no ▶, no `runslide`).
  `no_run_without_token` covers an editor page opened without `#t=`.

## §5 Observable JS (US4), native Julia (US5) — pass (one fix)

- [x] The OJS declaration `n = 3` shows no value on the slide; its
  placeholder now reads "Observable JS cell (a declaration shows no value)".
- [x] Fixed: the OJS text had been losing to the generic placeholder's more
  specific selector. It is now a CSS variable.
- [x] Its ▶ loaded the preview again, on the same slide, with `n * 2`
  showing 6, and made no request to `/api/run`.
- [x] Native Julia: `julia_cells` in CI (state, printed output and the last
  value, an error, `--timeout` → `stopped`).
