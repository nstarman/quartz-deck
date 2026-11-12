# Feature Specification: Export Formats

**Feature Branch**: `011-export-formats`

**Created**: 2026-11-12

**Status**: Draft

**Input**: User description: "Export formats (roadmap 011; PR #43 is 010 one-file-deck): an Export ▾ drop-down in the editor (and File ▸ Export) and `qd export --to <format> [--out PATH]` for agents with a JSON result, for every format Quarto/Pandoc can write. Default route: `quarto render --to <format>` on an export copy (as in 002), so code still runs and Quarto features (callouts, cross-references, layouts, themes) are kept; formats Quarto has no profile for go through `quarto pandoc` on the rendered markdown instead, with a note that Quarto-specific features may be simplified. Slide formats first (P1): offline reveal.js (002, unchanged), PDF of the slides (002), PowerPoint (pptx), Beamer PDF. Then documents (P2): Word, HTML page, PDF via LaTeX or Typst, Markdown (GFM/CommonMark), Jupyter notebook, ODT, EPUB; and (P3) the rest of Pandoc's writers. Per format the spec says what carries over (positioned boxes, fragments, speaker notes, backgrounds, interactive figures, columns, math, code output) and the result reports losses instead of silently dropping content; PDF/LaTeX/Typst formats need a TeX or Typst install, checked and reported like the browser in 002. The .qmd is never changed by an export."

## Context

A deck written in qd is a Quarto reveal.js deck. Today qd exports it two
ways: one offline HTML file and a PDF printed from it (spec 002, extended by
010). People also need the same talk as a PowerPoint file for a host's
laptop, a Beamer PDF for a LaTeX-minded audience, a Word or PDF handout, a
Markdown copy for a repository, or a notebook. Quarto and the Pandoc it
ships can write all of these, but each format keeps a different part of a
slide deck. A pptx file has no reveal.js fragments, a Word handout has no
positioned boxes, and a Markdown file has no themes. If an export silently
drops a speaker note or a placed figure, the person finds out in front of an
audience. This feature makes every format one click or one command away, and
says plainly what each export lost.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Export the slides as PowerPoint or Beamer (Priority: P1)

The person opens **Export ▾** in the editor (or File ▸ Export) and picks
PowerPoint or Beamer PDF. An agent runs `qd export --to pptx` or
`qd export --to beamer`. qd renders a copy of the deck in that format, with
the code run, and writes the file next to the deck. The result names the
file and lists what did not carry over, slide by slide: "slide 4: the box
placed at 300,120 is in the flow", "slide 6: 3 fragment steps shown at
once". The offline reveal.js export and the slides PDF from 002 stay in the
same menu and work as before.

**Why this priority**: Handing a talk to someone who runs PowerPoint, or to
a venue that wants a PDF built from LaTeX, is the most common reason to
leave reveal.js. Both are slide formats, so the deck's structure maps onto
them most directly.

**Independent Test**: Export a fixture deck that has a title slide, two
columns, a placed box, fragments, speaker notes, a background image, an
equation and a code cell with a plot to pptx and to Beamer. Open each file
and check that every slide, note, equation and plot is there, and that the
result lists the placed box and the fragments as losses on the right
slides.

**Acceptance Scenarios**:

1. **Given** a deck with 8 slides, **When** the person exports to
   PowerPoint, **Then** the .pptx file has 8 slides with the same titles in
   the same order, and each slide's speaker notes are in that slide's notes.
2. **Given** a slide with a box placed freely, **When** it is exported to
   pptx or Beamer, **Then** the box's content appears on that slide and the
   result lists it as a loss for that slide ("position not kept").
3. **Given** a deck with a code cell that makes a plot, **When** it is
   exported to either format, **Then** the plot appears as an image on its
   slide, rendered from a fresh run of the code.
4. **Given** the deck file before the export, **When** any export finishes
   or fails, **Then** the deck file is byte-for-byte unchanged.
5. **Given** an agent runs `qd export --to pptx`, **When** it finishes,
   **Then** it prints one JSON result with the output path, the format, the
   losses and whether it succeeded.
6. **Given** `qd export` with no `--to`, **When** it runs, **Then** it does
   exactly what it did before this feature (offline reveal.js HTML).

---

### User Story 2 — Check what a format needs before it runs (Priority: P1)

Beamer and the LaTeX PDF need a TeX install. (Typst comes with Quarto, so
the Typst PDF needs nothing more.) Before it
starts, qd checks for the tool the chosen format needs. If the tool is
missing, the export stops before any rendering, says which tool is missing,
and gives the command that installs it. The Export menu marks formats whose
tool is missing, and says why when the person hovers over one.

**Why this priority**: A missing TeX install shows up today as a long
LaTeX error after a minute of rendering. Checking first saves that time and
tells the person exactly what to install. US1's Beamer export depends on it.

**Independent Test**: With TeX hidden from the search path, ask for a
Beamer export. It must fail at once with "needs a TeX install" and the
install command, and the Export menu must show Beamer as unavailable with
that reason.

**Acceptance Scenarios**:

1. **Given** no TeX install, **When** a Beamer or LaTeX PDF export is
   requested, **Then** it fails before rendering, names the missing tool,
   and gives the install command.
2. **Given** no TeX install, **When** a Typst PDF export is requested,
   **Then** it runs, because Typst comes with Quarto.
3. **Given** a missing tool, **When** the person opens Export ▾, **Then**
   that format is shown disabled with the reason as its tooltip, and other
   formats stay available.
4. **Given** the tools a format needs are present, **When** the person asks
   `qd export --list`, **Then** each format is listed with whether it is
   ready and, if not, what it needs.

---

### User Story 3 — Export a handout or a document copy (Priority: P2)

The person exports the deck as a document: Word, a single HTML page, a PDF
(through LaTeX or through Typst), Markdown (GitHub or CommonMark), a Jupyter
notebook, ODT or EPUB. Each slide becomes a section with its title as the
heading. Speaker notes, where the format has nowhere better to put them,
appear after the slide's content as a marked note, so a handout keeps
them. The result lists what was simplified, as in US1.

**Why this priority**: Handouts, archives and copies for a repository are
common but second to giving the talk itself.

**Independent Test**: Export the US1 fixture to each document format. Open
each one and check the headings, the equation, the plot and the notes, and
that the result's losses match what the file shows.

**Acceptance Scenarios**:

1. **Given** the fixture deck, **When** it is exported to Word, **Then** the
   document has one heading per slide, in order, with the slide's text,
   equation and plot under it.
2. **Given** a slide with speaker notes, **When** it is exported to a
   document format, **Then** the notes appear under that slide's content,
   marked as notes.
3. **Given** a deck exported to Markdown, **When** the file is opened,
   **Then** plots are image links to files written beside it, and the
   result lists those files.
4. **Given** a deck exported to a Jupyter notebook, **When** it is opened,
   **Then** each code cell is a notebook code cell and the text between them
   is in Markdown cells.

---

### User Story 4 — Any other Pandoc writer (Priority: P3)

An agent or a person who knows Pandoc asks for a format that has no menu
entry, such as reStructuredText, Org, JATS or AsciiDoc. qd renders the deck
to plain Markdown with Quarto (so the code runs) and converts that with
Pandoc. The result says that Quarto-only features (callouts,
cross-references, layouts) may be simplified in this format.

**Why this priority**: These formats are rare for slides. They come almost
for free once the other routes exist.

**Independent Test**: Run `qd export --to rst` and `qd export --to org` on
the fixture deck. Each must produce a file that has every slide title, and
a result with the "may be simplified" note.

**Acceptance Scenarios**:

1. **Given** a writer name Pandoc lists but Quarto has no format for,
   **When** it is requested, **Then** qd writes the file and the result
   notes the simpler route.
2. **Given** a name Pandoc does not know, **When** it is requested, **Then**
   qd fails with the list of names it does know (or `qd export --list`).
3. **Given** the editor, **When** the person opens Export ▾ ▸ Other
   format…, **Then** they can pick any listed writer by name.

---

### Edge Cases

- **Code that fails to run**: the export stops with the cell's error and
  the slide it is on, as a render in the editor does. No partial file is
  left at the output path.
- **Output file already exists**: it is replaced. An existing file is never
  touched when the export fails.
- **Output open in another app** (a .pptx open in PowerPoint on Windows,
  locked): the export fails with "close the file and try again" and keeps
  the new file in the staging folder, named in the result.
- **Interactive figures** (Plotly, embedded apps): formats that cannot show
  them list the figure as lost, with the slide. Making a still image is
  left for later (research.md R5).
- **Background video or iframe**: listed as a loss in every format except
  the reveal.js export.
- **Front matter with options for the target format** (for example a
  `format: pptx:` block with a reference document): those options are used.
  Options under `format: revealjs:` that the target has no use for are
  ignored, and not reported as losses.
- **A very long export** (a large deck with slow code): the editor shows
  progress and the person can keep editing. The export works from a copy
  taken when it started, and the result says so if the deck changed
  meanwhile.
- **Two exports at once**: a second request for the same format waits for
  the first. Different formats can run at the same time.
- **Theme and colours**: a theme only reveal.js understands is not carried
  over. This is reported once per export, not on every slide.

## Requirements *(mandatory)*

### Functional Requirements

**Exporting**

- **FR-001**: Users MUST be able to export the current deck to any format
  in the format list (FR-010) from the editor's Export ▾ menu, from
  File ▸ Export, and with `qd export --to <format> [--out PATH]`.
- **FR-002**: `qd export` with no `--to` MUST behave exactly as before
  (offline reveal.js export, with `--pdf` and its options).
- **FR-003**: Exports MUST render a copy of the deck, never the deck
  itself, and MUST leave the deck file byte-for-byte unchanged, whether the
  export succeeds or fails.
- **FR-004**: Exports MUST run the deck's code cells in the deck's own
  environment (as rendering does, spec 004), so code output is fresh.
- **FR-005**: Where Quarto has a format of that name, the export MUST use
  Quarto's own rendering for it, so callouts, cross-references, figure
  layouts and columns are kept as Quarto keeps them.
- **FR-006**: For a Pandoc writer that Quarto has no format for, the export
  MUST render the deck to Markdown with Quarto first and convert that, and
  the result MUST note that Quarto-only features may be simplified.
- **FR-007**: The default output path MUST be `export/<deck name>.<ext>`
  next to the deck. `--out` sets another path; in the editor the person
  can change the path in the export dialog.
- **FR-008**: The file MUST appear at the output path only when the export
  succeeded. A failed export MUST leave any existing file there untouched.
- **FR-009**: Files a format writes beside the main file (images for
  Markdown, for example) MUST go in a folder next to it named after it, and
  the result MUST list them.

**Formats**

- **FR-010**: The format list MUST include, grouped as below, with these
  names on the command line:
  - Slides: offline web page (`revealjs`, the 002 export), slides PDF
    (`revealjs-pdf`, the 002 PDF), PowerPoint (`pptx`), Beamer PDF
    (`beamer`).
  - Documents: Word (`docx`), web page (`html`), PDF through LaTeX
    (`pdf`), PDF through Typst (`typst`), Markdown (`gfm`, `commonmark`),
    Jupyter notebook (`ipynb`), OpenDocument (`odt`), EPUB (`epub`).
  - Other: every other writer that the installed Pandoc lists.
- **FR-011**: `qd export --list` MUST print every format with its group,
  file extension, whether it is ready, and what it needs if it is not.

**What carries over**

- **FR-012**: For each format in the Slides and Documents groups, the
  carry-over table below MUST hold. Planning confirms each cell against
  the installed Quarto and records any change in the plan. "Kept" means
  the content is in the file and works the same way; "flow" means it is
  in the file but in reading order without its position; "still" means a
  static image replaces it; "lost" means it is not in the file.

  | Deck feature        | pptx  | beamer | docx/odt | html | pdf/typst | gfm/commonmark | ipynb | epub |
  |---------------------|-------|--------|----------|------|-----------|----------------|-------|------|
  | Slides and titles   | kept  | kept   | sections | sections | sections | sections   | sections | sections |
  | Speaker notes       | kept (notes) | lost | after content | after content | after content | after content | after content | after content |
  | Pauses (`. . .`)    | lost (all shown) | kept (overlays) | lost | lost | lost | lost | lost | lost |
  | Fragment blocks     | lost (all shown) | lost (all shown) | lost | lost | lost | lost | lost | lost |
  | Columns             | kept (two-content) | kept | flow | kept | flow | flow | flow | flow |
  | Placed boxes        | flow  | flow   | flow     | flow | flow      | flow           | flow  | flow |
  | Background image    | kept  | lost   | lost     | lost | lost      | lost           | lost  | lost |
  | Background video/iframe | lost | lost | lost  | lost | lost      | lost           | lost  | lost |
  | Math                | kept (native) | kept | kept (native) | kept | kept | kept (as TeX) | kept | kept |
  | Code output (plots, tables) | kept | kept | kept | kept | kept | kept (image files) | kept (outputs) | kept |
  | Interactive figures | lost  | lost   | lost     | kept (online) | lost | lost        | kept (online) | lost |
  | reveal.js theme     | lost  | lost   | lost     | lost | lost      | lost           | lost  | lost |

  Checked with a fixture deck against Quarto 1.11.9 (research.md R1). In
  every format the code itself stays hidden, as on the slides; "lost"
  for an interactive figure means the file says "Unable to display
  output", so qd reports it; "online" means the figure loads its library
  from the web.

- **FR-013**: Every export MUST report, for each deck feature the table
  marks "flow", "still" or "lost" for that format, every slide where the
  deck uses it, and what happened to it. Features the deck does not use
  MUST NOT be reported.
- **FR-014**: Speaker notes MUST never be dropped silently. In a format
  where they are lost (Beamer), the result MUST say so for every slide that
  has notes, and the export dialog MUST say so before the export starts.

**Tools**

- **FR-015**: Before rendering, qd MUST check that the tool a format needs
  is installed (TeX for `beamer` and `pdf`, the browser for `revealjs-pdf`
  as in 002; Typst comes with Quarto). If it is missing, qd MUST fail before
  rendering with the tool's name and the command that installs it.
- **FR-016**: qd MUST NOT install TeX itself.
- **FR-017**: The Export ▾ menu MUST show a format whose tool is missing as
  disabled, with the reason as its tooltip.

**Result and editor**

- **FR-018**: `qd export --to` MUST print one JSON result with: whether it
  succeeded, the format, the route taken (Quarto's format or Markdown then
  Pandoc), the output path, extra files, the losses (FR-013), notes (such
  as FR-006's), and on failure the error and the slide it concerns.
- **FR-019**: In the editor, an export MUST show progress while it runs,
  let the person keep editing, and end with the result: the file (with a
  button to show it in the file manager), and the losses grouped by slide,
  each slide name a link to that slide.
- **FR-020**: Export ▾ and every format in it MUST be actions in the 007
  action list, so they appear in File ▸ Export, in the command search and
  in the shortcut reference. The existing export actions MUST keep their
  ids and keys.
- **FR-021**: The server's export endpoint MUST require the per-session
  token, as other endpoints that write files or run code do.

### Key Entities

- **Format**: a name used on the command line, a label, a group (Slides,
  Documents, Other), a file extension, the route (Quarto's own format, or
  Markdown then Pandoc), the tool it needs, and its column of the
  carry-over table.
- **Export result**: success, format, route, output path, extra files,
  losses, notes, and an error with its slide on failure.
- **Loss**: a deck feature, the slides where the deck uses it, and what
  happened to it there (in the flow, a still image, or not in the file).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: From the editor, a person can export the open deck to
  PowerPoint in at most 2 clicks and see the result without leaving the
  editor.
- **SC-002**: On the fixture deck, every loss visible in the exported file
  of each Slides and Documents format is listed in the result, and the
  result lists nothing the file actually kept (0 missed, 0 false losses).
- **SC-003**: Every exported file of the Slides and Documents formats
  opens without a repair prompt in the app made for it (PowerPoint or
  Keynote for pptx, a PDF viewer, Word or LibreOffice, a browser, Jupyter,
  an EPUB reader).
- **SC-004**: With a needed tool missing, the export fails within 2
  seconds with the tool's name and install command, before any code runs.
- **SC-005**: Across the whole test suite, no export changes a deck file
  (checked byte-for-byte before and after every export test).
- **SC-006**: An agent can export to any listed format and learn every
  loss from the JSON result alone, without opening the file.

## Assumptions

- PR #43 (010 one-file-deck) is merged before this feature's code starts;
  the offline reveal.js export and the slides PDF build on its `qd export`.
- The installed Quarto (1.11.9 here) and its bundled Pandoc set which
  writers exist. qd reads the list from them rather than keeping its own.
- TeX is found on the search path or where Quarto finds it (including
  Quarto's own TinyTeX); the install command given is
  `quarto install tinytex`. Typst is bundled with Quarto (0.15.1 in 1.11.9).
- "PDF" in the Documents group means a document, one slide per section, as
  opposed to the slides PDF, which keeps one page per slide.
- Speaker notes "after content" are a quoted block headed "Speaker notes"
  (research.md R2).
- Exports run one per format at a time; their run time is the render's run
  time plus conversion, and no target time is set beyond SC-004.
