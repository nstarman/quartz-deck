# Feature Specification: Offline Export and PDF

**Feature Branch**: `002-offline-export`

**Created**: 2026-11-04

**Status**: Draft

**Input**: User description: "Offline export for quartz-deck decks: `qd export` produces a self-contained single HTML file (works offline, no network requests) and a PDF printed from it (one page per fragment step, speaker notes optional), plus an offline check that verifies the exported deck makes no network requests, references no missing local files, has no page errors and shows no raw TeX. Usable from the CLI (JSON output) and the browser editor. Modelled on the quarto-revealjs skill (nstarman/skills#25): MathJax/plotly loaded locally, background video/iframes must ship beside the deck, Chrome's plain --print-to-pdf gives blank pages so reveal's ?print-pdf route is used."

## Clarifications

### Session 2026-11-04

- Q: Who provides the offline setup (local MathJax/plotly.js and the
  quarto-revealjs configuration)? → A: qd applies it at export time, without
  changing the deck or project files; the libraries are fetched once into qd's
  cache (network needed that one time) and reused.
- Q: Which browser powers the offline check and PDF? → A: an installed Google
  Chrome/Chromium if one is found; otherwise qd downloads a pinned headless
  Chromium into its cache on first use.

## Context

A talk has to work on a conference laptop with no Wi-Fi, and an archived deck
has to open years later. A default Quarto reveal.js deck fails both: even with
`embed-resources` it fetches MathJax from the web (on every deck, math or not),
KaTeX is never embedded (math shows as raw TeX offline), every Plotly figure
pulls its own copy of MathJax and plotly.js from the web, and background
videos/iframes stay as separate files. On a connected machine all of this looks
fine, so "it rendered" proves nothing. The `quarto-revealjs` skill
(nstarman/skills#25, verified against Quarto 1.11.9 / reveal.js 5.1.0) records
these failure modes and a tested setup that avoids them; this feature makes
qd produce and *prove* such an archive.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Check a deck works offline (Priority: P1)

Before a talk, the author (or an agent) asks qd whether the rendered deck will
work with no network. qd opens it with all network access blocked, steps
through every slide, and reports each problem: a request to the internet, a
missing local file, a script error, or math left as raw TeX — with the slide
where it happens and how to fix it.

**Why this priority**: Detecting the problem is the core value and the
foundation for everything else; it is useful even before qd can fix anything,
and it is how the export in US2 is proven.

**Independent Test**: Run the check on (a) a default Quarto deck with an
equation — it must report the web MathJax request and raw TeX — and (b) a deck
set up per the quarto-revealjs skill — it must pass.

**Acceptance Scenarios**:

1. **Given** a rendered deck that loads a script from the web, **When** the
   offline check runs, **Then** it fails and lists that URL and the slides
   affected.
2. **Given** a deck with math rendered by a method that is not embedded,
   **When** the check runs, **Then** it reports raw TeX on the slides where it
   appears.
3. **Given** a deck whose background video or background iframe file is not
   beside the exported HTML, **When** the check runs, **Then** it reports the
   missing file and the slide.
4. **Given** a deck that is fully self-contained, **When** the check runs,
   **Then** it passes with an empty problem list.
5. **Given** the browser needed for the check is not available, **When** the
   check runs, **Then** it fails with a clear message on how to provide one,
   and all editing commands keep working.

---

### User Story 2 — Export a self-contained deck (Priority: P2)

The author runs one export command and gets a folder ready to copy to a USB
stick: one self-contained HTML file plus any files that cannot be inlined
(background videos, background iframes), with the offline check from US1
already run on it and its result reported.

**Why this priority**: The deliverable the user actually wants; depends on US1
to prove it.

**Independent Test**: Export a deck that has an equation, a Plotly figure and a
background video; disconnect the network; open the HTML: math renders, the
figure is interactive, the video plays, and the check passed.

**Acceptance Scenarios**:

1. **Given** a deck with math, **When** it is exported, **Then** the HTML
   contains everything needed to typeset that math with no network, and the
   check passes.
2. **Given** a deck with background video/iframe or parallax images, **When**
   it is exported, **Then** those files are copied next to the HTML at the
   relative paths the deck uses, and the result lists them.
3. **Given** a deck in a folder already set up per the quarto-revealjs skill,
   **When** it is exported, **Then** the project's own setup is used and the
   deck's source files are not modified.
4. **Given** the export cannot be made self-contained (e.g. a feature that
   cannot work offline such as multiplex, or chalkboard which refuses single
   files), **When** it is exported, **Then** qd says which feature blocks it
   and on which slide, instead of producing a deck that silently needs the
   network.
5. **Given** an existing export at the destination, **When** export runs
   again, **Then** it is replaced only after the new export succeeds.

---

### User Story 3 — Print the deck to PDF (Priority: P3)

The author gets a PDF of the exported deck as a backup and handout: one page
per slide or per fragment step, live content (interactive figures, embedded
apps) shown as a still image where one is provided, and speaker notes on their
own pages when asked for.

**Why this priority**: The archival backup; valuable but secondary to a
working HTML.

**Independent Test**: Export a deck with fragments, notes and an interactive
figure to PDF; the page count equals the number of fragment steps; with notes
requested, notes appear on separate pages; without, the audience view has none.

**Acceptance Scenarios**:

1. **Given** a deck with 3 slides, one of which has 2 fragment steps, **When**
   it is printed, **Then** the PDF has 4 pages (one per step) by default.
2. **Given** the notes option, **When** printing, **Then** speaker notes appear
   on separate pages; **without** it, notes do not appear anywhere in the PDF.
3. **Given** a slide with live content marked with a print still, **When**
   printing, **Then** the still appears instead of a blank area.
4. **Given** printing completes, **When** the result is reported, **Then** it
   includes the PDF path and page count.

---

### User Story 4 — Export and check from the editor and by agents (Priority: P4)

In the browser editor, an Export action runs export (+ optional PDF) and shows
the check result; problems link to the slides they affect. An agent can do the
same through the CLI with JSON output and the Skill documents it.

**Why this priority**: Same capability, more surfaces; follows constitution III
(every GUI action is a CLI command).

**Independent Test**: Click Export in the editor on a deck with a web request;
the problem list appears and clicking an entry navigates the preview to that
slide. The equivalent CLI command returns the same problems as JSON.

**Acceptance Scenarios**:

1. **Given** the editor, **When** the user clicks Export, **Then** the result
   (paths, check status, problems) is shown without leaving the editor.
2. **Given** a problem tied to a slide, **When** the user selects it, **Then**
   the preview shows that slide.
3. **Given** an agent, **When** it runs the export/check commands, **Then**
   it receives JSON with the same information and stable problem codes.

### Edge Cases

- Deck with no math and no Plotly: export must still not load anything from the
  web (the math plugin loads MathJax from the web on every deck by default).
- Plotly figures from code cells: each figure's own web MathJax/plotly.js
  references must not survive into the export.
- Embedded single-file apps (iframes inlined as data URLs) count as part of the
  deck; background iframes are files that must ship beside it.
- Fragments inside equations: page count and stepping must still be right.
- Very large decks (100 slides, large media): the check reports progress and
  does not time out silently.
- Links to external websites in slide text are not network *requests* and must
  not fail the check (they are only followed when clicked).
- `show-notes` set in the deck's YAML would show notes to the audience: the
  check warns about it.
- Code cells that need a kernel: export renders them like `quarto render`
  does; if rendering fails, the render error is reported (as for `qd render`).

## Requirements *(mandatory)*

### Functional Requirements

**Offline check (US1)**

- **FR-001**: The system MUST check a rendered deck in a real browser with all
  network access blocked, visiting every slide (including vertical slides and
  fragment steps).
- **FR-002**: The check MUST report, per problem: a stable code, the slide,
  a message and a fix hint, for: network request attempted; local file missing;
  page error; raw TeX visible; feature that cannot work offline.
- **FR-003**: Requests that load nothing by design (e.g. `about:blank`) MUST
  NOT count as network requests.
- **FR-004**: The check MUST be runnable on any rendered reveal.js HTML file,
  not only on qd exports.

**Export (US2)**

- **FR-010**: Export MUST produce one self-contained HTML file plus, in the
  same folder, every file the deck references that cannot be inlined, at the
  same relative paths.
- **FR-011**: Export MUST make math, Plotly figures and other libraries the
  deck uses load from the export itself, never from the network.
- **FR-012**: Export MUST apply the offline setup itself, at export time:
  load MathJax and (when the deck has Plotly figures) plotly.js from local
  copies, point the presentation's math plugin at nothing, and strip
  per-figure web MathJax/plotly.js references. It MUST NOT modify the deck's
  source files or the user's project configuration; a folder already set up
  per the quarto-revealjs skill MUST still export correctly.
- **FR-016**: The local libraries MUST be obtained once (pinned versions, the
  ones the quarto-revealjs skill verified: MathJax 3.2.2 full SVG build;
  plotly.js matching the deck's Python plotly when one is installed) into
  qd's cache and reused; the first export needs the network, later exports
  do not. A failed or tampered download MUST be detected (checksum) and
  reported, never used.
- **FR-013**: Export MUST run the offline check on its result and include the
  check report in its output; it MUST exit non-zero if the check fails.
- **FR-014**: Export MUST refuse (with the reason and slide) features that
  cannot be offline single-file (multiplex; chalkboard) rather than produce a
  deck that needs the network.
- **FR-015**: A failed export MUST leave any previous export intact.

**PDF (US3)**

- **FR-020**: The system MUST print the exported HTML to PDF through the
  presentation's own print mode, one page per fragment step by default
  (option: one page per slide).
- **FR-021**: Speaker notes MUST appear in the PDF only when requested, on
  separate pages.
- **FR-022**: Content marked as print-only stills MUST replace live content in
  the PDF.
- **FR-023**: The result MUST report the PDF path and page count.

**Surfaces (US4)**

- **FR-030**: Every capability MUST be a CLI command with JSON output and
  stable error/problem codes (constitution III), callable through the editor's
  existing operation API.
- **FR-031**: The editor MUST offer Export (and PDF) and show the check
  result, with problems linked to slides.
- **FR-032**: The agent Skill MUST document export, check and PDF, and the
  Skill/CLI sync test MUST cover them.

**Dependencies**

- **FR-040**: Editing commands MUST keep working when the export/check/PDF
  dependencies are missing; those commands MUST fail with an install hint.
- **FR-042**: The check and PDF MUST use an installed Google Chrome or
  Chromium when one is found (or the one named by the user). If none is
  found, qd MUST download a pinned headless Chromium into its cache on first
  use, saying so (size, destination) in its output, verifying the download,
  and reusing it afterwards. The user MUST be able to forbid the download
  (then the command fails with an install hint) and to remove the cached
  browser.
- **FR-041**: Any library qd ships or fetches MUST have a license compatible
  with personal, proprietary use (constitution I and VII); its version MUST be
  recorded.

### Key Entities

- **Export**: a folder containing one self-contained HTML deck, files that
  must ship beside it, optionally a PDF, and the check report.
- **Check report**: overall pass/fail plus a list of problems.
- **Problem**: code (e.g. network request, missing file, page error, raw TeX,
  offline-incompatible feature, notes shown to audience), severity, slide,
  message, hint, and the offending URL/path where relevant.
- **PDF print**: path, page count, options (per fragment / per slide, notes).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: On a default Quarto deck with one equation, the check reports
  the web MathJax request and raw TeX — 100% detection on the corpus of known
  offline failure modes from the quarto-revealjs guide (web MathJax, KaTeX,
  Plotly with web MathJax/plotly.js, missing background video/iframe).
- **SC-002**: Every exported deck in a test corpus (math, Plotly, fragments,
  notes, background video, embedded app) opens and works with the network
  disabled: zero network requests, zero raw TeX, zero page errors.
- **SC-003**: PDF page count equals the number of fragment steps (default) or
  slides (option) for every deck in that corpus; notes never appear unless
  requested.
- **SC-004**: Export of a 30-slide deck with math and one Plotly figure,
  including the check and the PDF, completes in under 60 seconds on a laptop.
- **SC-005**: An agent given only the Skill exports a deck, reads the check
  result and fixes one reported problem without help in at least 9 of 10
  trials.

## Assumptions

- Target is Quarto ≥ 1.4 (verified on 1.11.9, which bundles reveal.js 5.1.0)
  and the `revealjs` format only.
- The quarto-revealjs skill's findings are taken as given (raw TeX with KaTeX,
  web MathJax by default, Plotly's own MathJax, Chrome's plain print-to-PDF
  giving blank pages, show-notes in YAML showing notes to the audience);
  qd's tests re-verify the ones it relies on.
- Export output goes to a folder the user chooses (default: next to the deck
  in an `export/` folder); it is a build artefact, not deck state.
- Chalkboard and multiplex decks are out of scope for single-file export (they
  cannot work offline by design); they are reported, not exported.
- Theme/appearance of the exported deck is exactly what Quarto renders; qd adds
  no styling except print-only/print-hide support for stills.
- macOS is the primary platform; Linux should work (CI runs there, where a
  browser download or a CI-provided Chrome is used).
- The first export on a machine needs network access (libraries, and the
  browser if none is installed); every later export works offline.
