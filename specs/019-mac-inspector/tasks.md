---
description: "Tasks for 019 an inspector on the right when the window is wide"
---

# Tasks: An inspector on the right when the window is wide

**Input**: `specs/019-mac-inspector/` (spec.md, plan.md, research.md,
data-model.md, contracts/editor.md, quickstart.md)

**Tests**: required (constitution VI, test-first). Each story's browser tests
are written first and seen failing. They need a browser and skip without one.
They set the window to 1440 × 900 for the inspector and 1000 × 800 below the
width. Existing tests run at the default 800 × 600 and so stay without the
inspector.

**Organization**: by user story, in the spec's priority order. Each phase is
one PR:
1. Setup + Foundational + US1, P1: the inspector, its width check, and the
   element tabs (PR 1);
2. US2, P2: the Slide and Title tabs (PR 2);
3. US3, P3: adapting to the window and panes, the keyboard (PR 3);
4. Polish.

Paths are relative to the repo root.

- **Clean room (research R1):** tab names Slide, Title, Element, Text,
  Position; the editor's own look; no other app's screenshots, icons or
  wording anywhere.
- **Privacy:** tests and docs use made-up decks only.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on unfinished tasks)
- **[Story]**: US1–US3

---

## Phase 1: Setup

- [X] T001 Add a browser-test helper `Ed::window(w, h)` in `crates/qd/tests/onslide.rs`. It wraps `SetDeviceMetricsOverrideParams`, as `narrow_window` uses it, and waits for the editor's layout to settle. Switch `narrow_window` to it.

---

## Phase 2: Foundational (blocks every story)

- [X] T002 [P] Write a failing unit test in `crates/qd/src/settings.rs`: `ui.inspector` is a known bool key, default `true`; `qd settings` lists it.
- [X] T003 Add `ui.inspector` beside `ui.form_panel` in `crates/qd/src/settings.rs` (data-model "Setting"). T002 passes.
- [X] T004 Write a failing browser test `inspector_shows_when_wide` in `crates/qd/tests/onslide.rs`:
  - at 1440 × 900, `<main>` has the class `inspector`; `#panel` has a `tablist` labelled "Inspector" with the tabs of data-model "Which tabs show";
  - at 1000 × 800 the class is off and the page is as before;
  - with `ui.inspector` false (set through the editor's settings) the class is off at 1440.
- [X] T005 Create `crates/qd/src/ui/inspector.js` and wire it from `crates/qd/src/ui/editor.js`:
  - the width check (research R3: window ≥ 1200 px and the slide's column ≥ 640 px, two constants), run on load, on window resize and from the existing `ResizeObserver`;
  - the class `inspector` on `<main>`;
  - the `tablist`/`tabpanel` structure of contracts/editor.md, in `crates/qd/src/ui/index.html`;
  - `tab` state that follows the selection (data-model "Page state").
  T004 passes.
- [X] T006 [P] Styles in `crates/qd/src/ui/editor.css`: the inspector column (320 px, beside the stage, on the left of it with `source-right`), the row of tab buttons, grouped sections; dark mode and the VS Code theme through the existing variables.

**Checkpoint**: the inspector appears and hides with the width; its tabs are there but empty.

---

## Phase 3: US1, P1: change the selected element from the inspector (PR 1)

**Goal**: with an element selected on a wide window, nothing floats over the slide; Element, Text and Position hold the element bar, formatting bar and element panel.

**Independent test**: spec US1's independent test.

- [X] T007 [P] [US1] Write failing browser tests in `crates/qd/tests/onslide.rs`, at 1440 × 900:
  - **`inspector_element_tabs`:**
    - selecting a paragraph shows Element, Text and Position, with Element selected, and `#elbar` is inside `#panel` and not floating;
    - "Appears on click" there adds `.fragment` exactly as the element bar does, with one undo step;
    - Delete removes the paragraph;
  - **`inspector_text_tab_while_typing`:**
    - double-clicking a paragraph switches to Text with `#fmtbar` inside the tab;
    - Bold there bolds the selected word, and the typing session stays open (the caret is in the paragraph);
    - with no typing, Text shows its note;
  - **`inspector_position_tab`:**
    - for a box, typing a left edge in Position moves it as today's panel does;
    - Move down moves a paragraph after the next;
    - a paragraph has Move up/down but no position fields;
  - **`inspector_tabs_by_kind`:** a code cell shows Element (with Run) and Position, no Text; an arrow (spec 016) and a diagram box (spec 015) show their fields in Element;
  - **`inspector_remembers_the_element_tab`:** choosing Position, then selecting another element, keeps Position.
- [X] T008 [US1] Move the element bar and the element panel into the Element tab in `crates/qd/src/ui/editor.js` and `crates/qd/src/ui/inspector.js`:
  - while the inspector shows, `placeElbar` puts `#elbar` in the tab instead of floating it, and back when it hides;
  - `renderElementPanel` renders its kind, ref, diagram/arrow fields and Markdown into the Element tab, and its position/size form and Move up/down into Position.
  Handlers unchanged (research R2).
- [X] T009 [US1] Move the formatting bar into the Text tab in `crates/qd/src/ui/toolbar.js` and `crates/qd/src/ui/inspector.js`:
  - `place`/`placeNear` skip positioning while `#fmtbar` is in the tab;
  - a typing session opening or closing switches to or from Text (research R4);
  - the note shows when there is no session.
- [X] T010 [US1] Add the `inspector` action (View ▸ Inspector, `Alt+Mod+0`, toggle) to `crates/qd/src/ui/actions.json` and its handler, which sets `ui.inspector`. Make the `form-panel` action select the inspector's tab on a wide window (research R5). T007 passes; the existing element bar, formatting bar and Properties tests still pass at 800 × 600.

**As built (PR 1):**
- **Tabs filter sections.** The tabs show or hide `data-tab`-tagged parts of `#panel`: its `.bars` (the three bars, moved in) and its `.body` (the existing panel views, tagged in their templates). The bars keep their own code. While in the inspector they're `position: static`, so their placement code has no effect, and `placeElbar` skips placing.
- **Shortcut:** ⌥⌘0, not ⌥⌘I, which opens Chrome's developer tools.
- **Typing:** the editor clears the selection while typing on the slide, so the inspector shows the Text tab alone then.
- **New buttons:** the element view gains Move up/down, tagged `data-tab=position` and running the existing `up`/`down` actions. They show in the Properties panel too.

**Checkpoint**: PR 1. On a wide window an element's controls are all in the inspector.

---

## Phase 4: US2, P2: the slide's own settings when nothing is selected (PR 2)

**Goal**: with nothing selected, the Slide tab (or Title on the title slide) holds the slide bar and slide panel; no slide bar above the slide.

**Independent test**: spec US2's independent test.

- [X] T011 [P] [US2] Write failing browser tests in `crates/qd/tests/onslide.rs`, at 1440 × 900:
  - **`inspector_slide_tab`:**
    - with nothing selected the Slide tab shows, `#slidebar`'s controls are inside it, and the row above the slide is gone (the preview starts at the top of the stage);
    - setting the transition to fade writes `transition="fade"` with one undo step;
    - Hidden, Background and the ⋯ menu act as from the slide bar;
  - **`inspector_title_tab`:** on the title slide the Title tab shows the title panel's fields, and editing the subtitle writes it to the front matter as today.
- [X] T012 [US2] Move the slide bar and the slide panel into the Slide tab, and the title panel into the Title tab, in `crates/qd/src/ui/editor.js` and `crates/qd/src/ui/inspector.js`:
  - `renderSlidebar` fills `#slidebar` wherever it is;
  - `renderPanel` and `renderTitlePanel` render into their tabs;
  - the slide bar's row collapses while the inspector shows (`crates/qd/src/ui/editor.css`).
  T011 passes.

**As built (PR 2):**
- **Mostly done by PR 1.** With nothing selected, the slide or title panel is the only tab's content. The slide bar moves into the inspector, so its row above the slide is gone. `renderSlidebar` fills it wherever it is.
- **What was left was a duplicate.** The Slide tab showed background and transition twice: the slide bar's controls, applied at once, and the slide form's copies, applied with Apply. In the inspector the form's Background fieldset and Transition field are hidden (CSS). Title, classes and content stay in the form.
- **The Title tab** holds today's title panel, which has the deck's title only. Subtitle, author and date stay in the front matter (`qd meta`), as before.

**Checkpoint**: PR 2. Nothing floats over or above the slide on a wide window.

---

## Phase 5: US3, P3: it adapts to the window (PR 3)

**Goal**: the inspector hides when there's no room and comes back when there is, without losing anything; it can be hidden and reached by keyboard.

**Independent test**: spec US3's independent test.

- [ ] T013 [P] [US3] Write failing browser tests in `crates/qd/tests/onslide.rs`:
  - **`inspector_follows_the_width`:**
    - an element selected at 1440 stays selected at 1000, with `#elbar` floating over it;
    - back at 1440 it is in the inspector on the same tab;
    - an open typing session and an unsaved Markdown field in the Element tab survive both resizes (SC-003);
  - **`inspector_and_panes`:** at 1440, opening the source pane at its default width hides the inspector when the slide's column would drop below 640 px, and closing it brings the inspector back; the same for History;
  - **`inspector_toggle_is_remembered`:** ⌥⌘0 hides it, a reload keeps it hidden (`ui.inspector` false), and ⌥⌘0 shows it again;
  - **`inspector_keyboard`:**
    - F6 moves focus from the slide to the selected tab and back;
    - the arrow keys move between tabs;
    - Tab reaches the tab's first field;
    - Escape in a field returns to the slide.
- [ ] T014 [US3] Run the width check when the source or History pane opens, closes or is resized, in `crates/qd/src/ui/editor.js`. Keep the selection, typing session and field values when nodes move in and out, in `crates/qd/src/ui/inspector.js`.
- [ ] T015 [US3] Implement the keys:
  - F6 between the slide and the inspector, in `crates/qd/src/ui/keys.js`, with its entry in the allowances of `keys_are_handled_only_through_the_list`;
  - the tabs' arrow keys and Escape, in `crates/qd/src/ui/inspector.js`.
  T013 passes.

**Checkpoint**: PR 3. The feature is complete.

---

## Phase 6: Polish

- [ ] T016 [P] Docs: `README.md` (the inspector, ⌥⌘0, when it shows) and `skills/quartz-deck/SKILL.md` (`ui.inspector` in `qd settings`). Run the tests for any `qd <word>` the docs mention (`skill_matches_cli`).
- [ ] T017 Run quickstart §1–§9 and record it in `specs/019-mac-inspector/checklists/quickstart-run.md`.
- [ ] T018 With the person: try it on their own screens and decks. Tune the two width constants (R3) if they ask, and record the result. Nothing about their decks goes into the repository.
- [ ] T019 Final constitution re-check in `plan.md`; fmt, clippy (`--workspace --all-targets -D warnings`) and `cargo test --workspace --no-fail-fast` green.
- [ ] T020 Concision audit (`/pr-audit`), the last task before each PR is marked ready.

---

## Dependencies & Execution Order

- **Setup (T001) → Foundational (T002–T006)** → every story.
- **US1 (T007–T010)** after Foundational.
- **US2 (T011–T012)** after Foundational. It doesn't depend on US1, but both
  edit `editor.js` and `inspector.js`, so build it after US1.
- **US3 (T013–T015)** after US1 and US2: it moves every bar in and out.
- **Polish** at the end; T020 before every PR; T018 needs the person.

## Parallel opportunities

- T002 ∥ T004 (different files); T006 ∥ T005 once the structure is in.
- Within a story, the test task runs while the previous story is reviewed.

## Implementation strategy

- **MVP:** PR 1 (Setup, Foundational, US1): an element's controls are
  beside the slide on a wide window.
- **Then:** PR 2 (the slide's tab), PR 3 (adapting and keyboard), Polish.

Each PR passes `/pr-audit` (T020) and is merged only after the person's
"merge when green".
