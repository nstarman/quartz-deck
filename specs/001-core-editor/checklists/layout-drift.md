# Layout drift: preview copy vs final render (T050)

Question: does the editor's preview (the annotated copy with `data-qd`
markers, research.md R5) lay out exactly like the deck Quarto renders from the
file itself?

Method (2026-11-04, Quarto 1.11.9): `QD_DRIFT_OUT=… cargo test --test drift
-- --ignored` renders every corpus deck twice (`final.html` from the file,
`preview.html` from `preview_copy`). In the browser pane, both were loaded
side by side in 1050×700 iframes; for every slide, each content element
(`p, li, img, pre, table, h1–h5, figure, blockquote, .absolute,
mjx-container, iframe, video, svg`, outside speaker notes, non-zero size) was
measured relative to its `<section>` and compared in order.

Result after fixes: **21/21 decks, 149 elements, 0 differences > 1 px; slide
counts equal.** (`07-code-cells` needs Python/R kernels and `12-includes` a
missing include, so neither renders in either form.)

First run found, and these were fixed in `render::preview_copy`:

| Deck | Symptom | Cause | Fix |
|---|---|---|---|
| 04-slide-level-3 | preview had an extra slide | a `####` content heading wrapped in a `data-qd` div makes Pandoc start a slide at it | headings get `data-qd` in their own attributes, never a wrapper |
| 03-rules, 14-preamble | extra empty `<h2>` on `---`/implicit slides | the synthetic `## {#qd-N}` anchor heading | a raw-HTML style rule hides that empty heading |
| 21-unterminated | 4 slides final, 3 preview | wrapping blocks of an opaque slide changed how Pandoc reads the unclosed fence | opaque slides are copied verbatim, unannotated |

Known model mismatch (not a preview issue): for an *unterminated* code fence
Pandoc does not treat the rest of the file as code, so the final render has a
slide after it that qd's segmenter folds into the opaque slide. qd refuses
structural edits there and `qd check` warns, so nothing is corrupted.
