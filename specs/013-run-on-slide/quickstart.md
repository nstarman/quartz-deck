# Quickstart: Run on the slide (013)

Run in the browser pane against a scratch deck (qd's config and cache in
scratch folders, as for 012).

## 1. Run from the slide (US1)

A Python deck with an environment (`qd env init`, `qd env add matplotlib`)
and a slide with a plotting cell. `qd serve`, then open the editor.

Expected:
- hovering the cell shows ▶;
- clicking it marks the cell running, then the new plot replaces the old one
  in place, with no jump to another slide;
- editing the cell in the source pane marks its output stale on the slide.

## 2. Change a plot from the pop-up (US2)

Double-click the cell to open its code over the slide. Change the line
colour and press ⇧Enter.

Expected: the deck file has the new colour (one undo step), the plot on the
slide is re-drawn, and the pop-up is still open. Press ⇧Enter twice quickly:
the slide ends with the second run's plot.

## 3. Another engine (US3)

- **Jupyter**: with a second kernelspec installed (`qd-test`), a deck with
  `jupyter: qd-test` runs in it. `qd run` reports `"engine": {"kind":
  "jupyter", "kernel": "qd-test"}`.
- **knitr**: where R with knitr and rmarkdown is installed, an `{r}` deck
  runs three dependent cells one at a time from the slides. The outputs match
  `quarto render` (SC-004).
- **Nothing installed**: without R, ▶ says R is missing and runs nothing.

## 4. No Run without the token (FR-008)

Open `/present/` and the editor without `#t=…`. No ▶ shows anywhere, and
`POST /api/run` without the token is `403`.

## 5. Observable JS (US4), native Julia (US5)

When built: an `{ojs}` cell re-evaluates from its ▶ after a pop-up edit, and
an `engine: julia` deck runs its cells in one Julia session.
