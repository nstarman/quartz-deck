# Feature Specification: Source and Run

**Feature Branch**: `004-source-and-run`

**Created**: 2026-11-04

**Status**: Draft

**Input**: User description: "Source-and-run for quartz-deck (roadmap 004): the .qmd shown as an editable source pane beside the slides, kept in sync both ways with the on-slide editor (cursor in the source shows that slide; clicking a slide element scrolls the source to its block), with markdown/YAML/code highlighting; typing in the source saves through qd with the same conflict rules as everything else and the slides re-render. Every code cell gets a Run button, plus Run all; outputs (text, tables, figures, errors) appear under the cell in the source pane and in the slides. Agents get the same through the CLI (qd run <cell>, qd run --all) with JSON results. Running code is only ever an explicit action by the person or a CLI call; the browser API never runs code because a page asked. Outputs are never written into the .qmd. One place to type the Quarto source, edit the slides directly, and run the code."

## Clarifications

### Session 2026-11-04

- Q: How does Run execute a cell? → A: Both, choosable: a live session that
  stays running (notebook-like; later cells see earlier variables) by default,
  with "Restart & run all" for a clean result, and a per-deck setting to
  always re-run the deck's code from the top instead.
- Q: Which engines? → A: Python (Jupyter) in this spec; R and Julia later in
  their own small spec.
- Q: Source pane layout? → A: A resizable side-by-side split with the slides
  by default, toggleable, plus a button for full-width source.
- Q (user direction, at planning): how are Python packages managed? → A: Each
  presentation has its own reproducible environment with a lockfile, managed
  by **uv** by default, or **pixi** with PyPI dependencies (not conda
  packages) as the alternative — never by plain `pip install` into a shared
  Python.

## Context

A Quarto deck is three things at once: markdown text, slides, and code that
produces figures and tables. Today quartz-deck covers the first two well —
the slides are an editing surface (spec 003) and every edit is a minimal
change to the `.qmd` — but the source is only reachable piecemeal (one
element at a time, or the hidden form panel), and code cells are just text:
to see what a cell produces the person must render the whole deck, and to
iterate on a figure they must leave the editor.

This feature makes the editor a single place for all three: the whole `.qmd`
beside the slides, in sync with them, and code cells that can be run where
they are written, with their output visible immediately — for the person in
the browser and for an agent through the CLI.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — See and edit the whole source beside the slides (Priority: P1)

The person opens the source pane next to the slides. It shows the entire
`.qmd` with headings, YAML front matter, markdown and code distinguished by
colour. They type anywhere in it; the change is saved and the slides
re-render, just as with any other edit.

**Why this priority**: Without the whole source in view, the other stories
have nowhere to live; it is also the most direct way for people who think in
markdown to work.

**Independent Test**: Open a deck, show the source pane, change a word in a
paragraph and a value in the YAML; the file changes exactly there, the slides
re-render with both changes, and undo restores the file byte-for-byte.

**Acceptance Scenarios**:

1. **Given** a deck, **When** the person shows the source pane, **Then** the
   entire `.qmd` is shown, front matter included, with headings, markdown,
   YAML and code visually distinguished.
2. **Given** the source pane, **When** the person changes text and pauses or
   leaves the pane, **Then** the change is saved, the slides re-render, and
   the file differs from before only where they typed.
3. **Given** the person is typing in the source pane, **When** an agent edits
   another part of the deck, **Then** the agent's change appears in the pane
   without losing the person's cursor or unsaved text.
4. **Given** the person and an agent changed the same part of the deck,
   **When** the person's change is saved, **Then** it is refused as a
   conflict and the person sees both versions to choose, as in spec 003.
5. **Given** a change in the source pane that would break the deck (e.g. an
   unclosed fence), **When** it is saved, **Then** the slides keep showing the
   last good render with the error, and the text is kept for fixing.

---

### User Story 2 — Source and slides follow each other (Priority: P1)

Moving the cursor in the source shows the slide it is in; clicking or
selecting an element on a slide scrolls the source to that element's text
and highlights it.

**Why this priority**: The value of having both views is moving between them
without searching; without sync they are two unrelated windows.

**Independent Test**: Put the cursor in the third slide's text — the preview
shows slide 3; click a list on slide 5 — the source scrolls to that list
with it highlighted.

**Acceptance Scenarios**:

1. **Given** the source pane, **When** the cursor moves into a slide's text,
   **Then** the preview shows that slide within half a second.
2. **Given** an element selected or edited on a slide, **When** the source pane
   is open, **Then** the pane scrolls to and highlights that element's text.
3. **Given** the person is typing in the source pane, **When** the preview
   follows, **Then** keyboard focus stays in the source pane.

---

### User Story 3 — Run a code cell and see its output (Priority: P1)

Each code cell in the source pane (and on the slide) has a Run button. Running
it shows the output — printed text, a table, a figure, or an error — directly
under the cell in the source pane, and the slide shows the new output.

**Why this priority**: Running code where it is written is the core new
capability; it turns the editor into a place to iterate on figures and
results, not only on words.

**Independent Test**: In a deck with a Python cell that prints a value and
another that draws a plot, run each: the printed value and the figure appear
under the cells and on their slides; change the plot code, run again, the
figure updates; the `.qmd` is unchanged by running.

**Acceptance Scenarios**:

1. **Given** a code cell, **When** the person presses its Run button, **Then**
   the cell's output appears under it, and the slide containing the cell
   shows that output.
2. **Given** a cell that raises an error, **When** it is run, **Then** the
   error and its traceback are shown under the cell and nothing else breaks.
3. **Given** a cell that runs for a long time, **When** the person presses
   Stop, **Then** the run is interrupted and they are told so.
4. **Given** any run, **When** it finishes, **Then** the `.qmd` file is
   byte-identical to before (outputs are never written into it).
5. **Given** "Run all", **When** pressed, **Then** every cell runs in document
   order and stops at the first error, showing which cell failed.

---

### User Story 4 — Agents run code through the CLI (Priority: P2)

An agent runs one cell or all cells with `qd run` and gets JSON describing
each output (text, tables, figures saved as files, errors), so it can check a
result or fix a failing cell — and the person sees the same outputs in the
browser.

**Why this priority**: Agents are first-class users (constitution III); an
agent asked to "fix the plot on slide 4" needs to run it and read the result.

**Independent Test**: `qd run results/1` on a deck returns the cell's output as
JSON (with a path to a saved figure), the browser's source pane shows the same
output, and `qd run --all` reports the first failing cell.

**Acceptance Scenarios**:

1. **Given** a cell reference, **When** an agent runs `qd run <cell>`, **Then**
   the result lists each output with its kind (text, table, figure, error) and
   content or file path.
2. **Given** an agent run, **When** the person has the editor open, **Then**
   the new output appears under the cell in their source pane.
3. **Given** a request to run code that comes from a web page through the
   editor's local connection, **When** it is not the person's own click in
   the editor, **Then** no code runs.

---

### User Story 5 — Outputs persist between sessions (Priority: P3)

Outputs from the last run of each cell are kept (outside the `.qmd`) and shown
again the next time the deck is opened, marked as stale when the cell's code
has changed since.

**Why this priority**: Re-running expensive cells every session is
frustrating; but correctness comes first, so stale outputs are flagged.

**Independent Test**: Run a cell, stop the editor, reopen it: the output is
there; edit the cell: it is marked stale until run again.

**Acceptance Scenarios**:

1. **Given** a cell that was run, **When** the deck is reopened, **Then** its
   last output is shown, without running anything.
2. **Given** a cell whose code changed since its output was produced, **When**
   it is shown, **Then** the output is marked stale.

---

### Edge Cases

- Cells in languages the deck's engine can't run (e.g. an R cell in a Python
  deck) show a clear "cannot run here" message rather than failing silently.
- Cells with options (`#| echo: false`, `#| fig-cap:`, `#| eval: false`):
  `eval: false` cells are not run by Run all; the options are respected when
  showing output on the slide.
- A deck with no code cells: the source pane works; Run controls are absent.
- No kernel/engine installed (e.g. Jupyter missing): Run explains what to
  install; editing still works.
- Output that is very large (thousands of lines, huge images): shown
  truncated with a way to see the full output.
- Interactive outputs (Plotly, widgets): shown on the slide as in the
  rendered deck; in the source pane at least as a static preview.
- A cell that never finishes or asks for input: Stop interrupts it; reading
  from standard input is reported as unsupported.
- An external editor (e.g. VS Code) changes the file while the source pane
  has unsaved text: handled like any concurrent change (no silent loss).
- Very long decks (hundreds of slides): the source pane stays responsive.

## Requirements *(mandatory)*

### Functional Requirements

**Source pane (US1, US2)**

- **FR-001**: The editor MUST offer a source pane showing the entire `.qmd`,
  with front matter, headings, markdown, fenced divs and code cells visually
  distinguished.
- **FR-002**: Text typed in the source pane MUST be saved through the same
  editing operations as all other edits, writing only the changed text, with
  undo restoring the file byte-for-byte.
- **FR-003**: Saves from the source pane MUST be refused, not applied, when the
  affected part of the file changed elsewhere since it was read, keeping the
  person's text and showing both versions (as spec 003).
- **FR-004**: Changes made elsewhere MUST appear in the source pane without
  discarding unsaved text or moving the person's cursor.
- **FR-005**: Moving the cursor into a slide's text MUST show that slide;
  selecting or editing an element on a slide MUST scroll the source pane to
  and highlight that element's text.
- **FR-006**: The source pane MUST appear as a resizable side-by-side split
  with the slides by default, MUST be toggleable, and MUST offer a full-width
  mode; the layout is remembered per browser.

**Running code (US3, US5)**

- **FR-007**: Every code cell MUST have a Run control in the source pane and on
  its slide; there MUST be Run all and Stop.
- **FR-008**: Running a cell MUST show its outputs (text, tables, figures,
  errors with traceback) under the cell in the source pane and update the
  slide that contains it.
- **FR-008a**: By default cells run in a live session per deck that keeps state
  between runs; the person MUST be able to restart the session and to
  "Restart & run all" for a clean, render-faithful result. Outputs produced
  out of document order MUST be marked (e.g. run number) so order effects are
  visible.
- **FR-008b**: A per-deck setting MUST switch Run to "re-run from the top":
  each Run executes all cells up to and including the chosen one in a fresh
  session, so the result always matches a full render.
- **FR-009**: Running code MUST never change the `.qmd`.
- **FR-010**: Run all MUST run cells in document order, skip `eval: false`
  cells, and stop at the first error, reporting the failing cell.
- **FR-011**: Outputs MUST be kept outside the `.qmd` and shown again when the
  deck is reopened, marked stale when the cell's code has changed since they
  were produced.
- **FR-012**: Python cells (Jupyter engine) MUST be runnable; cells in other
  languages MUST show "cannot run here yet" (R, Julia and Observable are a
  later spec).

**Reproducible environments (all stories)**

- **FR-017**: Each deck MUST run its code in its own environment described by
  a manifest and a lockfile kept next to the deck (or at its Quarto project
  root): uv (`pyproject.toml` + `uv.lock`) by default, or pixi (`pixi.toml` /
  `pyproject.toml` + `pixi.lock`, with PyPI dependencies) when the deck
  already uses pixi.
- **FR-018**: A deck without an environment MUST be offered one in a single
  step (editor button or `qd env init`), created with the packages Quarto and
  Run need, and locked; qd never installs into the system or a shared Python.
- **FR-019**: Adding a package (editor or `qd env add <pkg>`) MUST update the
  manifest and the lockfile; running code and rendering MUST use exactly the
  locked versions and MUST refuse, with an explanation, when the lockfile is
  out of date with the manifest.
- **FR-020**: Rendering, preview, export and Run MUST all use the deck's
  environment, so the output on screen, in the export and on a colleague's
  machine (after one sync step) come from the same package versions.

**Agents and safety (US4)**

- **FR-013**: `qd run <cell>` and `qd run --all` MUST run code and return JSON
  listing each output's kind and content or saved-file path; figures are saved
  as image files next to qd's other cached files.
- **FR-014**: Outputs produced by an agent's run MUST appear in the person's
  open editor, and vice versa.
- **FR-015**: Code MUST run only on an explicit action: the person's click or
  key press in the editor, or a `qd run` command. The editor's local
  connection MUST reject run requests that do not come from the editor page
  itself, and no other page or link may cause code to run.
- **FR-016**: Long runs MUST be interruptible (Stop / a CLI timeout), and the
  editor MUST stay usable while code runs.

### Key Entities

- **Environment**: the deck's manifest + lockfile and the tool that manages
  them (uv or pixi), the folder they live in, and whether the lock is current.
- **Code cell**: a fenced code block with an engine language (e.g.
  `{python}`), its options (`#|` lines), its source text and a fingerprint of
  that text.
- **Run**: one execution of one or more cells — who started it (person or
  agent), when, status (running, done, error, stopped).
- **Output**: what a cell produced on its last run — ordered items of kind
  text, table, figure (image file), error — tied to the cell fingerprint it
  was produced from (stale when it no longer matches).
- **Source view state**: cursor position, scroll, unsaved text, and the
  slide/element it is linked to (browser only).

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: From the source pane, a person changes text in three different
  slides and the YAML title without using the slide view, each saved in under
  2 seconds and undoable to identical bytes.
- **SC-002**: Moving the cursor between slides in the source shows the
  matching slide within 0.5 s in 95% of moves; clicking a slide element brings
  its text into view within 0.5 s.
- **SC-003**: Running a short cell (prints a value, < 1 s of compute) shows its
  output under the cell within 3 s of pressing Run (warm engine); a figure
  cell's image appears on its slide within 5 s.
- **SC-004**: Over a corpus of decks with code, running cells never changes
  the `.qmd` (100% byte-identical before/after).
- **SC-005**: An agent asked to fix a failing cell does so with `qd run` and
  `qd edit` in at most 6 commands, without usage errors, in 9 of 10 trials.
- **SC-007**: On a second machine with only uv and Quarto installed, a deck's
  environment is recreated from its lockfile in one command and renders with
  identical package versions (100% of locked packages match).
- **SC-006**: No code runs in a test where another local web page tries to
  trigger a run through the editor's connection (0 of all attempts).

## Assumptions

- Code execution uses a Jupyter kernel inside the deck's own uv (or pixi)
  environment; qd needs `uv` (or `pixi`) installed, not a global Jupyter.
  qd does not bundle Python.
- The final rendered deck is still produced by Quarto's own render; outputs
  shown in the editor are a working view and may differ from a full render if
  cells depend on run order (see FR-008).
- Code runs with the person's own permissions on their machine — the same as
  `quarto render`. Running a deck's code is as trustworthy as the deck.
- Specs 001–003 are the base: their operations, conflict rules, undo history,
  live preview and on-slide editing are reused unchanged.
- One person edits in the browser at a time; agents use the CLI.
- Highlighting covers markdown, YAML and the cell languages; full IDE
  features (autocomplete, linting) are out of scope.
