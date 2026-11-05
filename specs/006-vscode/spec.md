# Feature Specification: VS Code Integration

**Feature Branch**: `006-vscode`

**Created**: 2026-11-05

**Status**: Draft

**Input**: User description: "VS Code integration for quartz-deck (roadmap 006): everything the browser editor does, inside VS Code, working with the Quarto extension rather than replacing it. A small personal-use VS Code extension (same licence, installed from a .vsix) adds a command "Open in quartz-deck" that starts `qd serve` for the active .qmd and shows the interactive slide editor in a panel beside the text editor. The text editor and the panel edit the same deck: on-slide edits appear in the VS Code text buffer, typing in VS Code re-renders the panel, and the text cursor and the slide follow each other through the shared focus from spec 005 (moving the cursor shows that slide; selecting on a slide moves the cursor to its source). VS Code holds unsaved changes in its buffer while qd writes the file on disk, so neither side may ever overwrite the other's unsaved work (constitution V): panel edits go into the buffer as ordinary VS Code edits (undoable with VS Code's undo), or the extension saves/guards around them. Running code uses the Quarto extension's own Run Cell where it is installed, or spec 004's Run when not; render, preview and export map to qd commands in the command palette. Also: keyboard focus moves cleanly between panel and text editor, the panel follows VS Code's light/dark theme, and it works over Remote-SSH (the panel reaches qd serve through VS Code's port forwarding). Agents (Claude in VS Code) keep using the qd CLI, and qd focus reports the VS Code cursor too."

## Clarifications

### Session 2026-11-05

- Q: In VS Code, should panel edits stay unsaved in the buffer until the
  person saves, or be written to the file at once (as the browser editor
  does)? → A: Unsaved until the person saves, like any VS Code edit; the
  file changes only on save (US2-6, FR-010).

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Slides beside the text, in VS Code (Priority: P1)

The person is editing a talk's `.qmd` in VS Code. They run "Open in
quartz-deck" from the command palette (or an editor title button), and the
interactive slide editor opens in a panel beside the text: the same slides,
on-slide typing, menus, boxes and source pane as in the browser editor. It
looks like part of VS Code (light or dark to match), and they never leave the
window.

**Why this priority**: VS Code is where the person already writes; having the
slides next to the text, without a browser window to juggle, is the reason
for the feature.

**Independent Test**: Open a `.qmd` in VS Code, run "Open in quartz-deck": the
panel shows the deck within a few seconds, in VS Code's theme; closing the
panel stops what it started.

**Acceptance Scenarios**:

1. **Given** a `.qmd` is the active editor, **When** the person runs "Open in
   quartz-deck", **Then** a panel opens beside it showing that deck's slides,
   with the full editor (typing, menus, source pane, Run).
2. **Given** VS Code is in a dark (or light) theme, **When** the panel opens or
   the theme changes, **Then** the panel follows it.
3. **Given** the panel is open, **When** the person closes it (or the `.qmd`
   editor), **Then** anything the extension started for it stops, and nothing
   is left running.
4. **Given** the deck is already open in a panel, **When** the command is run
   again, **Then** the existing panel is shown rather than a second one.
5. **Given** qd or Quarto is missing, **When** the command runs, **Then** the
   person gets a clear message saying what to install, not a blank panel.

---

### User Story 2 — One deck, two views, never a lost change (Priority: P1)

The text editor and the panel edit the same deck. Typing in VS Code updates
the slides; typing on a slide updates the text. Unsaved work on either side is
never overwritten by the other, and undo in VS Code undoes panel edits too.

**Why this priority**: Without this, two editors on one file is a data-loss
machine (constitution V). It ships together with US1.

**Independent Test**: Type in the VS Code buffer without saving, then edit a
different slide in the panel: both changes are present, nothing is lost, and
VS Code's undo reverts the panel edit.

**Acceptance Scenarios**:

1. **Given** the person types in the VS Code text editor, **When** they pause,
   **Then** the panel shows the change (whether or not they have saved).
2. **Given** the person edits on a slide in the panel, **When** the edit is
   made, **Then** it appears in the VS Code text editor at once, as an edit
   VS Code's undo can revert.
3. **Given** the VS Code buffer has unsaved changes, **When** the person edits
   another part of the deck in the panel, **Then** both sets of changes are
   kept; neither side's unsaved work is overwritten.
4. **Given** both sides change the same element at the same time, **When**
   that is detected, **Then** the person sees both versions and chooses, as
   in spec 003's conflict handling; nothing is silently dropped.
5. **Given** an agent edits the deck with `qd` while the buffer has unsaved
   changes, **When** that happens, **Then** VS Code shows the agent's change
   in the buffer, with no silent overwrite in either direction.
6. **Given** the person edits on a slide in the panel, **When** the edit is
   made, **Then** the VS Code buffer shows it as an unsaved change (the
   editor's dirty marker), and the file on disk changes only when the person
   saves, exactly like typing in VS Code.

---

### User Story 3 — Cursor and slide follow each other (Priority: P2)

Moving the text cursor into a slide's markdown shows that slide in the panel;
clicking or selecting an element in the panel moves the VS Code cursor to its
source and reveals it. An agent in VS Code that runs `qd focus` gets what the
person has selected, whether in the text editor or in the panel.

**Why this priority**: It makes the two views feel like one; it reuses spec
005's shared focus, so it is cheaper than it looks.

**Independent Test**: Put the VS Code cursor in slide 3's text: the panel
shows slide 3. Click a list on slide 5 in the panel: VS Code's cursor moves
to that list's source. `qd focus` reports each.

**Acceptance Scenarios**:

1. **Given** the panel is open, **When** the text cursor moves into another
   slide's markdown, **Then** the panel shows that slide.
2. **Given** the person clicks an element in the panel, **When** they do,
   **Then** the VS Code cursor moves to the start of its source, scrolled into
   view, without stealing keyboard focus from the panel.
3. **Given** text is selected in the VS Code editor, **When** an agent runs
   `qd focus`, **Then** it gets that selection, its element(s) and slide,
   with `where` saying it came from the text editor.
4. **Given** an agent runs `qd focus --set`, **When** the panel is open,
   **Then** the panel shows it as in spec 005, and the text editor reveals its
   source too (unless the person is typing, as in 005).

---

### User Story 4 — Run, render, export from VS Code (Priority: P3)

Code cells run where the person expects: with the Quarto extension installed,
its own Run Cell is used and quartz-deck stays out of the way; without it,
the panel's Run (spec 004) works as in the browser. Render, preview and export
are in the command palette.

**Why this priority**: Useful, but each piece already exists elsewhere; this
is wiring.

**Independent Test**: In VS Code without the Quarto extension, Run a cell in
the panel: output shows as in 004. With it installed, VS Code's own Run Cell
runs the cell and the panel shows no second Run button. "quartz-deck: Export"
in the palette produces the same files as `qd export`.

**Acceptance Scenarios**:

1. **Given** the Quarto extension is installed, **When** the panel is open,
   **Then** cells are run with the Quarto extension's commands, and the
   panel does not offer a competing Run.
2. **Given** it is not installed, **When** the person presses Run in the
   panel, **Then** the cell runs as in spec 004 (same environment, outputs).
3. **Given** the palette, **When** the person runs "quartz-deck: Render",
   "…: Export" or "…: Check offline", **Then** the matching `qd` command runs
   and its result (output path, problems) is shown in VS Code.

---

### Edge Cases

- **Remote-SSH / remote workspaces**: the deck and qd live on the remote
  machine; the panel still shows and edits it, reached through VS Code's own
  port forwarding (no extra setup by the person).
- **The `.qmd` is not saved yet (untitled) or is not a reveal.js deck**: the
  command explains it needs a saved `.qmd`; non-revealjs formats open but say
  that slides are a reveal.js view.
- **The file is renamed, moved or deleted while the panel is open**: the panel
  says so and offers to reopen; no edits are written to the old path.
- **The same deck is also open in a browser editor**: both stay consistent
  under the existing conflict rules.
- **Several decks open**: each panel belongs to its own deck.
- **Keyboard focus**: a shortcut moves focus between text editor and panel;
  the panel's own shortcuts never fire while VS Code's text editor has focus
  and vice versa.
- **VS Code restarts** with the panel open: it reopens on the same deck.

## Requirements *(mandatory)*

### Functional Requirements

**Panel (US1)**

- **FR-001**: An "Open in quartz-deck" command (palette and editor title) MUST
  open the deck of the active `.qmd` in a panel beside it, with the full
  editor of specs 001–005.
- **FR-002**: Running it again for the same deck MUST reveal the existing
  panel; each deck has at most one panel per window.
- **FR-003**: The panel MUST follow VS Code's light/dark/high-contrast theme,
  at open and when the theme changes.
- **FR-004**: Closing the panel MUST stop everything the extension started for
  it; reopening VS Code with the panel open MUST restore it.
- **FR-005**: Missing tools (qd, Quarto) MUST produce a message naming what to
  install, not an empty panel.

**Same deck, no lost changes (US2)**

- **FR-006**: Changes typed in the VS Code buffer MUST reach the panel without
  a save, within the same time the browser editor takes after a file change.
- **FR-007**: Edits made in the panel MUST appear in the VS Code buffer as
  edits VS Code's undo can revert.
- **FR-008**: Neither side MAY overwrite the other's unsaved work: concurrent
  changes to different parts are both kept; to the same element, the person
  chooses (spec 003's conflict rules).
- **FR-009**: Changes made on disk by others (agents via `qd`, other editors)
  MUST reach both the buffer and the panel with the same guarantees.
- **FR-010**: Panel edits MUST leave the VS Code buffer unsaved until the
  person saves; the file on disk changes only on save (US2-6). While a VS
  Code panel is open, the buffer — saved or not — is what the panel shows.

**Cursor ↔ slide (US3)**

- **FR-011**: The VS Code cursor and selection MUST feed spec 005's shared
  focus (`where` = the text editor); the panel MUST follow it to the slide.
- **FR-012**: Selecting an element in the panel MUST move the VS Code cursor
  to its source and reveal it, without taking keyboard focus.
- **FR-013**: `qd focus --set` MUST also reveal the target in the text editor,
  under spec 005's never-while-typing rule.

**Run, render, export (US4)**

- **FR-014**: With the Quarto extension installed, running cells MUST use its
  commands and the panel MUST NOT offer a second Run; without it, spec 004's
  Run MUST work in the panel.
- **FR-015**: Palette commands MUST cover render, export (HTML/PDF), and the
  offline check, each showing qd's result in VS Code.

**Environment and safety**

- **FR-016**: The panel MUST work in remote workspaces (Remote-SSH and the
  like) using VS Code's own forwarding, with no manual port setup.
- **FR-017**: Spec 004/005's safety rules MUST hold unchanged: code runs only
  on an explicit action, and only the panel (and qd on the machine) can run
  code or read/set the focus; nothing else in VS Code or on the web gains that
  access through the extension.
- **FR-018**: The extension MUST be installable from a single file without a
  marketplace, under the project's licence, and MUST NOT replace or disable
  the Quarto extension.

### Key Entities

- **Deck panel**: one per deck per window; tied to the deck's file and the
  running editor it shows; ends with the panel.
- **Buffer**: VS Code's in-memory text of the `.qmd`, possibly unsaved; the
  person's source of truth while they are editing in VS Code.
- **Focus** (spec 005): now also fed by the VS Code cursor and selection.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: From "Open in quartz-deck" to the slides showing in the panel:
  ≤ 5 s for a 30-slide deck on a warm machine.
- **SC-002**: A change typed in the VS Code buffer shows in the panel within
  3 s (spec 001's preview target), saved or not.
- **SC-003**: In a 15-minute session alternating edits between buffer and
  panel (and an agent editing via `qd`), 0 changes are lost and every panel
  edit is undoable with VS Code's undo.
- **SC-004**: Moving the text cursor to another slide shows it in the panel
  within 0.5 s in 95% of moves; selecting in the panel moves the text cursor
  within 0.5 s.
- **SC-005**: The same session works over Remote-SSH with no manual setup
  beyond installing the extension and qd on the remote.
- **SC-006**: Nothing outside the panel and local qd can run code or read the
  focus through the extension (0 of all attempts in a security test like
  specs 004/005's).

## Assumptions

- The extension is personal-use, installed from a `.vsix`, same licence; no
  marketplace listing.
- qd and Quarto are installed where the deck lives (locally, or on the remote
  for Remote-SSH); the extension does not install them.
- The panel shows the same editor the browser shows (one editor to maintain);
  VS Code-specific behaviour is added around it, not a second editor.
- Agents keep using the `qd` CLI from VS Code's terminal; the extension adds
  no agent-specific interface.
- Out of scope: other editors (JetBrains, Neovim), notebook (`.ipynb`)
  decks, and any marketplace publishing. Spec 007 (app shell) may reuse the
  panel work.
- Depends on specs 004 (Run, source pane) and 005 (shared focus).
