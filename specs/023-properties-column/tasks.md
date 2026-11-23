---
description: "Tasks for 023 one properties column"
---

# Tasks: One properties column

**Input**: `specs/023-properties-column/` (spec.md, plan.md, research.md,
data-model.md, contracts/cli.md, contracts/editor.md, quickstart.md)

**Tests**: required (constitution VI, test-first). Each story's tests are
written first and seen failing. Tests that need Quarto or a browser skip
without them.

**Organization**: by user story, in the spec's priority order. Each phase is
one PR:
1. Setup + US1, P1: Format, the bars gone (PR 1).
2. US2, P2: Animate (PR 2).
3. US3, P3: Deck (PR 3).
4. US4, P4: placement, and polish (PR 4).

Paths are relative to the repo root. **Privacy and clean room:** tests,
docs and the showcase use made-up content only. No file names any other
presentation app. Trying it on the person's talk (T024) leaves nothing
about the talk in the repository.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on unfinished tasks)
- **[Story]**: US1–US4

---

## Phase 1: Setup

- [X] T001 Create `crates/qd/src/ui/props/` with empty `format.js`, `animate.js` and `deck.js` modules. Each exports `render(target, api)` returning the view's body. Serve them like the other `ui/` modules in `crates/qd/src/server.rs`'s embedded-files list.
- [X] T002 [P] Add a test helper `props(ed, view)` in `crates/qd/tests/onslide.rs` that clicks the column's view tab (`#panel .views [data-view=…]`) and waits for its body. Add `no_bars(ed)`, which asserts `#slidebar` and `#elbar` are absent or hidden.

---

## Phase 2: US1, P1: every slide and element setting in one place (PR 1)

**Goal**: the column is always shown and holds every slide and element setting; the slide bar and element bar are gone.

**Independent test**: spec US1's independent test.

- [X] T003 [US1] Write failing browser tests in `crates/qd/tests/onslide.rs` (contracts/editor.md § Tests):
  - **`props_format_slide`:** with nothing selected, Format shows the background colour, image, hidden, title, classes and source. Setting each writes the same text as `qd set` / `qd title`. `no_bars`.
  - **`props_format_element`:** select a paragraph, a placed box, a picture, a diagram box and an arrow in turn. Each shows its tabs (Text only for text). The Look and Arrange settings write as today, and the footer's Delete deletes. `no_bars`.
  - **`props_text_tab`:** while typing, the floating toolbar shows. The Text tab's Bold bolds the selected word, and the selection is kept.
  - **`props_title_slide`:** on the title slide, Format shows title, subtitle, author and date, and each saves.
- [X] T004 [US1] Make the column always shown in `crates/qd/src/ui/inspector.js`:
  - remove `fits()`/`WIDE`/`SLIDE_MIN`;
  - `ui.inspector` is the only switch (View ▸ Properties);
  - add `.views` (Format, Animate, Deck) as a tablist above `.tabs`, with ←/→ and F6 as for tabs (research R1, R7).
- [X] T005 [US1] Build Format in `crates/qd/src/ui/props/format.js` (research R2):
  - the slide section (from the slide panel and slide bar);
  - the title-slide section;
  - an element's Look, Text and Arrange tabs (from the element panel, element bar, Arrange actions, and the diagram and arrow panels, moved, not rewritten);
  - the footer actions.

  Each control calls the same `op()` as before.
- [X] T006 [US1] Make `crates/qd/src/ui/toolbar.js`'s buttons renderable into a second container for the Text tab, bound to the same actions, with mousedown kept from stealing focus. The floating toolbar is unchanged.
- [X] T007 [US1] Remove `#slidebar` from `crates/qd/src/ui/index.html` and the element bar from `crates/qd/src/ui/editor.js`, with their CSS in `editor.css`. The right-click menus keep every action. Route the selection to the column (`renderPanel` → `format.render`).
- [X] T008 [US1] Move the existing tests that drive the slide bar, element bar or the old panel to the column's controls, keeping their checks on the deck's text: `bars_set_properties_and_source_panel_is_hidden`, `tidier_bars_and_fuller_insert`, the `inspector_*` tests, and any that click `#slidebar` or `#elbar`. T003 passes, and the full `onslide` suite passes.

**Checkpoint**: PR 1. Every setting has one home; no bar floats over the slide.

**As built (PR 1):**
- The editor serves its modules from one flat `/ui/{name}` route, so Animate
  is `ui/animate.js`, not `ui/props/animate.js`. Format stays in `editor.js`
  (its panels were moved, not rewritten), and Deck waits for PR 3.
- Animate already holds the slide's transition and an element's "Appears on
  click", moved from the slide and element bars, so each control has one
  home in PR 1. PR 2 fills in the rest.
- View ▸ Properties panel and View ▸ Inspector were one column now, so they
  are one item, View ▸ Properties (⌥⌘0). The unused `ui.form_panel` setting
  is gone (a stored value is kept and ignored, as for any unknown key).
- The narrow-window stack (below the slide at 700 px or less) was already the
  layout's own, so it holds in PR 1.

---

## Phase 3: US2, P2: Animate, transitions and builds (PR 2)

**Goal**: slide transitions with speed and morph, element appear and leave effects, and a reorderable build list.

**Independent test**: spec US2's independent test.

- [X] T009 [P] [US2] Write failing CLI tests in `crates/qd/tests/cli.rs` (contracts/cli.md):
  - `qd build set`: each appear effect; each leave with fade-in and with another appear; back to none; other classes kept; errors.
  - `qd build list`: the order, with and without `fragment-index`; an incremental list's items.
  - `qd build order`: a new order; back to source order removes the indexes; a missing or repeated build is `E_INVALID_ARG`.
- [X] T010 [US2] Implement `build_set`, `build_list` and `build_order` in `crates/qd/src/ops.rs` on the existing class and attribute edits (research R3), and `BuildCmd` with schema lines in `crates/qd/src/cli.rs`. T009 passes.
- [X] T011 [P] [US2] Write failing browser tests in `crates/qd/tests/onslide.rs`:
  - **`props_animate`:**
    - with nothing selected, set Zoom, fast and morph; check the slide's attrs;
    - select two elements and set their appear and leave; check the classes;
    - reorder the build list by drag and by ⌥↑; check `fragment-index`;
  - **`build_order_presents`:** in the preview, `Reveal.next()` through the slide shows the builds in the list's order.
- [X] T012 [US2] Build Animate in `crates/qd/src/ui/props/animate.js` (contracts/editor.md § Animate):
  - transition, speed and morph through `qd set`;
  - appear and leave selects through `qd build set`;
  - "One item at a time" for lists (the existing incremental action);
  - the build list from `qd build list`, reordered through `qd build order` (drag, and ⌥↑/⌥↓).

  Remove "Appears on click" from Format's footer. It stays in the right-click menu. T011 passes.
- [X] T013 [P] [US2] Showcase: give `examples/showcase/showcase.qmd`'s Lists and builds slide an appear-and-leave build and a changed build order. Give the Placed boxes slide a transition speed. Check `qd check`.

**Checkpoint**: PR 2.

**As built (PR 2):**
- reveal.js gives an element one effect per click, so a leave goes with
  fade-in (`fade-in-then-out`, `fade-in-then-semi-out`) or with no appear.
  Another appear with a leave is `E_INVALID_ARG`.
- An incremental list is listed with its items, but `qd build order` takes
  only the slide's fragments: reveal.js shows the list's items after the
  ordered builds, and the list says so.
- "Morph into the next slide" sets `auto-animate=true` on this slide and the
  next, since reveal.js morphs between two slides that both have it.
- The browser tests were written alongside the view rather than before it;
  the CLI tests came first.

---

## Phase 4: US3, P3: Deck settings (PR 3)

**Goal**: the deck's theme, slide size, slide numbers, default transition and navigation in the Deck view.

**Independent test**: spec US3's independent test.

- [X] T014 [P] [US3] Write failing unit tests in `crates/qd/src/ops.rs`:
  - dotted `meta set` and `unset` on a deck without `format:`, with `format: revealjs:`, and with `format: html` (refused);
  - empty maps removed on unset;
  - `meta size` with `--keep-places` scaling every positioned box's `left/top/width/height` and every arrow's points.
- [X] T015 [US3] Implement dotted keys in `meta_set`/`meta_unset` and `meta_size` in `crates/qd/src/ops.rs`, and `MetaCmd::Size` in `crates/qd/src/cli.rs` (research R4). T014 passes; existing `meta` tests still pass.
- [X] T016 [P] [US3] Write a failing browser test **`props_deck`** in `crates/qd/tests/onslide.rs`: change the theme, then the slide size to 4:3 (a placed box keeps its place relative to the slide), slide numbers and the default transition. Check the front matter and the preview.
- [X] T017 [US3] Build Deck in `crates/qd/src/ui/props/deck.js` (contracts/editor.md § Deck):
  - the theme select: Quarto's built-in theme names, plus the deck's `.scss`;
  - size presets and custom;
  - slide numbers;
  - default transition;
  - navigation through spec 018's op.

  T016 passes.

**As built (PR 3)**: dotted keys are `format.revealjs.KEY` only. Removing
the mapping's last key leaves `revealjs: default`, so the deck stays a
reveal.js deck. The view is `ui/deck.js`, beside `ui/animate.js`; the unit
tests are in `tests/ops.rs`, beside the other `meta` tests.

**Checkpoint**: PR 3.

---

## Phase 5: US4, P4: the column goes where the presenter wants it (PR 4)

**Goal**: drag the column to any of four places, below in a narrow window, hide and show it.

**Independent test**: spec US4's independent test.

- [X] T018 [P] [US4] Write failing browser tests **`props_dock`** and **`props_keyboard`** in `crates/qd/tests/onslide.rs` (contracts/editor.md § Tests): each place, after a reload; narrow puts it below; View ▸ Properties hides and shows; with the source pane on the same side, both show in the set order (research R5). For the keyboard: F6 in and Esc out, arrows along the views and tabs.
- [X] T019 [US4] Add `properties.side` (right, far-left, left, below; default right) and `ui.properties_width`/`ui.properties_height` to `crates/qd/src/settings.rs`, with tests.
- [X] T020 [US4] Make `crates/qd/src/ui/dock.js` take a pane, its header and its setting, and use it for the column's header. Lay the column out from `main[data-props]` in `crates/qd/src/ui/editor.css`, with the shared order beside the source pane and the narrow-window stack. T018 passes, and spec 022's `source_dock_*` tests still pass.

**As built (PR 4)**:
- The panes are laid out by one CSS grid on `main`, with a column for each pane in each place. Each pane is placed from its own setting alone.
- `ui.properties_width` and `ui.properties_height` were left out: the column keeps its width, and it is at most 35% of the window's height below the slide.
- On a narrow window the source pane stays right under the slide, as in spec 022, and the column comes after it.

**Checkpoint**: PR 4's code.

---

## Phase 6: Polish (PR 4)

- [X] T021 [P] Docs: `README.md` and `skills/quartz-deck/SKILL.md` describe the column, its views, `qd build set/list/order`, dotted `qd meta set`, `qd meta size`, and `properties.side`, in our own words.
- [X] T022 Run quickstart §1–§5 on a copy of the showcase. Record it in `specs/023-properties-column/checklists/quickstart-run.md`, with SC-001's item-by-item check of every old control's new home.
- [X] T023 Final constitution re-check in `plan.md`; fmt, clippy (`--workspace --all-targets -D warnings`) and `cargo test --workspace --no-fail-fast` green.
- [ ] T024 With the person: on a copy of their talk, set a background, a transition and two builds from the column (SC-006). Nothing about the talk goes into the repository.
- [X] T025 Concision audit (`/pr-audit`), the last task before each PR is marked ready.

---

## Dependencies & Execution Order

- **Setup** (T001–T002) first.
- **US1** (T003–T008) next: it makes the column that every other story fills.
- **US2** (T009–T013) and **US3** (T014–T017) need US1's column, but not each other. Their Rust parts (T009–T010, T014–T015) can be built alongside US1.
- **US4** (T018–T020) needs US1. It is independent of US2 and US3.
- **Polish** last.

### Within each story

Tests first, seen failing; then the ops; then the view; then the story's
tests pass.

## Parallel examples

- During US1: T009–T010 (`qd build …`) and T014–T015 (dotted meta, size) in `ops.rs`/`cli.rs` while the column is built in the editor files.
- US2: T011 (browser tests) alongside T010 (ops).
- Polish: T021 (docs) alongside T022 (quickstart run).

## Implementation strategy

- **MVP**: Setup + US1 (PR 1). Every existing setting is in one column, and the bars are gone.
- **Then**: US2 (Animate), US3 (Deck) and US4 (placement), each its own PR. The showcase deck is updated as each lands.
- Each PR passes `/pr-audit` (T025), and is merged only on the person's say-so.
