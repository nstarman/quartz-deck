# Research: Export Formats (011)

All findings come from Quarto 1.11.9 (Pandoc 3.10, Typst 0.15.1 bundled)
on this machine, with TeX Live on the path. The test deck is
`tests/fixtures/export/carry.qmd`. Each feature on it carries a marker word
(`NOTESCOLS`, `PLACEDBOX`, `FRAGA`…) so a text search of the output shows
what survived. Each format was rendered with
`quarto render carry.qmd --to <f> -o out-<f>.<ext>`, using a uv
environment with jupyter, matplotlib and plotly.

## R1. What each format keeps (measured)

Decision: the carry-over table in spec FR-012 is now the measured result.
It corrects the first draft in these places:

- **Pauses (`. . .`)**: Pandoc treats a pause as a pause only in Beamer,
  where it becomes an overlay (the Fragments slide became 2 pages). Every
  other format writes the dots as a literal ". . ." paragraph, pptx
  included. That is visible garbage, so the export filter removes it (R2).
- **Fragment blocks (`::: {.fragment}`)** are not overlays even in Beamer.
  Their content is shown at once in every format.
- **Speaker notes** are written as plain text in the body in every
  document format, with nothing to tell them apart from slide text. In
  pptx they land in the slide's notes (correct). Beamer writes `\note{}`,
  which a normal Beamer PDF does not show.
- **Code**: revealjs hides code by default (`echo: false`), and so does
  pptx. All the document formats show it (`echo: true`), so a handout would
  suddenly contain source code the slides never showed. The export sets
  `echo: false` unless the deck sets `echo` itself.
- **Interactive figures (Plotly)**: pptx, Beamer, docx, odt and both PDFs
  write "Unable to display output for mime type(s): text/html" where the
  figure was. html and ipynb keep it, but it loads plotly.js from the web.
  GFM/CommonMark and EPUB embed the raw HTML and scripts, which their
  readers do not run.
- **Columns**: kept in pptx (Two Content layout), Beamer and the html page
  (Quarto's html CSS has `.columns { display: flex }`). In the PDF/Typst
  documents, docx, odt, Markdown, ipynb and EPUB, the columns come out one
  after the other.
- **Placed boxes**: in the flow in every format (absolute positions are a
  revealjs feature).
- **Background image**: kept only in pptx (the slide gets a `<p:bg>`).
- **Callouts**: kept everywhere (GFM as `> [!NOTE]`).
- **Math**: kept everywhere (OMML in docx/pptx, MathML in odt/epub, TeX
  in Markdown).
- **Plots**: kept everywhere as images. GFM/CommonMark reference
  `<stem>_files/figure-commonmark/*.png`.

Rationale: the spec's table has to match what the file really contains,
or the loss report (FR-013) would be wrong (SC-002).

## R2. Adapting a slide deck for other formats: one Lua filter

Decision: every Quarto-route export (except revealjs and revealjs-pdf)
renders the copy with `--metadata-file <tmp>.yml` holding
`echo: false` (only when the deck has no `echo` of its own) and
`filters: [qd-export.lua]`. The filter is embedded in the binary
(`include_str!`) and written next to the copy. It does three things:

- removes pause paragraphs (`. . .`), except when `FORMAT == "beamer"`;
- turns `.notes` divs into a block quote headed **Speaker notes**,
  except in pptx and Beamer, which have their own place for notes;
- unwraps `.fragment` and `.absolute` divs, so their content stays in the
  flow without leftover wrapper divs (in GFM those would otherwise be raw
  `<div class="absolute" data-left=…>`).

Tested in typst, docx and gfm: no ". . ." is left, notes show under a
"Speaker notes" label, and no code is shown.

Alternatives considered:
- *Turn notes into `.callout-note`*: a user filter that runs `pre-quarto`
  is too late for Quarto's callout parsing, so the div was left as plain
  text. A block quote works the same in every writer.
- *Edit the copy's text instead of using a filter*: this would mean
  re-parsing pauses and divs that Pandoc already parses. The filter is
  20 lines.

## R3. Routes

Decision:
- **Quarto route**: `quarto render <copy> --to <f> --metadata-file … -o
  <name>`, run in the deck's folder through the same helper that live
  rendering uses (`render::quarto_cmd` + `run_quarto`), so the deck's
  environment (004), its kernel and the one-Quarto-run-at-a-time lock
  apply. Used when `<f>` is one of Quarto's formats.
- **Pandoc route** (P3): render the copy `--to md` by the Quarto route
  (code runs, filter applied), then
  `quarto pandoc <copy>.md -t <writer> -o <out>` in the same folder. Used
  for writers that Quarto has no format for. The result carries a note
  (FR-006).
- How qd tells the two apart: names in a fixed list of Quarto formats
  (`pptx beamer docx odt html pdf typst gfm commonmark commonmark_x ipynb
  epub epub2 epub3 latex context ms rtf jats* docbook* asciidoc
  markdown*`) take the Quarto route, and other writers from `quarto pandoc
  --list-output-formats` take the Pandoc route. Quarto accepts any Pandoc
  writer with `--to`, but formats outside its list skip its format
  defaults, so Pandoc is the simpler and more predictable path for them.
  ponytail: fixed list; read Quarto's own list if it ever gains a command
  for it.

## R4. Tools

Decision: `typst` needs nothing, because it is bundled
(`quarto typst --version` → 0.15.1). `pdf` (LaTeX) and `beamer` need TeX.
Found = `xelatex`, `lualatex` or `pdflatex` on `PATH`, or Quarto's TinyTeX
(`~/Library/TinyTeX/bin/*` on macOS, `~/.TinyTeX/bin/*` on Linux).
Missing → `E_NO_TEX`, with the hint
`install TeX: quarto install tinytex`, raised before any render (SC-004).
`revealjs-pdf` keeps 002's browser lookup. qd never installs TeX (FR-016):
TinyTeX is a few hundred MB and changes the person's TeX setup.

## R5. Interactive figures

Decision: report them, don't convert them. Interactive = the deck uses
plotly (`export::uses_plotly`), or a cell has a recorded output (004
outputs) whose only rich type is `text/html` or a widget. Formats marked
"lost" list each such slide. A still would need kaleido or a headless
browser screenshot per figure, which can be added later if people need
it.

## R6. Losses come from the deck, not from the output file

Decision: losses are computed from the deck model (slides, blocks, attrs)
and the format's column of the table: notes blocks, pause blocks,
`.fragment` divs or `fragment` classes, `.absolute` boxes, slide
`background-image`/`background-video`/`background-iframe`, `.columns`,
interactive cells, and the theme (`format.revealjs.theme`, reported once).
Each is reported only if the deck uses it (FR-013). Reading pptx/docx/pdf
output files back would mean a parser per format.

## R7. Safety and the deck file

- The copy is written next to the deck with a temp name (as 002's export
  copy and the preview copy are), so relative paths resolve. It is
  removed afterwards, along with the filter and metadata files.
- The output goes first to `<out>.qd-tmp-<nanos>.<ext>` and is renamed onto
  `<out>` only on success (FR-008). A rename that fails (for example, the
  file is locked) leaves the temp file and names it in the error.
- Tests hash the deck before and after every export (SC-005).
- **Token**: `/api/op` checks only same-origin today, yet `render`,
  `export` and `pdf` run code. The plan makes `/api/op` require the session
  token for those three ops (FR-021, constitution Constraints). The editor's
  `op()` sends no token today, so it gains `X-QD-Token: run.token`, as
  `/api/layouts` and image drops already do.

## R8. Coexisting with PR #43 (010 one-file-deck)

#43 changes `src/export.rs` (52 lines), `src/offline.rs`, `src/cli.rs`, and adds
`inline.rs` and `bib.rs`. To keep the overlap small, the new code goes in
**`src/formats.rs`**. The only `export.rs` change is one early branch in
`cli.rs`'s `Export` handler (`--to` set and not `revealjs` → `formats::run`).
The code waits until #43 is merged, then rebases (as the user chose).

## Measured times (fixture deck, this machine)

pptx 12 s, beamer 43 s (first LaTeX run), docx 10 s, odt 10 s, html 10 s,
pdf 19 s, typst 11 s, gfm 11 s, commonmark 10 s, ipynb 10 s, epub 21 s.
Most of each is the Jupyter kernel start.
