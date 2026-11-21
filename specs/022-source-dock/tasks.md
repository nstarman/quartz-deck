# Tasks: Dock the source pane by dragging it

**Input**: [spec.md](spec.md), [plan.md](plan.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/editor.md](contracts/editor.md), [quickstart.md](quickstart.md)

**Tests**: required (constitution VI): each story's browser tests are written first and fail before the code.

Order of work follows the plan's PRs: the places (US2, US3) first, since
the drag (US1) only writes the setting they read.

## Phase 1: Setup

- [x] T001 Name the new browser tests `source_dock_*` so `cargo test -p qd --test onslide source_dock` runs them (quickstart); reuse `Ed::open`, `Ed::window`, `press_and_move`, `drag` in crates/qd/tests/onslide.rs

## Phase 2: Foundational (the setting)

- [x] T002 Unit test in crates/qd/src/settings.rs: `source.side` accepts `far-left`, `left`, `right`, `below`; refuses `top`; an old file with `right` still reads `right`; `ui.source_height` is a number
- [x] T003 In crates/qd/src/settings.rs: `source.side` becomes `Kind::Choice(&["far-left", "left", "right", "below"])`, default `left`, title "Source pane place", description "Where the source pane sits: before the slide list, beside it, right of the slide, or below the slide."; add `ui("ui.source_height", Kind::Number…)` next to `ui.source_width`
- [x] T004 Update any assertion that lists the choices in crates/qd/tests/serve.rs and crates/qd/tests/cli.rs (run them; change only what fails)

**PR 1 starts here** (T002–T017).

## Phase 3: User Story 2 — the place is remembered (P1)

**Goal**: the pane shows in the place `source.side` names, after reloads and in every editor.

**Independent test**: set each value through Settings; reload; the pane's rect is where the data-model table says.

- [x] T005 [US2] Browser tests in crates/qd/tests/onslide.rs, 1280×800, pane open: for each of the four values set through `/api/settings`, the pane's bounding rect is left of the nav (`far-left`), between nav and stage (`left`), right of the stage (`right`), under the stage and as wide as it (`below`); `main[data-source]` matches; after a reload it is the same; choosing a value in the Settings window moves the pane at once (reuse the `choose` helper near line 2394)
- [x] T006 [US2] Browser test: type in the pane without pausing, change the place, the unsaved text, caret and scroll are unchanged and save afterwards (SC-004)
- [x] T007 [US2] In crates/qd/src/ui/editor.js `settings.onChange`: replace `classList.toggle('source-right', …)` with `mainEl.dataset.source = v['source.side']`
- [x] T008 [US2] In crates/qd/src/ui/editor.css: replace the three `main.source-right` rules with `main[data-source=far-left]` (pane `order: -2`, splitter `order: -1`), `main[data-source=right]` (`#panel` `order: 2`, splitter `3`, pane `4`; History stays beside the nav, research R2), and the `below` grid: `main[data-source=below] { display: grid; grid-template-columns: auto auto minmax(0, 1fr) auto; grid-template-rows: minmax(0, 1fr) auto auto }`, nav/history/panel `grid-row: 1 / -1` in columns 1, 2, 4, stage row 1, splitter row 2, pane row 3 in column 3, pane `width: auto; height: var(--srch, 35vh)`
- [x] T009 [US2] Remove every other use of `source-right` (`grep -rn source-right crates/`), including `onRight()` in crates/qd/src/ui/editor.js (now `mainEl.dataset.source === 'right'`)

## Phase 4: User Story 3 — splitter, Full width and the inspector in each place (P2)

**Goal**: the existing controls work in each place.

**Independent test**: in each place, the splitter resizes along its axis and is remembered; Full width fills the stage's room; the inspector's order on the right.

- [x] T010 [US3] Browser tests in crates/qd/tests/onslide.rs: in `below`, dragging the splitter up grows the pane's height, ↑/↓ change it by 20 px, `#splitter[aria-orientation=horizontal]`, the height survives a reload and switching to `left` brings back the old width; in `far-left` dragging right grows the width
- [x] T011 [US3] Browser tests: Full width in each of the four places hides `#stage` and `#splitter` and the pane fills the stage's room (for `below`, the whole column); turning it off restores the place and size
- [x] T012 [US3] Browser test at 1440×900 with `ui.inspector` on and the pane `right`: select an element; the rects go stage, `#panel`, pane from left to right, and the inspector shows the element's tabs; with `below` the inspector shows too
- [x] T013 [US3] Browser test at 650×800: for each value the pane is under the stage, `main[data-source]` still holds the setting, and widening to 1280 px puts the pane back (research R6)
- [x] T014 [US3] In crates/qd/src/ui/editor.js: the splitter's axis from the place: a `vertical()` check (`false` for `below` or `matchMedia('(max-width: 700px)')`), `aria-orientation` set on each settings change and media change; `setHeight(px)` like `setWidth` writing `--srch` and `ui.source_height` (clamped to 120 px … window height − 160 px); pointer move uses `r.bottom - ev.clientY` when horizontal; keys ↑/↓ when horizontal; apply `ui.source_height` on load next to `ui.source_width`
- [x] T015 [US3] In crates/qd/src/ui/editor.css: `main[data-source=below] > #splitter` and the narrow-window splitter `height: 6px; width: auto; cursor: row-resize`; `main[data-source=below].source-full > #srcpane { grid-row: 1 / -1; height: auto }`
- [x] T016 [US3] In the `@media (max-width: 700px)` block of crates/qd/src/ui/editor.css: `main[data-source] { display: flex }`, stage `order: 1`, splitter `order: 2`, pane `order: 3; width: auto; height: var(--srch, 35vh)`, `#panel` `order: 4`
- [x] T017 [US3] Run the whole onslide suite at the default place; fix only what this feature broke

**PR 1 ends**: `/pr-audit`, then open the PR.

## Phase 5: User Story 1 — drag the pane to another place (P1)

**Goal**: drag the title bar onto a zone to move the pane.

**Independent test**: drag to each zone; Esc and a drop outside cancel; a button press starts no drag.

**PR 2** (T018–T026).

- [ ] T018 [US1] Browser tests in crates/qd/tests/onslide.rs at 1280×800: `press_and_move` from the "Source" label to the middle of each `#dock-zones > [data-place]` sets `source.side` to that place and moves the pane; during the move `#dock-zones` shows four zones with labels and the one under the pointer has `.over`; after release it is hidden
- [ ] T019 [US1] Browser tests: a drag released outside any zone, and one ended with Esc, leave `source.side` unchanged; pressing on "Full width" and moving starts no drag and the button still toggles; a move under 4 px is a plain click; at 650 px wide only the `below` zone shows
- [ ] T020 [US1] Create crates/qd/src/ui/dock.js: `init(set)`; `pointerdown` on `#srcpane .srcbar` unless `e.target.closest('button, input, select, a')`; after 4 px `setPointerCapture`, add `main.docking`, show the zones; `pointermove` sets `.over` from `document.elementsFromPoint`; `pointerup` over a zone calls `set(place)` if it differs; Esc, `pointercancel` and `lostpointercapture` cancel; zones hidden on every end
- [ ] T021 [US1] In dock.js: place the zones from the live rects each time a drag starts: `far-left` a 64 px strip at `<main>`'s left edge, `left` a strip at the stage's left edge, `right` at `<main>`'s right edge, `below` a strip along the stage's bottom; only `below` when `matchMedia('(max-width: 700px)')` matches; labels "Before the slide list", "Beside the slide list", "Right of the slide", "Below the slide"
- [ ] T022 [US1] Add `<div id="dock-zones" hidden></div>` to crates/qd/src/ui/index.html; serve `/ui/dock.js` the way the other modules are embedded in crates/qd/src/server.rs (`include_str`)
- [ ] T023 [US1] In crates/qd/src/ui/editor.js: `dock.init((place) => settings.set({ 'source.side': place }))`
- [ ] T024 [US1] In crates/qd/src/ui/editor.css: `#dock-zones` fixed, `pointer-events: none` on the container; zones `color-mix` of `--accent` with a dashed `--accent` border and the label centred, `.over` stronger; `main.docking { user-select: none; cursor: grabbing }`; `#srcpane .srcbar { cursor: grab }` on its empty part; check light, dark and `hc` themes (FR-011)

## Phase 6: Polish

- [ ] T025 [P] README.md (the `qd settings set source.side` line near 127) and skills/quartz-deck/SKILL.md: the four places and the drag, one or two sentences
- [ ] T026 Run quickstart.md in the browser pane; final constitution re-check in plan.md; `/pr-audit`, then open PR 2

## Dependencies

- T002–T004 before everything.
- US2 (T005–T009) before US3 (T010–T017): the splitter needs the places.
- US1 (T018–T024) needs only `source.side` and `data-source` (US2), so it can start once PR 1 is merged or be built on its branch.
- Inside each story, the tests first.

## Parallel opportunities

- T002 and T005 (different files) can be written together.
- T010–T013 are independent tests in one file: write them in one pass.
- T025 alongside T020–T024.

## Implementation strategy

MVP is PR 1: every place, chosen in Settings, with the splitter, Full
width and the inspector working. PR 2 adds the drag on top.
