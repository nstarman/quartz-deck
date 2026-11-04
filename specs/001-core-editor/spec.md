# Feature Specification: Core Editor — Quarto reveal.js GUI + LLM-Friendly CLI

**Feature Branch**: `001-core-editor`

**Created**: 2026-11-03

**Status**: Draft

**Input**: User description: "A local GUI editor, built in Rust, for building Quarto ↔ reveal.js presentations. Clean-room (no slides.com code). A clear CLI and Skills designed for LLMs, with a live browser preview so an LLM like Claude can edit a deck in-session through the CLI and see the result."

## Clarifications

### Session 2026-11-03

- Q: What is the source of truth? → A: The `.qmd` file. GUI and CLI write minimal edits back to it; reveal.js HTML is only render output.
- Q: Direction of conversion? → A: Quarto → reveal.js rendering only. Importing existing reveal.js HTML decks is out of scope.
- Q: Visual editing scope for v1? → A: Slide structure and text editing **and** free drag/resize of positioned boxes.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — LLM edits a deck through the CLI (Priority: P1)

An LLM agent (e.g. Claude Code) is asked "add a slide after the intro
summarizing the results, and retitle slide 3". It lists the deck's slides,
reads the ones it needs, applies edits with CLI commands, validates the
deck, and reports back — all without a GUI and without rewriting the file by
hand.

**Why this priority**: This is the smallest slice that delivers value on its
own and it defines the edit operations everything else reuses. The GUI is a
second client of the same operations.

**Independent Test**: On a sample `.qmd`, run the list/show/add/move/remove/
set commands; check the JSON output, check that the file contains exactly the
intended change (diff limited to the edited region), and that `quarto render`
still succeeds.

**Acceptance Scenarios**:

1. **Given** a deck with 5 slides, **When** the agent lists slides, **Then**
   it receives JSON with each slide's stable ID, position, title, and a short
   content summary.
2. **Given** a deck, **When** the agent adds a slide after slide `intro`
   with a title and markdown body, **Then** the new slide appears in that
   position, gets a stable ID, and no other bytes of the file change.
3. **Given** a deck, **When** the agent moves, removes, or retitles a slide,
   or changes a slide attribute (e.g. background), **Then** only that slide's
   text changes.
4. **Given** an invalid command (unknown slide ID, bad attribute), **When**
   it is run, **Then** the CLI exits non-zero with a stable error code and a
   message saying how to fix it, and the file is unchanged.
5. **Given** an agent with no prior knowledge of the tool, **When** it
   reads the bundled Skill or the self-describing command schema, **Then** it
   can complete scenarios 1–3 without guessing syntax.

---

### User Story 2 — Live browser preview of the deck (Priority: P2)

The user (or agent) starts the editor for a deck. A browser page shows the
rendered reveal.js presentation and updates by itself whenever the `.qmd`
changes — whether the change came from the CLI, the GUI, or an external text
editor. An agent can point its browser tool at the same URL to check its
work.

**Why this priority**: It closes the loop for both humans and LLMs: edits
become visible without manual re-rendering.

**Independent Test**: Start the editor, open the URL, edit the file with
the CLI and with a plain text editor, and observe the preview update to the
edited slide each time.

**Acceptance Scenarios**:

1. **Given** a deck, **When** the user starts the editor, **Then** it
   prints (and in JSON mode returns) a localhost URL, and that URL shows the
   rendered deck.
2. **Given** the preview is open on slide 4, **When** slide 4 is edited
   from any source, **Then** the preview shows the change and stays on
   slide 4.
3. **Given** the edit makes the deck fail to render, **When** rendering
   fails, **Then** the preview keeps the last good render and shows the
   render error; the CLI `check` command reports the same error.
4. **Given** the editor is running, **When** the agent asks the CLI for the
   preview URL of a specific slide, **Then** it gets a URL that opens on that
   slide.

---

### User Story 3 — Structural and text editing in the GUI (Priority: P3)

The user works in the browser editor: a slide list on one side (thumbnails
or titles), the live preview in the middle, a properties panel on the other
side. They reorder slides by dragging in the list, add/duplicate/delete
slides, click an element in the preview to select it, edit its text, and
change slide and element properties in the panel.

**Why this priority**: The main human workflow, built on top of US1's edit
operations and US2's preview.

**Independent Test**: Through the GUI only, reorder, add, delete, and
retitle slides and edit a paragraph; verify the `.qmd` diff matches what the
equivalent CLI commands produce.

**Acceptance Scenarios**:

1. **Given** the editor is open, **When** the user drags slide 5 above
   slide 2 in the slide list, **Then** the `.qmd` is reordered the same way
   `move` would do it, and the preview updates.
2. **Given** a selected text element, **When** the user edits its text and
   confirms, **Then** the corresponding markdown in the `.qmd` is updated
   and nothing else changes.
3. **Given** any GUI action, **When** it completes, **Then** the same change
   could have been made with a single CLI command (the GUI exposes no
   capability the CLI lacks).
4. **Given** a GUI change, **When** the user undoes it, **Then** the file
   returns to its previous content.

---

### User Story 4 — Drag and resize positioned boxes (Priority: P4)

On a slide, the user adds a positioned box (text or image), drags it
anywhere on the slide canvas, and resizes it with handles. Positions are
stored in standard Quarto absolute-positioning syntax, so the deck renders
the same with stock Quarto and no editor.

**Why this priority**: Requested for v1, but depends on US1–US3, and is the
riskiest UI work.

**Independent Test**: Add a box, drag it, resize it, then render the
`.qmd` with plain `quarto render` and confirm the box's position and size
match what the editor showed.

**Acceptance Scenarios**:

1. **Given** a slide, **When** the user adds an image box and drags it,
   **Then** the box follows the pointer smoothly and on release the `.qmd`
   records its new position.
2. **Given** a box, **When** the user resizes it with a corner handle,
   **Then** width and height are written back; aspect ratio is kept when a
   modifier key is held.
3. **Given** a box, **When** the agent runs the CLI to move or resize it by
   ID, **Then** the GUI shows the box at the new position.
4. **Given** positions are written, **When** the deck is rendered with
   stock Quarto, **Then** boxes appear where the editor showed them (within
   1 px at the deck's native size).

---

### User Story 5 — Render and export (Priority: P5)

The user or agent renders the deck to its final reveal.js HTML (and any
other format Quarto supports for the document) from the CLI or GUI.

**Why this priority**: Thin wrapper over Quarto, but completes the workflow.

**Independent Test**: Run the render command; the output matches what
`quarto render` produces for the same file.

**Acceptance Scenarios**:

1. **Given** a deck, **When** the user renders it, **Then** the output
   path is reported (JSON) and opens as a working reveal.js presentation.
2. **Given** Quarto is not installed, **When** any command that needs it
   runs, **Then** it fails with a clear error explaining how to install it;
   commands that only edit the `.qmd` still work.

### Edge Cases

- Hand-written decks with unusual but valid syntax (slides from `#` vs `##`
  headings, `---` separators, nested fenced divs, code cells, raw HTML,
  includes, YAML-only title slide): the tool must read them, edit them
  safely, or refuse an edit it cannot make safely — never corrupt them.
- Slides without IDs: the tool assigns IDs on first edit (writing `{#id}` to
  that slide only) and reports them; IDs stay stable across later edits.
- Duplicate or clashing IDs: reported by `check`; edits that address an
  ambiguous ID are refused.
- Concurrent edits: the file changes between read and write (CLI and GUI at
  once, or an external editor) → the edit is refused with a "file changed"
  error rather than overwriting.
- Computational cells (R/Python/Julia): rendering may be slow or fail; the
  preview stays responsive and shows the last good render.
- Missing images or includes: shown as warnings, not crashes.
- Very large decks (100+ slides): list and edit commands remain fast.
- Editor started on a file that does not exist: offer to create a minimal
  reveal.js deck.
- Multi-file Quarto projects (`_quarto.yml`): the deck renders with its
  project settings.

## Requirements *(mandatory)*

### Functional Requirements

**Document and edits**

- **FR-001**: The system MUST treat the `.qmd` file on disk as the only
  persistent state of a deck.
- **FR-002**: The system MUST parse a deck into slides and slide elements
  (headings, text blocks, images, code, positioned boxes), each with a stable
  ID and its exact source location.
- **FR-003**: Every edit MUST change only the source text of the edited
  item; all other bytes MUST be preserved. Reading and writing an unedited
  deck MUST be byte-identical.
- **FR-004**: The system MUST support these operations: list slides; show a
  slide; add, duplicate, move, remove a slide; set slide title, body, and
  attributes (e.g. background, classes, transitions); edit an element's text;
  add, move, resize, and remove positioned boxes; edit document-level
  options (front matter); undo the last edit.
- **FR-005**: Positioned boxes MUST be stored in standard Quarto
  absolute-positioning syntax so stock Quarto renders them identically.
- **FR-006**: Writes MUST be atomic and MUST detect if the file changed since
  it was read; in that case the edit is refused with a distinct error.
- **FR-007**: The system MUST validate a deck (`check`): syntax problems,
  duplicate IDs, missing resources, and Quarto render errors.

**CLI and LLM interface**

- **FR-010**: Every capability in FR-004/FR-007 and every GUI action MUST be
  available as a non-interactive CLI command.
- **FR-011**: CLI output MUST be JSON by default with a documented, stable
  shape; a human-readable format MUST be available by flag.
- **FR-012**: Errors MUST go to stderr as JSON with a stable error code, a
  message, and a hint; the exit code MUST be non-zero.
- **FR-013**: The CLI MUST describe itself: a command that prints all
  commands, arguments, and output shapes as machine-readable schema.
- **FR-014**: The project MUST ship an agent Skill that teaches the
  inspect → edit → check → view-preview loop, with worked examples, and is
  kept in sync with the CLI.
- **FR-015**: Edit commands MUST support a dry-run mode that returns the
  diff without writing.

**Preview and GUI**

- **FR-020**: The system MUST serve an editor UI and live preview on a
  localhost-only address, usable in any modern browser.
- **FR-021**: The preview MUST update automatically after any change to the
  `.qmd`, keep the current slide, and keep the last good render if a render
  fails while showing the error.
- **FR-022**: The GUI MUST provide: slide list with reorder by drag,
  add/duplicate/delete; click-to-select elements in the preview; text
  editing; a properties panel for slide/element attributes; drag/resize of
  positioned boxes; undo/redo.
- **FR-023**: The CLI MUST be able to report the running editor's URL,
  including a URL for a specific slide.
- **FR-024**: Preview output MUST be produced by Quarto so it matches the
  final render.

**Render**

- **FR-030**: The system MUST render the deck through Quarto and report the
  output location.
- **FR-031**: When Quarto is unavailable, render/preview/check MUST fail
  with installation guidance; pure edit commands MUST keep working.

**Legal**

- **FR-040**: The implementation MUST follow `CLEANROOM.md` (no slides.com
  code, assets, formats, or design).
- **FR-041**: All dependencies MUST have licenses compatible with a
  proprietary, personal-use project and be listed in the plan.

### Key Entities

- **Deck**: one `.qmd` file — front matter (document options) plus an
  ordered list of slides. Identified by its file path.
- **Slide**: a section of the deck starting at a slide heading or
  separator; has a stable ID, title (optional), attributes, and elements.
- **Element**: a piece of slide content — text block, image, code block,
  or positioned box — with an ID (assigned when first addressed) and source
  location.
- **Positioned Box**: an element with position and size (top, left, width,
  height) on the slide canvas, holding text or an image.
- **Edit**: one operation on a deck; has a before/after text, a diff, and
  can be undone.
- **Editor Session**: a running preview/GUI server for one deck, with a URL.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: An LLM agent given only the Skill completes a 5-step edit task
  (add, move, retitle, set background, add positioned image) with zero
  syntax errors on its first attempt in at least 9 of 10 trials.
- **SC-002**: Round-trip of a corpus of at least 20 real-world Quarto
  reveal.js decks without edits is byte-identical for 100% of them.
- **SC-003**: For any single edit, the resulting diff touches only the
  edited slide (verified across the corpus).
- **SC-004**: CLI edit commands on a 100-slide deck complete in under
  200 ms (excluding Quarto rendering).
- **SC-005**: For decks without computational cells, the preview shows a
  change within 3 seconds of the file being saved.
- **SC-006**: Dragging a box tracks the pointer at display frame rate with
  no visible lag, and the stored position matches the stock-Quarto render
  within 1 px.
- **SC-007**: No change made through the editor ever loses user content:
  zero data-loss bugs across concurrent CLI/GUI/external-editor tests.

## Assumptions

- Single user, local machine, one deck per editor session; no
  collaboration, accounts, or cloud features.
- The Quarto CLI is installed separately by the user (it bundles Pandoc
  and reveal.js).
- Target is Quarto's `revealjs` format; other formats are rendered but not
  visually edited.
- Importing existing reveal.js HTML decks is out of scope.
- Theme editing beyond setting Quarto's existing options is out of scope
  for v1.
- Rich WYSIWYG editing of complex content (tables, code cells, math) is
  out of scope for v1; such content is edited as markdown text.
- macOS is the primary platform; Linux should work; Windows is not a v1
  target.
