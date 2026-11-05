# Feature Specification: App shell — menus, settings, shortcuts, consistent behaviour

**Feature Branch**: `007-app-shell`

**Created**: 2026-11-05

**Status**: Draft

## Clarifications

### Session 2026-11-05

- Q: When the person is editing text on a slide (or in a source popup) and
  presses Escape, what happens to what they typed? → A: It is kept, as if
  they had clicked elsewhere; undo takes it back. This changes today's
  behaviour, where Escape cancels.

**Input**: User description: "App shell for the quartz-deck editor (roadmap 007): desktop-app conventions so the browser editor (and the same page framed in VS Code, spec 006) feels like one coherent application. A menu bar — File, Edit, Insert, Slide, View, Run, Help — that lists every action the editor already has (undo/redo, add/duplicate/move/remove slides, insert elements, formatting, source pane, run cell / run all / stop, render, export and PDF, offline check, preview) with its keyboard shortcut, so nothing is only reachable by right-click or a hidden key; menus are keyboard-navigable and accessible. A Settings window for the person's preferences (e.g. source pane on the left or right — default left, editor theme, autosave/preview behaviour, and room for later settings such as 009's "save as git commit"), stored per user and applied live. A shortcut reference (Help → Keyboard shortcuts, and "?") generated from the same action list so it can never drift from the real bindings. Consistent paradigms across the editor: one selection model, clipboard (cut/copy/paste of elements and slides, not just text), undo/redo covering every edit, Escape always backs out one level and Enter always goes in / confirms, focus always visible. In VS Code (006) the menu bar is hidden or folded into a compact menu since VS Code has its own, and shortcuts must not clash with VS Code's. Agents are unaffected: every menu action already has a qd CLI command; the spec may add a `qd` way to list actions/shortcuts for the skill. Clean-room: our own design, no slides.com code."

## Overview

The editor has grown feature by feature (specs 001–006). Its actions are now
spread over a header of buttons, right-click menus, property toolbars, the
source pane's bar and keys that are only discoverable by accident. This
feature gives it the shape of a desktop application: one menu bar holding
every action, one place for preferences, one list of shortcuts, and the same
behaviour for selection, clipboard, undo, Escape and Enter everywhere. It adds
no new deck editing power; it makes the existing power findable and
predictable.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Every action is in a menu (Priority: P1)

The person opens the editor and finds a menu bar: File, Edit, Insert, Slide,
View, Run, Help. Each menu lists the actions of that kind with their keyboard
shortcuts beside them. Actions that do not apply right now (Run cell with no
cell selected, Paste with nothing copied) are shown greyed out rather than
hidden, so the menu layout never shifts. The menus work with the keyboard
alone and are announced properly by a screen reader.

**Why this priority**: It is the core of the feature. Right-click menus and
hidden keys are fine for experts but leave everyone else guessing; a menu
bar is the one place a person can always look.

**Independent Test**: Open a deck in the editor. Without right-clicking or
pressing any shortcut, reach and run each of: undo, add slide after,
duplicate slide, delete slide, insert a paragraph, bold, toggle the source
pane, run all cells, render, export, offline check, open the preview. Then
open every menu with the keyboard only and run one item from each.

**Acceptance Scenarios**:

1. **Given** the editor is open, **When** the person opens any menu, **Then**
   it lists its actions with their shortcuts, and actions not available now
   are shown disabled.
2. **Given** a slide element is selected, **When** the person opens Edit,
   Insert or Slide, **Then** the element and slide actions from its
   right-click menu are there too, acting on the same selection.
3. **Given** keyboard focus is anywhere in the editor, **When** the person
   presses the menu key (F10, or Alt on its own) and uses the arrow keys,
   Enter and Escape, **Then** they can open any menu, run any item, and
   return focus to where it was.
4. **Given** a screen reader is running, **When** the person moves through a
   menu, **Then** each item is read with its name, state (checked, disabled,
   has submenu) and shortcut.
5. **Given** an action runs from a menu, **When** it finishes, **Then** it
   behaves exactly as the same action does from its button, right-click menu
   or shortcut (same result, same undo step, same messages).

---

### User Story 2 — Settings that stick (Priority: P2)

The person opens Settings (from the File or app menu, or ⌘,) and sets how the
editor looks and behaves: which side the source pane sits on, the editor's
colour theme, whether the source pane opens by default, and the other
preferences listed below. Each change applies at once, without a reload. The
next time they open any deck, in a new browser tab, after restarting the
server, or inside VS Code, their settings are the same.

**Why this priority**: Today the few preferences there are get forgotten
whenever the editor's address changes, and VS Code's panel never sees them.
Settings that stick are what make the editor feel like the person's own
application.

**Independent Test**: Change the source-pane side, the theme and the
"source pane open by default" setting; close the editor, stop the server,
open a different deck; the three settings are as set. Open the same deck in
VS Code's panel: the source-pane side is as set there too.

**Acceptance Scenarios**:

1. **Given** Settings is open, **When** the person changes a setting,
   **Then** the editor reflects it immediately, with no reload and no lost
   work.
2. **Given** settings were changed, **When** the person opens another deck,
   restarts the server, or opens the editor in VS Code, **Then** the same
   settings apply.
3. **Given** a setting's value is invalid or the settings cannot be read,
   **When** the editor starts, **Then** it uses that setting's default, says
   so once, and keeps working.
4. **Given** the person wants the defaults back, **When** they choose "Reset
   to defaults" in Settings, **Then** every setting returns to its default
   after a confirmation.
5. **Given** a later feature adds a setting (for example 009's "Save as git
   commit"), **When** it is added, **Then** it appears in the same Settings
   window and is stored the same way, without changing this feature.

---

### User Story 3 — One list of shortcuts (Priority: P2)

The person presses ? (or chooses Help → Keyboard shortcuts) and sees every
keyboard shortcut the editor has, grouped like the menus and searchable. The
list always matches the real behaviour, because the menus, the reference and
the key handling all come from the same list of actions. An agent can get the
same list from the command line.

**Why this priority**: A shortcut nobody can find does not exist. A
reference that drifts from the real keys is worse than none.

**Independent Test**: Open the reference with ?; search for "slide"; every
listed shortcut, pressed in the editor, does what the reference says; no key
handled by the editor is missing from the list. `qd` prints the same list as
JSON.

**Acceptance Scenarios**:

1. **Given** the editor has focus and the person is not typing in a field,
   **When** they press ?, **Then** the shortcut reference opens; Escape closes
   it and returns focus.
2. **Given** the reference is open, **When** the person types in its search
   box, **Then** only matching actions and shortcuts are shown.
3. **Given** any action with a shortcut, **When** its shortcut is pressed in
   the place the reference says it works (slide, source pane, text being
   edited), **Then** that action runs.
4. **Given** an agent, **When** it asks `qd` for the editor's actions,
   **Then** it gets each action's name, menu, shortcut and the matching `qd`
   command where there is one, as JSON.

---

### User Story 4 — The same rules everywhere (Priority: P3)

Wherever the person is (the slide list, a slide, an element, text being
edited, the source pane, a dialog), the same keys mean the same things:
Escape backs out one level (closes a menu or dialog, then ends editing, then
clears the selection, and in VS Code then hands the keyboard back to the
text); Enter goes in one level or confirms (opens editing on a selected
element, confirms a dialog). Cut, copy and paste work on whatever is
selected: text, an element, or a whole slide, and a copied slide pastes into
another deck. Every change can be undone with one undo step per action. The
keyboard focus is always visible.

**Why this priority**: Most of this already holds in parts of the editor;
this story makes it hold everywhere and closes the gaps (slides cannot be
copied today; a few dialogs ignore Enter). Valuable, but the editor is
usable without it.

**Independent Test**: In each of the slide list, a slide, an element, text
editing, the source pane and the Settings window, press Escape repeatedly:
each press backs out exactly one level and focus stays visible. Copy a slide
in one deck and paste it into another; undo removes it in one step.

**Acceptance Scenarios**:

1. **Given** any nested state (a menu open over an element being edited, on
   a selected slide), **When** the person presses Escape repeatedly, **Then**
   each press backs out one level in a fixed order, and an edit in progress
   is kept, not discarded (undo takes it back).
2. **Given** a slide is selected in the slide list, **When** the person
   copies it and pastes it after another slide (in the same deck or another
   deck's editor), **Then** an identical slide appears there, with fresh ids
   where its ids would clash.
3. **Given** an element or slide was copied in the editor, **When** the
   person pastes into a plain text editor elsewhere, **Then** they get that
   element's or slide's markdown source.
4. **Given** any edit made through the editor (menu, shortcut, toolbar,
   drag, typing, source pane), **When** the person presses undo, **Then**
   exactly that edit is undone, and redo puts it back.
5. **Given** the person moves through the editor with Tab and the arrow
   keys, **When** focus lands on any control, element or slide, **Then** a
   clear focus indicator shows where it is, in every theme.

---

### User Story 5 — Inside VS Code (Priority: P3)

In VS Code's slide panel (spec 006) there is no menu bar taking room; the
same actions are reachable from one compact menu button and from VS Code's
own command palette. The editor's shortcuts never take a key VS Code needs.

**Why this priority**: The panel is narrow and VS Code already has a menu
bar and a palette; a second full menu bar would waste space and confuse.

**Independent Test**: Open a deck in VS Code's panel: no menu bar is shown;
the compact menu lists the same menus and items; none of the editor's
shortcuts overrides a default VS Code binding while the panel has focus.

**Acceptance Scenarios**:

1. **Given** the editor is shown in VS Code's panel, **When** it loads,
   **Then** the menu bar is replaced by one menu button holding the same
   menus.
2. **Given** a shortcut the editor uses would clash with a VS Code default
   (for example ⌘P, ⌘⇧P, ⌘B, ⌘W, ⌘J), **When** the panel has focus, **Then**
   VS Code's meaning wins and the editor offers that action under a
   different shortcut or through the menu, and the shortcut reference in the
   panel shows what applies there.
3. **Given** settings were set in the browser editor, **When** the panel
   opens, **Then** it uses the same settings, except the theme, which follows
   VS Code (spec 006).

---

### Edge Cases

- **Small windows**: below a width where the menu bar fits, menus fold into
  the same compact menu button as in VS Code; nothing becomes unreachable.
- **Typing in a field**: printable shortcuts (?, single letters) never fire
  while the person is typing in a text field, the source pane or text on a
  slide; only modified shortcuts (with ⌘/Ctrl/Alt) and Escape act there.
- **Mac and other platforms**: shortcuts are shown and handled with ⌘ on
  macOS and Ctrl elsewhere; the reference shows the ones for this platform.
- **Browser-reserved keys**: keys the browser will not give to a page (for
  example ⌘W, ⌘T, ⌘N) are never used as shortcuts.
- **An action that runs code** (Run cell, Run all, Restart & run all) stays
  an explicit action by the person (spec 004): menus and shortcuts only
  trigger it on the person's own key press or click.
- **Two editors open** on the same or different decks: settings changed in
  one apply to the other at its next reload at the latest; they never
  corrupt each other.
- **Settings written by a newer version** with unknown keys: unknown keys are
  kept and ignored, not deleted.
- **Clipboard permission denied** by the browser: copy and paste inside the
  editor still work; copying to other applications reports that the browser
  blocked it.
- **A pasted slide or element that is not valid deck content** (for example
  arbitrary text from elsewhere): it is pasted as a paragraph of that text,
  not rejected silently and not interpreted as raw HTML.
- **Paste across decks** of an element referring to a file (an image) that
  does not exist next to the target deck: it is pasted, and the deck check
  reports the missing file as usual.

## Requirements *(mandatory)*

### Functional Requirements

**Actions (all stories)**

- **FR-001**: The editor MUST have one list of actions. Each action has a
  name, a menu (and position in it), an optional shortcut per platform, the
  places where its shortcut works, and when it is available. Menus,
  right-click menus, toolbar tooltips, the shortcut reference and key
  handling MUST all be derived from this list.
- **FR-002**: Every action the editor offers anywhere (header buttons,
  right-click menus, property toolbars, the formatting toolbar, the source
  pane's bar, keyboard-only shortcuts) MUST be in the list and in a menu.
- **FR-003**: Running an action from a menu, a button, a right-click menu or
  its shortcut MUST have the same effect, the same undo step and the same
  messages.

**Menu bar (US1)**

- **FR-004**: The browser editor MUST show a menu bar with File, Edit,
  Insert, Slide, View, Run and Help. Unavailable items are shown disabled;
  toggles show their state; items that open a dialog end in "…".
- **FR-005**: Menus MUST be fully operable from the keyboard (open with
  F10 or Alt alone, move with arrows, run with Enter, close with Escape,
  type-ahead to jump to an item) and MUST follow the accessible menu pattern
  so screen readers announce name, state and shortcut.
- **FR-006**: Closing a menu MUST return focus to where it was before.

**Settings (US2)**

- **FR-007**: A Settings window MUST be reachable from the menus and with
  ⌘, (Ctrl+, elsewhere). Changes MUST apply immediately.
- **FR-008**: Settings MUST belong to the person (not to a deck), and MUST be
  the same across decks, server restarts, browser tabs and VS Code's panel.
- **FR-009**: The first settings are: source pane side (left, default;
  right); source pane open when the editor opens (default: as last time);
  editor theme (follow the system, default; light; dark; high contrast; in
  VS Code the theme follows VS Code); preview update while typing (live,
  default; or only on pause); confirm before deleting a slide (default: on);
  show shortcut hints in tooltips (default: on).
- **FR-010**: Each setting MUST have a default; an unreadable or invalid
  stored value MUST fall back to the default with one notice, and "Reset to
  defaults" MUST restore all of them after a confirmation.
- **FR-011**: Settings MUST be stored where only the person can read and
  write them, MUST NOT contain deck content or secrets, and unknown keys MUST
  be preserved.
- **FR-012**: Later features MUST be able to add settings to the same window
  and store (009's "Save as git commit" is expected) without changing how
  existing settings are stored.

**Shortcut reference (US3)**

- **FR-013**: The shortcut reference MUST open with ? (when not typing) and
  from Help, list every action with a shortcut grouped by menu, show where
  each works, and be searchable.
- **FR-014**: The reference MUST be generated from the action list (FR-001);
  a test MUST fail if any key the editor handles is not in the list.
- **FR-015**: `qd` MUST offer the action list to agents as JSON: each
  action's name, menu, shortcut and the matching `qd` command where one
  exists. The agent skill MUST mention it.

**Consistent behaviour (US4)**

- **FR-016**: Escape MUST back out exactly one level, in this order: close
  the topmost menu or dialog; end the text or source edit; clear the element
  selection; (in VS Code) hand the keyboard to the text. Ending an edit with
  Escape MUST keep what was typed, as if the person had clicked elsewhere;
  undo takes it back. (Today Escape cancels an on-slide text edit; this
  changes it.) Enter MUST go in one level or confirm.
- **FR-017**: Cut, copy and paste MUST work on the current selection, which
  is text, one element or one slide. Copying puts the deck markdown on the
  system clipboard (when the browser allows) and keeps an internal copy, so
  paste works inside the editor even without clipboard permission.
- **FR-018**: Pasting a slide or element MUST insert it after the selection
  (or at the end), assign fresh ids where they would clash, and be one undo
  step. Pasted text that is not deck markdown MUST become a paragraph.
- **FR-019**: Every edit made in the editor MUST be undoable and redoable as
  one step per action, in the order made, whatever surface made it.
- **FR-020**: Every focusable control, slide and element MUST show a visible
  focus indicator with enough contrast in every theme.

**VS Code (US5)**

- **FR-021**: When framed in VS Code (spec 006), the menu bar MUST fold into
  one menu button holding the same menus; the shortcut reference MUST list
  the shortcuts as they apply in the panel.
- **FR-022**: While the panel has focus, the editor MUST NOT take any key
  bound by default in VS Code; an action whose usual shortcut clashes MUST
  get a different one there or be menu-only.

**Agents and safety**

- **FR-023**: No action may become reachable only from the editor: every
  action that changes the deck MUST keep its `qd` command (constitution
  III).
- **FR-024**: Actions that run code MUST still run only on the person's own
  click or key press (spec 004); a menu or shortcut never runs code because
  of something a page or deck did.

### Key Entities

- **Action**: something the person can do in the editor. Name, menu and
  position, shortcut per platform, where its shortcut works (anywhere, slide,
  element, text editing, source pane), when it is available, whether it is a
  toggle, and the `qd` command that does the same (if any).
- **Menu**: a named group of actions in a fixed order, with separators and
  submenus; the same menus appear in the menu bar, the compact menu and the
  shortcut reference.
- **Setting**: a named preference with a type, allowed values, a default and
  a description; belongs to the person; applies live.
- **Clipboard item**: what was last cut or copied (text, an element or a
  slide), with its deck markdown and the deck it came from.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of the editor's actions (as counted from its buttons,
  right-click menus, toolbars and handled keys before this feature) are
  reachable from the menus, checked by an automated comparison.
- **SC-002**: Every menu item can be reached and run with the keyboard alone
  in at most 10 key presses from anywhere in the editor.
- **SC-003**: A changed setting is visible in the editor within 0.5 s, and
  is still in effect after closing the editor, restarting the server and
  opening another deck (3 of 3 settings tried).
- **SC-004**: The shortcut reference lists 100% of the shortcuts the editor
  handles, and 0 listed shortcuts do something other than what it says
  (automated check over every listed shortcut).
- **SC-005**: From any state reachable in the editor, at most 5 presses of
  Escape return to the plain slide view, and each press backs out exactly one
  level without discarding an edit in progress (checked for every state in an
  automated test).
- **SC-006**: A slide copied in one deck and pasted into another arrives
  identical (same markdown apart from renamed clashing ids) and is removed by
  one undo.
- **SC-007**: In VS Code's panel, 0 of VS Code's default key bindings are
  taken by the editor while the panel has focus.
- **SC-008**: Opening a menu or the shortcut reference takes under 100 ms on
  a 30-slide deck.

## Assumptions

- The editor stays a single web page with no build step (constitution VII);
  menus are the editor's own, not the browser's or the operating system's
  native menu bar.
- "The person" is a single local user; settings are per user on this
  machine (or on the remote host when working over Remote-SSH, where qd
  runs).
- The existing right-click menus stay as shortcuts to the same actions;
  they are not removed.
- Shortcut customisation (rebinding keys) is out of scope; the action list
  makes it possible later.
- Settings sync between machines is out of scope.
- Theme "follow the system" means the operating system's light/dark choice
  as the browser reports it.
- The Export ▾ menu's extra formats arrive with spec 008 and slot into File
  → Export; until then File → Export holds today's offline HTML and PDF.
- Copying a slide or element to another application gives its markdown
  source; rich (HTML/image) clipboard formats are out of scope.
