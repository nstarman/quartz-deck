# Quickstart: Export Formats (011)

Checks that prove the feature works end to end. The fixture is
`tests/fixtures/export/carry.qmd`: one marker word per feature, a plot, a
Plotly figure and notes. It needs an environment (`qd env init` in a copy
of the folder) because it has code.

## 1. List formats

```bash
qd --deck tests/fixtures/export/carry.qmd export --list
```

Expected: pptx, beamer, docx, odt, html, pdf, typst, gfm, commonmark,
ipynb and epub, plus revealjs and revealjs-pdf, all `ready: true` on a
machine with TeX. Then Other writers such as `rst` and `org`.

## 2. PowerPoint (US1)

```bash
qd --deck carry.qmd export --to pptx
```

Expected: `export/carry.pptx` with 9 slides. The Columns slide uses Two
Content, the Background slide has a background, and the Columns slide's
notes contain NOTESCOLS. There is no ". . ." anywhere. `losses` lists
`placed` (#placed), `pause` and `fragment` (#frags), `interactive`
(#plotly), and `background-media` only if the deck has one. The deck's
hash is unchanged.

## 3. Beamer (US1, US2)

```bash
qd --deck carry.qmd export --to beamer
```

Expected: a PDF with 10 pages (the pause makes 2 pages of Fragments).
`losses` has `notes` as `hidden` (#cols), `background-image` (#bg) and
`interactive` (#plotly).

With TeX hidden:

```bash
PATH=/usr/bin:/bin qd --deck carry.qmd export --to beamer
```

Expected: `E_NO_TEX` in under 2 s, no `export/carry.pdf` created,
and the hint `quarto install tinytex`. `qd export --to typst` still works
with TeX hidden.

## 4. Documents (US3)

```bash
for f in docx typst gfm ipynb; do qd --deck carry.qmd export --to $f; done
```

Expected: every file has all marker words. NOTESCOLS sits under a
"Speaker notes" label, there's no ". . ." and no Python source (echo off).
For gfm, `export/carry_files/` holds the plot and is listed in `files`.

## 5. Other writers (US4)

```bash
qd --deck carry.qmd export --to rst
```

Expected: `route: "pandoc"`, a note that Quarto features may be
simplified, and every slide title in `export/carry.rst`.

## 6. Editor

Open the editor link from `qd url`. Export ▾ ▸ PowerPoint: the sheet
lists the losses before the export and the result links the file. With
TeX hidden, Beamer is disabled and its tooltip says why. Clicking a slide
name in the result selects that slide.

## 7. Token

`curl -X POST localhost:<port>/api/op -H 'Origin: http://localhost:<port>' -d '{"op":"export","args":{"to":"pptx"}}'`
without the token header → 403.
