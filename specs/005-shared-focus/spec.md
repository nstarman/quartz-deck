# Feature Specification: Shared Focus

**Feature Branch**: `005-shared-focus`

**Created**: 2026-11-05

**Status**: Draft

**Input**: User description: "Shared focus for quartz-deck (roadmap 005): the editor reports what the person is looking at and has selected, so an agent in chat knows what "this slide", "this paragraph" or "the selected text" refer to, and an agent can point the editor at a slide or element. `qd focus` returns, as JSON, the current slide, the selected element (if any) and any selected text (in the slide view or the source pane), plus whether the editor is open at all and how recently the focus changed. `qd focus --set <slide|element>` (or `qd show-slide <ref>`) moves the person's editor to that slide and highlights the element, without interrupting typing in progress. Agents use it in the skill: when the person says "this", read `qd focus` first. Focus is local only (never leaves the machine), carries no deck content beyond refs and the selected text, and reading it never changes anything. The same two-way link is the one 004's source↔slide sync uses and that 006's VS Code integration will reuse for editor↔VS Code cursor sync."

## User Scenarios & Testing *(mandatory)*

### User Story 1 — An agent knows what "this" means (Priority: P1)

The person has the deck open in the editor and is chatting with an agent. They
click a paragraph on slide 4 (or select a few words, or put the cursor in a
block of the source pane) and say "make this shorter". The agent asks qd what
the person has in focus and gets the slide, the element and the selected text,
then edits exactly that element, without asking "which slide?".

**Why this priority**: This is the whole point of shared focus. Today an agent
has to guess or ask, and "this", "here" and "the selected text" are the most
natural way people refer to things while looking at their slides.

**Independent Test**: With the editor open, select a word in a paragraph on
some slide. `qd focus` reports that slide, that paragraph and the selected
word. An agent told "make this bold" changes only that word.

**Acceptance Scenarios**:

1. **Given** the editor is open on a slide with nothing selected, **When** an
   agent reads the focus, **Then** it gets that slide (with the same reference
   other qd commands accept), no element and no selected text.
2. **Given** the person has clicked an element on the slide, **When** the
   agent reads the focus, **Then** it gets the slide and that element, ready
   to pass to an editing command.
3. **Given** the person has selected some words (on the slide or in the source
   pane), **When** the agent reads the focus, **Then** it gets those words,
   where they were selected, and the element they belong to.
4. **Given** the person moves the cursor in the source pane, **When** the
   agent reads the focus, **Then** it gets the slide and element under the
   cursor, the same as clicking that element on the slide.
5. **Given** no editor is open for the deck, **When** the agent reads the
   focus, **Then** it is told plainly that no editor is open (not an error),
   so it can ask the person instead.
6. **Given** the focus was last changed long ago, **When** the agent reads it,
   **Then** it can see how long ago, to judge whether it still reflects what
   the person means.

---

### User Story 2 — An agent points the person at a slide or element (Priority: P2)

An agent has made a change, or wants to ask about something specific ("is this
the chart you meant?"). It tells qd to show a slide or element, and the
person's editor goes to that slide and briefly highlights the element.

**Why this priority**: The reverse direction makes conversation about the deck
two-way: the agent can show, not just describe. It is less essential than
knowing what the person means.

**Independent Test**: With the editor open on slide 1, `qd focus --set results/1`
moves the editor to the Results slide and highlights its second element; the
person sees which element is meant.

**Acceptance Scenarios**:

1. **Given** the editor is open and the person is not typing, **When** an
   agent asks to show a slide, **Then** the editor shows that slide.
2. **Given** the same, **When** an agent asks to show an element, **Then** the
   editor shows its slide and highlights that element for a moment, and the
   source pane (if open) scrolls to it.
3. **Given** the person is in the middle of typing (an edit is open on the
   slide or they are typing in the source pane), **When** an agent asks to
   show something, **Then** nothing moves and nothing typed is lost; the
   person sees an unobtrusive "go to …" offer they can click, and the agent is
   told the move was deferred.
4. **Given** the reference does not exist, **When** an agent asks to show it,
   **Then** it gets the same not-found error as other commands and the editor
   does not move.
5. **Given** no editor is open, **When** an agent asks to show something,
   **Then** it is told no editor is open and nothing else happens.

---

### Edge Cases

- **Several editor tabs or windows on the same deck**: the focus is the one
  the person used most recently; pointing affects that one.
- **A selection spanning several elements**: the focus lists every element the
  selection touches, first one first; the selected text is reported whole
  (up to the length limit).
- **A very long selection**: the selected text is cut at a fixed length and
  marked as cut, so a stray "select all" never floods an agent.
- **The deck changed after the focus was taken** (the person or an agent
  edited it): the focus carries the deck version it refers to, so an agent
  can tell when a reference may have moved; references to elements that no
  longer exist are dropped.
- **Focus on the title slide or front matter**: reported as such (no slide
  element), so "this title" still has a clear meaning.
- **Focus on a hidden slide**: reported normally; hidden slides are shown in
  the editor.
- **The editor is open but in a background tab or minimised**: still reported
  as open; the age of the last focus change tells the agent how fresh it is.
- **Another program or web page on the machine tries to read or set the
  focus**: refused, under the same rules that keep other pages from running
  code (see FR-009).

## Requirements *(mandatory)*

### Functional Requirements

**Reading the focus (US1)**

- **FR-001**: `qd focus` MUST return, as JSON: whether an editor is open for
  the deck; the current slide; the selected element(s), if any; the selected
  text, if any, with where it was selected (slide or source pane); the source
  line under the cursor when the source pane has it; the deck version the
  focus refers to; and how many seconds ago the focus last changed.
- **FR-002**: Slide and element references in the focus MUST be the same
  references every other qd command accepts, so an agent can pass them
  straight to an edit.
- **FR-003**: When no editor is open, `qd focus` MUST succeed and say so
  (`open: false`) rather than fail, so agents can branch on it.
- **FR-004**: The focus MUST follow the slide view and the source pane alike:
  clicking or selecting on a slide, moving the cursor or selecting in the
  source pane, and moving between slides all update it.
- **FR-005**: Selected text MUST be reported up to 2,000 characters, marked as
  cut when longer.
- **FR-006**: With several editor tabs open on the deck, the focus MUST be the
  most recently used tab's.

**Pointing the editor (US2)**

- **FR-007**: `qd focus --set <ref>` MUST move the most recently used editor
  to the slide of `<ref>` and, for an element, highlight it briefly and scroll
  the source pane to it. The result MUST say whether the editor moved, the
  move was deferred, or no editor is open.
- **FR-008**: A move requested while the person is typing (an open edit on the
  slide, or typing in the source pane within the last few seconds) MUST NOT
  move the view, take the cursor, or lose any typed character; the person MUST
  instead see a dismissable "go to …" offer, and the result MUST report the
  move as deferred.

**Safety and privacy**

- **FR-009**: Only qd on this machine and the person's own editor page MAY
  read or set the focus; other web pages and links MUST NOT, under the same
  rules that protect running code (session token, local-only).
- **FR-010**: Reading the focus MUST NOT change the deck, the editor's view, or
  anything else. The focus MUST NOT be written to disk or sent anywhere off
  the machine, and MUST contain nothing from the deck beyond references and
  the selected text.

**Agents**

- **FR-011**: The agent skill MUST tell agents to read the focus first when the
  person says "this", "here", "the selected …" or similar, to act on it when it
  is fresh, and to ask when no editor is open or the focus is old or empty.
- **FR-012**: The focus MUST be a single shared notion used by the editor's own
  source↔slide sync as well as by agents, so that later integrations (spec
  006, VS Code) can follow and set the same focus without a second mechanism.

### Key Entities

- **Focus**: what the person is attending to in the editor at a moment:
  open/closed, slide reference, element reference(s), selected text and where
  it was selected, source line, deck version, time of last change. Lives only
  in the running editor session; never stored.
- **Point request**: an agent's request to show a slide or element; its
  outcome is moved, deferred, or no editor.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: After the person clicks an element, selects text, or moves the
  source cursor, `qd focus` reflects it within 0.5 s in 95% of cases.
- **SC-002**: `qd focus --set` shows the requested slide (and highlights the
  element) within 1 s of the command when the person is not typing.
- **SC-003**: In a test where the person types continuously while an agent
  points the editor elsewhere 20 times, no typed character is lost and the
  view never moves mid-edit (0 of 20).
- **SC-004**: Agents asked "make this …" about something the person has
  selected change exactly that element, without asking which one, in 9 of 10
  trials, with no usage errors.
- **SC-005**: Reading the focus changes nothing: the deck file and the
  person's view are identical before and after 100 reads.
- **SC-006**: Another local web page cannot read or set the focus (0 of all
  attempts in a test like spec 004's security check).

## Assumptions

- One command (`qd focus` with `--set`) covers both directions; a separate
  `qd show-slide` alias is not needed.
- The focus is session-only state: when the editor closes, it is gone. There
  is no history of past focus.
- "Typing in progress" means an open on-slide edit, or keystrokes in the
  source pane within the last few seconds; the exact window is a plan detail.
- The highlight from a point request is brief and visual only; it does not
  select the element or change anything in the deck.
- Out of scope: following a person's focus live (an agent waiting for the next
  focus change), presenter mode, and VS Code (spec 006 reuses this focus).
- Depends on spec 004's source pane and on the editor's existing session
  token rules for local-only access.
