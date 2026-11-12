# Feature Specification: Run on the slide

**Feature Branch**: `013-run-on-slide`

**Created**: 2026-11-12

**Status**: Draft

**Input**: User description: "Run on the slide (roadmap 013): run code where you see it, on the slide, with whatever engine Quarto uses for the deck. Today (004): each cell has a ▶ in the source pane, ⇧Enter runs the cell under the caret, Run all / Stop / Restart sit in the source bar; on the slide a cell runs only from its right-click menu ("Run cell"), and outputs do show on the slide. (P1) On the slide: a ▶ on each code cell in the preview (shown on hover and when the cell is selected, reachable from the keyboard), with the cell's state (running, done, error, stale) and its output updating in place on the slide. Same rules as 004: code runs only on the person's own click or key, with the session token; nothing is written into the .qmd. (P1) In the source pop-up: editing a cell's code over the slide (the box that opens on a code cell, today with only "⌘Enter or Esc saves" and Cancel) gets a Run button and ⇧Enter, as in the source pane: it saves the edit, runs the cell and shows the new output (a re-drawn plot) on the slide, so a plot can be changed and seen without leaving the slide. (P2) Quarto's engines: today qd runs Python through its own kernel bridge in the deck's locked environment. Run should use the engine Quarto picks for the deck (engine:/jupyter:/knitr in the front matter): Jupyter kernels (Python, Julia, others) and knitr (R), so a cell runs here the way quarto render would run it. Open questions: how to run one cell with Quarto's backend (Quarto has no per-cell API: a live Jupyter kernel for Jupyter decks, a persistent R session for knitr, or re-render with freeze and pick out the cell's output); environments for R and Julia (004 locks Python with uv/pixi); whether the presented deck (/present/, no editor) ever gets Run (it has no session token, so probably not); outputs Quarto makes that a kernel doesn't (cross-references, figure captions, layout options)."

## Clarifications

### Session 2026-11-12

- Q: Which environment do R and Julia cells run in? → A: the person's own
  installed R or Julia and packages, the same ones `quarto render` uses. qd
  says what is missing and how to install it. Locked per-deck environments
  for R and Julia (as 004 does for Python) are left for a later spec.
- Q: Which of Quarto's engines does Run cover? → A: all of them, eventually,
  in this order: first Jupyter kernels (any language) and knitr (US3), then
  Observable JS (US4), then Quarto's native Julia engine (US5).

## Context

Spec 004 made code cells runnable from the source pane. Each cell has a ▶ in
the pane's gutter, ⇧Enter runs the cell under the caret, and Run all, Stop
and Restart sit in the source bar. The outputs show on the slides. On the
slide itself, though, a cell runs only from its right-click menu, and editing
a cell's code over the slide (the source pop-up) can only save or cancel. So
changing a plot means opening the source pane, finding the cell, running it,
and looking back at the slide. Only Python runs. A deck written for R, or for
a Jupyter kernel other than Python, shows its code but can't run it here,
even though `quarto render` runs it fine.

This feature puts Run where the code is. Each cell on the slide gets a ▶ and
shows whether its output is current. The source pop-up runs what it saves,
so a plot can be changed and seen without leaving the slide. And a cell runs
with the engine Quarto uses for the deck, so R and other Jupyter languages
work as well as Python.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Run a cell from the slide (Priority: P1)

The person points at a code cell on the slide, or selects it, and a ▶ appears
on it. Clicking ▶, or pressing the run key while the cell is selected, runs
that cell. While it runs, the cell shows that it is running. When it ends,
the cell's output on the slide is replaced by the new one: a plot, a table,
printed text or an error. A cell whose code changed since its output was made
is marked stale until it runs again.

**Why this priority**: This is the core of the feature. A talk with code
shows its results on the slides, and the slide is where the person looks.
Today they must leave it to run anything.

**Independent Test**: Open a deck with a plotting cell whose stored output is
stale. Hover the cell, click ▶, and see the cell marked running, then the new
plot in place on the slide with the stale mark gone, without opening the
source pane.

**Acceptance Scenarios**:

1. **Given** a slide with a code cell, **When** the person hovers or selects
   the cell, **Then** a ▶ shows on it. It can be reached and pressed from the
   keyboard (focus on the cell, then the run key), and it has an accessible
   name.
2. **Given** the person presses ▶, **When** the cell runs, **Then** the cell
   shows "running" until it ends, and then its output on the slide is the new
   one, in the same place, without the slide jumping to another slide.
3. **Given** a cell whose code changed since its last output, **When** the
   slide shows it, **Then** the output is marked stale until the cell runs
   again.
4. **Given** a cell that raises an error, **When** it ends, **Then** the error
   shows where the output goes, marked as an error, and the deck file is
   unchanged.
5. **Given** a cell is running, **When** the person presses Stop on that cell,
   **Then** it stops and is marked as stopped.
6. **Given** the deck is opened without the session token (a plain link, or
   the presented deck), **When** a cell is shown, **Then** it has no ▶. Code
   never runs except on the person's own click or key in the editor.

---

### User Story 2 — Change the code and see the result, on the slide (Priority: P1)

The person opens a cell's code over the slide (the source pop-up), changes it,
and presses **Run** (or ⇧Enter). The change is saved to the deck as an
ordinary edit, the cell runs, and the new output (a re-drawn plot) appears on
the slide beside the pop-up. The pop-up stays open, so the person can change
the code again and run again until the plot looks right.

**Why this priority**: Tuning a figure (a colour, an axis limit, a label) is
the most common reason to touch code in a talk. Doing it in a loop on the
slide is what makes the editor worth using for code.

**Independent Test**: Open the pop-up on a plotting cell, change the line
colour, press ⇧Enter, and see the deck file updated and the slide's plot
re-drawn in the new colour, with the pop-up still open.

**Acceptance Scenarios**:

1. **Given** the source pop-up is open on a code cell, **When** the person
   looks at its bar, **Then** it shows **Run** with its key (⇧Enter) next to
   the existing save hint and Cancel.
2. **Given** a changed cell, **When** the person presses Run, **Then** the
   edit is saved (one undo step, conflict-checked as any edit), the cell runs,
   and the new output appears on the slide while the pop-up stays open.
3. **Given** an unchanged cell, **When** the person presses Run, **Then** the
   cell runs and nothing is written to the deck.
4. **Given** the code can't be saved (the file changed underneath, or the
   cell is gone), **When** the person presses Run, **Then** nothing runs and
   the pop-up says why, keeping the person's text.
5. **Given** the person presses Run again while the cell is still running,
   **Then** the new run waits for the current one or replaces it, and the
   slide never shows the older output after the newer one.

---

### User Story 3 — Run cells with the deck's own engine (Priority: P2)

A deck written for R (knitr), or for a Jupyter kernel such as Julia, runs here
the way `quarto render` runs it. qd picks the engine the way Quarto does,
from the deck's front matter and its cells. A cell's outputs on the slide
are what that engine produces. A Python deck keeps working as it does today.

**Why this priority**: Without it the first two stories serve Python decks
only. It comes second because it needs per-language setup, and Python decks
work without it.

**Independent Test**: Open a deck with `engine: knitr` and an R cell that
draws a plot. Run the cell from the slide and see the plot. Do the same with
a Julia deck that uses a Jupyter kernel.

**Acceptance Scenarios**:

1. **Given** a deck whose engine is knitr, **When** an R cell runs, **Then**
   it runs in a session that keeps its state between runs (a variable set in
   one cell is there for the next), and its printed and plotted output shows
   on the slide.
2. **Given** a deck whose engine is a Jupyter kernel (`jupyter: julia-1.10`,
   say), **When** a cell runs, **Then** it runs in that kernel.
3. **Given** a deck whose engine, kernel or language isn't installed,
   **When** the person presses ▶, **Then** nothing runs and qd says what is
   missing and how to install it, in the same way 004 says Python's
   environment is missing.
4. **Given** a Python deck with a locked environment (004), **When** its
   cells run, **Then** they run in that environment, exactly as today.
5. **Given** any engine, **When** cells run, **Then** **Run all**, **Stop**
   and **Restart** in the source bar work the same way they do for Python.

---

### User Story 4 — Observable JS cells (Priority: P3)

A deck's `{ojs}` cells run in the browser, not in a kernel, and the preview
already evaluates them. They get the same ▶ and state on the slide. Pressing
▶ evaluates that cell again (and the cells that depend on it), so the person
can re-run an interactive figure after changing its code in the pop-up.

**Why this priority**: OJS cells already work in the preview, so this only
makes them behave like the other cells. It comes after the kernel engines,
which don't work at all today.

**Independent Test**: Open a deck with an `{ojs}` cell, change its code in
the pop-up, press ⇧Enter, and see the new result on the slide.

**Acceptance Scenarios**:

1. **Given** an `{ojs}` cell, **When** the person presses ▶, **Then** the cell
   and the cells that depend on it are evaluated again on the slide, with no
   kernel started.
2. **Given** an `{ojs}` cell with an error, **When** it runs, **Then** the
   error shows where its output goes, as for other cells.

---

### User Story 5 — Quarto's native Julia engine (Priority: P4)

A deck with `engine: julia` (Quarto's own Julia engine, without Jupyter)
runs its cells here the way `quarto render` runs them, in a Julia session
that keeps state between runs.

**Why this priority**: Julia decks already run through a Jupyter kernel
(US3). The native engine is newer and used by fewer decks, so it comes last.

**Independent Test**: Open a deck with `engine: julia` and three dependent
cells, run them one by one from the slides, and get the outputs
`quarto render` gives.

**Acceptance Scenarios**:

1. **Given** a deck with `engine: julia`, **When** a cell runs, **Then** it
   runs in a persistent Julia session, not a Jupyter kernel.
2. **Given** Julia is not installed, **When** the person presses ▶, **Then**
   nothing runs and qd says how to install it.

---

### Edge Cases

- A cell's output is large (a big table, a long log). The slide shows it as
  the final render would, cut off the same way 004 cuts off long outputs.
- A cell has no output (an import, an assignment). The run ends with "done"
  and the slide shows nothing below the code, as before.
- A cell is hidden on the slide (`echo: false`): only its output shows. Its ▶
  sits on the output instead.
- A cell runs on a slide the person leaves before it ends. The run goes on;
  the result is there when they come back, and the slide list doesn't jump.
- The deck is edited in the source pane or by an agent while a cell runs.
  The output is matched to the cell by its identity, and if the cell's code
  changed meanwhile, the output is marked stale.
- Cells depend on earlier cells. Running one cell alone uses the session's
  current state. If that state is missing (a fresh session), the cell fails,
  and the error says so. **Run all** from the top fixes it.
- Quarto adds things a single run doesn't: figure captions, cross-reference
  numbers, layout options (`layout-ncol`). The slide shows the cell's own
  outputs, laid out as the preview already lays them out. Captions and
  numbers come back with the next full render.
- Two editors are open on the same deck. Both see the new output; the run
  happens once.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: Each runnable code cell on a slide MUST show a ▶ when it is
  hovered or selected. The ▶ MUST be focusable and named for assistive
  technology, and the run key MUST run the selected cell.
- **FR-002**: Running a cell from the slide MUST show its state on the slide:
  running, done, error, stopped, or stale when its code changed since its
  output was made.
- **FR-003**: When a run ends, the cell's output on the slide MUST be replaced
  in place, without re-rendering the whole deck, moving to another slide or
  losing the selection.
- **FR-004**: A running cell MUST be stoppable from the slide.
- **FR-005**: The source pop-up on a code cell MUST offer **Run** (button and
  ⇧Enter). It saves the edit as an ordinary qd edit (undoable,
  conflict-checked), then runs the cell and keeps the pop-up open.
- **FR-006**: If the save fails, the run MUST NOT happen, and the pop-up MUST
  keep the person's text and say why.
- **FR-007**: The slide MUST never show an older run's output after a newer
  one for the same cell.
- **FR-008**: Code MUST run only on the person's own click or key in the
  editor, and only with the session token. A view without the token (the
  presented deck, a plain link) MUST NOT show ▶ and MUST NOT run code.
- **FR-009**: Running MUST NOT write outputs into the `.qmd`. Outputs are
  stored beside it, as in 004.
- **FR-010**: Cells MUST run with the engine Quarto would use for the deck:
  the deck's `engine:` or `jupyter:` front matter, otherwise Quarto's own
  choice from the cells' languages.
- **FR-011**: Jupyter decks MUST run in the named kernel. Python keeps the
  deck's locked environment (004).
- **FR-012**: knitr decks MUST run R in one session per deck that keeps state
  between runs until Restart.
- **FR-013**: When the engine, kernel or language is missing, ▶ MUST say what
  is missing and how to install it, and run nothing.
- **FR-014**: `qd run` (004's CLI) MUST use the same engine choice, so an
  agent's run and the person's run agree.
- **FR-015**: Run, Run all, Stop and Restart MUST be actions in the 007
  action list, so the menus, the shortcut reference and `qd actions` show
  them.
- **FR-016**: R and Julia cells MUST run with the person's own installed R
  or Julia and packages, the ones `quarto render` uses. qd doesn't lock
  environments for them in this feature.
- **FR-017**: Run MUST cover Jupyter kernels and knitr (US3), then
  Observable JS cells (US4), then Quarto's native Julia engine (US5), in that
  order of delivery.
- **FR-018**: Observable JS cells MUST run in the browser without a kernel.
  Running one evaluates it again together with the cells that depend on it.

### Key Entities

- **Code cell**: an executable fenced block (```` ```{python} ````, ```` ```{r} ````, …)
  with an identity in the deck, its language and its text.
- **Run**: one execution of one cell: its state (queued, running, done,
  error, stopped), when it started, and the outputs it produced.
- **Stored output**: the last run's outputs for a cell, with a fingerprint of
  the code that made them (stale when the code differs). Kept beside the
  deck, never in it.
- **Engine session**: the live process that runs a deck's cells (a Jupyter
  kernel, an R session or, later, a Julia session), one per deck while the
  editor is open, ended by Restart or by closing the server. Observable JS
  cells have none; they run in the page.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: From the slide, the person can run a cell and see its new output
  without opening the source pane, in one click or key.
- **SC-002**: For a cell that takes under a second to run, the new output
  appears on the slide within 2 seconds of the click.
- **SC-003**: Changing a plot's code in the pop-up and seeing the re-drawn
  plot takes one key press (⇧Enter) after typing, with the pop-up still open
  for the next change.
- **SC-004**: An R deck and a Julia deck, each with three dependent cells,
  give the same outputs when run cell by cell on the slides as in
  `quarto render`.
- **SC-005**: No code ever runs in a view without the session token. A test
  of the presented deck and of a link without the token finds no ▶ and no
  way to run.
- **SC-006**: The `.qmd` is byte-identical before and after any number of
  runs, apart from edits the person saved from the pop-up.

## Assumptions

- The preview keeps showing stored outputs (004 research R7). A run updates
  one cell's output on the slide; it doesn't execute the whole deck.
- Running one cell uses a live session per deck, as 004 does for Python,
  rather than a full Quarto render per click, because a render is too slow
  for the edit-and-see loop. The final `qd render` / `qd export` still runs
  the whole deck through Quarto, so the deliverable is always the real thing.
- The presented deck (`/present/`) never gets Run. It has no session token,
  and presenting is not the time to run code.
- ⇧Enter keeps its 004 meaning in the source pane, and gets the same meaning
  in the pop-up. On the slide, with a cell selected, the run key is ⇧Enter
  too.
- Quarto's extras (captions, cross-reference numbers, `layout-*` options)
  are not reproduced by a single run. They return with the next render
  (edge cases).
- Locked environments for R (renv) and Julia (a project) are a later spec.
  Until then a deck's R or Julia packages are whatever the person has
  installed, as with `quarto render`.
- Only the editor in a browser (and in VS Code, 006) is in scope. The
  presentation and exported files stay as they are.
