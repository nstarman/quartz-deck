# Quickstart run (T018, 2026-11-12)

The quickstart, run on a copy of `tests/fixtures/export/carry.qmd`. The
machine has Quarto 1.11.9, TeX Live and a uv environment with jupyter,
matplotlib and plotly. The editor is the debug build, in the Claude browser
pane.

| § | Check | Result |
|---|---|---|
| 1 | `qd export --list` | 13 menu formats, all ready; 63 other writers, `rst` and `org` among them |
| 2 | `--to pptx` | 9 slides; slide 2 uses Two Content; NOTESCOLS in the notes; `<p:bg>` on the Background slide; no ". . ."; losses `placed` [placed], `pause` and `fragment` [frags], `interactive` [plotly] |
| 3 | `--to beamer` | 10 pages (the pause gives Fragments 2 pages); losses `notes` hidden [cols], `placed`, `fragment`, `background-image` [bg], `interactive` |
| 3 | TeX hidden (PATH = Quarto's folder and `/usr/bin:/bin`, empty HOME) | `E_NO_TEX` in 0.06 s with the hint `install TeX: quarto install tinytex`; `--out nope.pdf` not created; `--to typst` still works |
| 4 | docx, typst, gfm, ipynb | all ok; the Typst PDF has the "Speaker notes" label; the Markdown has no `plt.plot`; `export/carry_files/figure-commonmark/` is listed in `files` |
| 5 | `--to rst` | rendered straight by Quarto (re-run after the spec revision); the slide titles and the labelled notes are in the file, and the plot is in `export/carry_files/figure-rst/`, listed in `files` |
| 6 | Editor | Export ▾ lists Slides, Documents and Other format…; PowerPoint shows the losses, exports, and the result panel opens with slide links and a download link; Other format… suggests the writers and exports `rst` (browser test `export_other_format`); with TeX hidden, Beamer and PDF (LaTeX) are greyed out with the hint (browser test `export_menu_disables_missing_tools`) |
| 7 | `POST /api/op` export without `X-QD-Token` | 403; with the token, 200 |

The deck's SHA-1 was the same before and after, and no `.qd-` temp files were
left in the folder or in `export/`.

Found while running it (fixed in #46): since 008 the side panel is closed by
default, so export results, including 002's offline export, were invisible.
They now open the panel and leave the setting as it was.
