# Offline fixtures (002-offline-export)

Hand-written for quartz-deck (2026-11-04); same license as the repository.
One failure mode (or setup) per deck, from the quarto-revealjs guide
(nstarman/skills#25) and specs/002-offline-export/research.md.

| File | Purpose |
|---|---|
| math.qmd | default deck with math: web MathJax request + raw TeX when not exported |
| fragments.qmd | 3 slides, 2 fragments, speaker notes, `.print-only` / `.print-hide` |
| bgvideo.qmd + media/clip.mp4 | background video must ship beside the export (tiny stand-in file) |
| bgiframe.qmd + apps/app.html | background iframe must ship beside the export |
| missing.qmd | background video that does not exist → P_MISSING_FILE |
| missing-iframe.qmd | background iframe that does not exist → P_MISSING_FILE |
| katex.qmd | `html-math-method: katex` (never embedded) → P_RAW_TEX + P_NETWORK |
| chalkboard.qmd | cannot be a single offline file → P_OFFLINE_INCOMPATIBLE |
| shownotes.qmd | `show-notes` in YAML shows notes to the audience → P_NOTES_VISIBLE |
| throws.qmd | script error → P_PAGE_ERROR |
| plotly.qmd | Python Plotly figure (needs Python plotly; tests skip otherwise) |
| onefile.qmd + figures/ | spec 010: a `data-app` iframe (the test writes `apps/big.html`, 2.5 MB, which throws a known error when it runs), a background-iframe app, an `<object>` SVG, an SVG `<image>`, an image used twice, math, notes, a box past the right edge |
| skillsetup/ | folder already set up per the skill (`_quarto.yml`, header, `_vendor/` stub replaced by tests) |
