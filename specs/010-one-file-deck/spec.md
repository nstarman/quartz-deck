# Feature Specification: One-file deck — the build tools move into qd

**Feature Branch**: `010-one-file-deck`

**Created**: 2026-11-11

**Status**: Implemented

**Input**: "Make qd do everything the per-deck Python build tools of the
`quarto-revealjs` skill do (`inline_apps.py`, `make_pdf.py`,
`check_offline.py`, `check_overflow.py`, `size_report.py`, the `share`
profile), so a deck carries no tooling of its own: its Makefile only calls
`qd export` and `qd pdf`. After inlining, delete Quarto's copied resources in
the output folder."

## Context

Spec 002 gave qd an offline export (`qd export`), check (`qd check
--offline`) and PDF (`qd pdf`). Every deck made with the `quarto-revealjs`
skill still carries six Python scripts beside it, because 002 stops short of
what a real talk needs:

- **Apps.** 002 copies a background-iframe app beside the HTML. A talk's
  apps (single-file interactive figures, several MB each) should be inside
  the one file. Pandoc never inlines `data-background-iframe`, and it turns an
  `<iframe src>` into a `data:` URL, which Chrome refuses to load over 2 MB
  (the iframe shows `about:blank#blocked`) and which runs with an opaque
  origin, so many apps stay blank. The skill's answer: write
  `<iframe data-app="apps/x.html">` (pandoc leaves `data-app` alone), store
  each app gzipped in a `<script type="text/plain">`, and unzip it in the page
  (DecompressionStream) into a `blob:` URL.
- **`<object data>` and SVG `<image href>`** are never inlined by pandoc.
- **Repeated images**: pandoc embeds a file once per use, so a logo on 20
  slides is in the deck 20 times.
- **A shared copy** without speaker notes.
- **Leftovers**: in a Quarto project, `quarto render` copies the deck's
  resources (figures/, apps/…) into the output folder and never prunes them.
- **PDF size**: Chrome stores images losslessly (WebP becomes huge
  FlateDecode); recompressing through ghostscript to 200 dpi JPEG takes a
  50 MB PDF to about 12 MB.
- **Checks 002 lacks**: any missing local file (not only backgrounds),
  elements sticking out of the slide, and where the file's megabytes go.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Everything in one file (Priority: P1) 🎯 MVP

The author exports a deck whose slides use apps (`data-app` iframes and
background iframes, one over 2 MB), an `<object>` SVG, an SVG with an
`<image href>`, the same image on several slides, and math. The export
folder holds one HTML file (plus only background videos/parallax images,
which stay files), every app runs from inside it with the network blocked,
the repeated image is stored once, and Quarto's copied resources are gone.

**Independent Test**: export the fixture deck `tests/fixtures/offline/onefile.qmd`;
the export folder lists only `deck.html` (and qd's marker); the check finds no
missing file and no raw TeX, and the only network request is the one the
>2 MB app makes on purpose when it runs, which proves it ran from its `blob:` URL.

**Acceptance Scenarios**:

1. **Given** `<iframe data-app="apps/x.html">` (or `src`/`data-src` left as a
   path) or `background-iframe="apps/x.html"`, **When** exported, **Then**
   the app is stored gzipped in the HTML and loaded from a `blob:` URL when
   its slide is shown; no copy of it ships beside the HTML.
2. **Given** `<object data="f.svg">` or `<svg><image href="f.jpg">`, **When**
   exported, **Then** the file becomes a `data:` URI in the HTML.
3. **Given** a large `data:` URI used several times, **When** exported,
   **Then** it is stored once and put back into each use before reveal.js
   starts.
4. **Given** a referenced app/object/image file that does not exist,
   **When** exported, **Then** the check fails with `P_MISSING_FILE` naming
   the path, and the previous export stays.
5. **Given** a deck in a Quarto project, **When** exported, **Then** the
   export folder holds no copies of the deck's resources.

### User Story 2 — A copy to share, without notes (Priority: P2)

`qd export --strip-notes --out share` writes the same one-file deck with
every speaker-notes `<aside class="notes">` removed.

**Independent Test**: export the fixture with `--strip-notes`; the HTML has
no notes text; without the flag it has.

### User Story 3 — The check covers what the build checked (Priority: P2)

`qd check --offline` (and so `qd export`) also reports any local file the
page fails to load (`P_MISSING_FILE`), every element that sticks out of its
slide with all fragments shown (`P_OVERFLOW`, a warning), and lists the
largest embedded parts of the file (apps named by their path).

**Independent Test**: the fixture's slide with a box placed past the right
edge gives one `P_OVERFLOW` naming that slide; the report's `size.largest`
starts with the >2 MB app, named `apps/big.html`.

### User Story 4 — Smaller PDFs (Priority: P3)

When ghostscript (`gs`) is installed, `qd pdf` and `qd export --pdf`
recompress the printed PDF's images to 200 dpi JPEG (RGB), and say so in the
result. `qd pdf --notes` writes `<deck>-notes.pdf` by default, so the slides
PDF and the notes PDF sit side by side.

**Independent Test**: print the fixture with `gs` on PATH: `compressed` is
true and the page count is unchanged; with `--notes` the file is
`deck-notes.pdf`.

### User Story 5 — References from Zotero (Priority: P3)

`qd bib --zotero KEY` writes the deck's References list from a Zotero
collection straight into the `.qmd`, between `<!-- bibliography:start … -->`
and `<!-- bibliography:end -->` on the References slide (port of the skill's
`bibliography.py`). One `- {{< paper url= who= year= title= >}}` line per
item, sorted by label, equal labels getting 2026a/2026b; the link is the DOI,
or the item's URL for arXiv's 10.48550 DOIs; an item with no link is a
warning. It is an ordinary qd edit: JSON result, `--dry-run`, `--expect`,
`qd undo`, a patch of only those lines. Nothing leaves the machine (Zotero 7's
local API on localhost).

**Independent Test**: against a stand-in for Zotero's API, the list between
the markers is replaced and nothing else changes; undo restores it; a missing
pair is `E_NOT_FOUND`, two pairs `E_AMBIGUOUS`, Zotero not running `E_ZOTERO`,
and the file is untouched in each case. On the PPP talk with the real Zotero
the result is byte-identical to `bibliography.py`'s.

### Edge Cases

- An inlined library's own code mentions `<iframe src="${…}">`: script
  bodies are not searched for apps.
- The deck opens on an app's slide: reveal.js has already set the iframe's
  `src` to the path; the loader sets it to the `blob:` URL too.
- `xlink:href` is restored as `href` (SVG 2 reads both).
- `data:` URIs that hold JavaScript are never de-duplicated (scripts run
  before the restore script).
- Paths may carry `?query`/`#hash` and `%`-escapes; HTML entities in
  attributes are decoded.

## Requirements *(mandatory)*

- **FR-001**: `qd export` MUST inline apps referenced by
  `data-background-iframe`, `<iframe data-app|data-src|src>` (local paths
  only) as gzipped base64 in `<script type="text/plain">`, loaded through
  DecompressionStream into `blob:` URLs, before `Reveal.initialize`.
- **FR-002**: `qd export` MUST inline `<object data>` and SVG
  `<image href|xlink:href>` local files as `data:` URIs.
- **FR-003**: `qd export` MUST store each `data:` URI of 4000+ characters
  that occurs more than once only once, restored by a script before
  `Reveal.initialize`.
- **FR-004**: `qd export --strip-notes` MUST remove speaker notes.
- **FR-005**: The export folder MUST hold only the HTML, files that cannot be
  inlined (background video, parallax image), the PDF when asked, and qd's
  marker. Background iframes are no longer shipped as files.
- **FR-006**: Missing files referenced by FR-001/FR-002 MUST fail the export
  with `P_MISSING_FILE`; any `file:` request for a missing file during the
  check MUST be `P_MISSING_FILE`.
- **FR-007**: The check MUST report `P_OVERFLOW` (warning) for each visible
  element outside the slide box (2 px tolerance, slide coordinates, all
  fragments shown), skipping speaker notes, anything inside `.overflow-ok`
  (a deck's own footer that bleeds off the edge on purpose), SVG and MathJax internals,
  elements inside a clipping container, and invisible or empty elements.
- **FR-008**: The check report MUST include `size`: the file's bytes and its
  8 largest embedded parts (`data:` URIs and scripts over 20 000 characters;
  inlined apps named by path).
- **FR-009**: When `gs` is on PATH, PDFs MUST be recompressed (200 dpi, JPEG
  q 88, RGB) and the result MUST say `compressed: true`.
- **FR-010**: `qd pdf --notes` without `--out` MUST write `<stem>-notes.pdf`;
  `qd export --pdf --notes` likewise.
- **FR-012**: `qd bib --zotero KEY` MUST read only `localhost:23119`
  (Zotero 7's local API; attachments and notes skipped; pages of 100), accept
  only an 8-character alphanumeric key, and replace only the lines between the
  one marker pair, as an edit with `--dry-run`, `--expect` and undo. It MUST
  fail before contacting Zotero when the pair is missing or repeated.
- **FR-011**: The CLI, `qd schema`, the editor API (`strip-notes`) and the
  Skill MUST describe the new flag and fields.

## Out of scope

- WebKit (`--webkit` in the Python checks): qd drives Chrome only.
- Running the Python scripts' standalone `--root`/`-o` modes on an arbitrary
  HTML file: `qd export` builds the one file from the deck itself.
- Failing the build on overflow: it is a warning, listed in the report.
- Errors thrown inside an app's own iframe: Chrome runs such an iframe as
  its own target, and chromiumoxide 0.9 cannot enable exception events on
  it. Its network requests and missing files are caught (the interception
  moved to the browser level for this).

## Success Criteria *(mandatory)*

- **SC-001**: A deck with eleven `data-app` apps (the PPP talk, 2026-11-10)
  exports with `qd export` alone to a folder with one HTML file that passes
  the check with the network blocked.
- **SC-002**: A deck's Makefile needs only `qd export` and `qd pdf`.
