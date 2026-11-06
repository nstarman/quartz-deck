---
description: "Tasks for 008 usability"
---

# Tasks: Usability — the editor made easy to use

**Input**: `specs/008-usability/` (spec.md, plan.md, research.md, data-model.md,
contracts/, quickstart.md, audit/)

**Tests**: required (constitution VI, test-first). Each story's tests are
written first and seen failing.

**Organization**: by user story, in the spec's priority order. P1: US1 (slide
list), US2 (canvas), US3 (readable), US4 (Present), US5 (sheets and images).
P2: US6 (words, one top bar), US7 (bars, Insert), US8 (narrow). P3: US9.
Paths are relative to the repo root.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on unfinished tasks)
- **[Story]**: US1–US9

---

## Phase 1: Setup

- [X] T001 [P] Write `specs/008-usability/audit/status.md`: one row per audit finding (L1–L5, C1–C6, E1–E3, B1, S1–S2, D1–D2, T1–T5, H1–H4, N1–N2) with its requirement(s), the task that fixes it, and a status column (`open`). Updated as tasks land; the evidence column is filled in T039.
- [X] T002 [P] The browser tests use the audit deck itself (`include_str!` of `specs/008-usability/audit/deck/talk.qmd`), so the tests and the audit can't drift apart.

---

## Phase 2: Foundational (blocks US1, US8, US9; T004 is the yardstick for US2, US3, US7)

- [X] T003 (Done together with T006–T009: `list.js` was written once, in its new form, rather than moved and then rewritten.) Move the slide list out of `src/ui/editor.js` into a new `src/ui/list.js` (rendering, drag, the hover pop-up, "+ Add slide"), with the editor calling it through a small interface (`render(deck, current)`, `onPick(id)`, `focus()`); add it to the served UI modules in `src/server.rs`. No behaviour change: every existing browser test stays green.
- [X] T004 [P] Write the failing measurement test `audit_measures` in `tests/onslide.rs` on the audit fixture. In light, dark and high contrast it walks every visible control and text in the editor and fails on:
  - contrast below 4.5:1 (3:1 for ≥18 pt or bold);
  - an interactive target below 20×20 px;
  - a bar overlapping another `[data-qd]` element as each element is selected and then edited.

  It reports each failure with selector, value and theme, and today's failures match the audit's (T1, T2, T3, C1).

**Checkpoint**: the list has its own module; the measurement test runs and fails as the audit predicts.

---

## Phase 3: User Story 1 — The slide list works like a slide app's (P1) 🎯 MVP

**Goal**: thumbnails, title slide first, + in every gap, untitled new slides with a placeholder, a drag you can see.

**Independent Test**: spec US1; quickstart §1.

- [X] T005 [P] [US1] Write failing browser tests `slide_list_like_a_slide_app` in `tests/onslide.rs` (audit fixture):
  - every item has a thumbnail (a shadow root holding a copy of its section), and editing slide 2's text refreshes only slide 2's thumbnail;
  - the title slide is first, unnumbered, not draggable;
  - hovering the gap between items 2 and 3 shows a +, and clicking it adds an untitled slide at index 3;
  - Enter on a focused item adds one after it;
  - the new slide's empty title shows "Click to add title" on the canvas, and the file has `## {#s-…}` with no placeholder text;
  - dragging item 6 by its handle to second shows a drag image and a gap, Escape cancels (order unchanged), and dropping moves it with one undo step;
  - ⌥↑/⌥↓ on a focused item move it;
  - on a 40-slide deck, dragging near the bottom edge scrolls the list.
- [X] T006 [US1] Thumbnails in `src/ui/list.js` (research R1):
  - a copy of each slide's `<section>` from the preview frame, in a shadow root, sharing one constructed stylesheet made from the deck's styles per render;
  - relative `src`/`href` made absolute, and the slide's background colour/image on the box;
  - built for items in view only (IntersectionObserver), rebuilt when a render lands and the slide's source hash changed;
  - remove the hover pop-up (`#peek` in `editor.js`).

  A `ponytail:` comment notes the ceiling (script-drawn outputs show static).
- [X] T007 [US1] The title slide as the list's first item when `qd slides` reports `title_slide`: unnumbered, not draggable, not deletable from the list (research R2).
- [X] T008 [US1] Adding anywhere (research R3) in `src/ui/list.js`:
  - a + in each gap on hover and after the last item, each running `qd add --after/--before` without a title;
  - Enter on a focused item runs `slide-after`;
  - "+ Add slide" goes;
  - in `PREVIEW_CSS` (`src/ui/editor.js`), `section > h2:empty::before { content: "Click to add title" }` in the muted colour, editor only;
  - clicking it starts the existing title edit.
- [X] T009 [US1] Pointer-driven drag in `src/ui/list.js` (research R4), replacing HTML5 drag and drop:
  - a ⠿ handle on hover (target ≥20 px), and dragging starts after 4 px of movement;
  - a translucent copy of the thumbnail follows the pointer;
  - items move apart (transform with a 120 ms transition) to open a gap at the landing index;
  - the list auto-scrolls within 40 px of its edges, and Escape cancels;
  - the drop runs one `qd move`.

  In `src/ui/actions.json`, give `slide-move-up`/`slide-move-down` keys `Alt+ArrowUp`/`Alt+ArrowDown` where `slides` (check `tests/actions.rs` for clashes with the element's `up`/`down`; they apply in different places).
- [X] T010 [US1] List styles in `src/ui/editor.css`: 16:9 thumbnail boxes at the list's width, the number and title under each, the current slide's ring, the gap +, the drag states, and focus rings; checked against `audit_measures` for the list's controls. T005 passes.

**Checkpoint**: the slide list is usable on its own (MVP).

---

## Phase 4: User Story 2 — Nothing covers the slide (P1)

**Independent Test**: spec US2; quickstart §2.

- [X] T011 [P] [US2] Write failing browser tests `nothing_covers_the_slide` in `tests/onslide.rs`:
  - selecting and then editing the first element under the title on each slide puts no bar over another element (the overlap part of `audit_measures`, asserted per slide here);
  - the preview has no visible `.slide-menu-button`, `.controls` or `.progress`;
  - the canvas's top edge is the same on the title slide and on a content slide.
- [X] T012 [US2] One placement rule for the element bar and the formatting bar (research R5) in `src/ui/editor.js` and `src/ui/toolbar.js`: above, else below, else docked at the top of the canvas area; reject a position that intersects another element or the slide's heading. Keep the ResizeObserver re-placement.
- [X] T013 [US2] In the frame hook (`src/ui/editor.js`), `Reveal.configure({controls: false, progress: false, slideNumber: false})` and hide `.slide-menu-button`; the slide bar keeps its row on every slide (`visibility: hidden` and disabled on the title slide) (research R6). T011 passes.

---

## Phase 5: User Story 3 — Everything can be read and reached (P1)

**Independent Test**: spec US3; quickstart §3; `audit_measures` contrast and size parts.

- [X] T014 [US3] Themes and sizes in `src/ui/editor.css` (and `PREVIEW_CSS` in `src/ui/editor.js`) (research R10):
  - `color-scheme` per theme;
  - theme colours for `select`, `input`, `textarea`, checkboxes and the colour swatch;
  - the source highlight colours as tokens with a light, dark and high-contrast value each, each ≥4.5:1 on the pane (record the chosen values and ratios in research R10);
  - one control height token (28 px) for bar buttons, selects and toggles;
  - ≥20 px targets for the gutter ▶, checkboxes and box handles (visible size may be smaller, with a larger hit area);
  - selection and editing outlines from the theme's accent, passed into the frame.

  `audit_measures` contrast and size parts pass.

---

## Phase 6: User Story 4 — Present is one click (P1)

**Independent Test**: spec US4; quickstart §4.

- [X] T015 [P] [US4] Write failing browser test `present_from_here_and_from_start` in `tests/onslide.rs`:
  - with `Element.prototype.requestFullscreen` stubbed to record its element, Present on slide 4 asks for the preview frame full screen at slide 4, with reveal's keyboard on;
  - "From the beginning" goes to the title slide;
  - a `fullscreenchange` back restores the editing configuration (no controls, Escape the editor's).
- [X] T016 [US4] Actions `present` ("Present from this slide", ⌥⌘P) and `present-start` ("Present from the beginning", ⇧⌥⌘P) in `src/ui/actions.json` (View), checked against `vscode/default-keys.json` by the node test.

  A ▶ Present button with a ▾ in the header (`src/ui/index.html`, `src/ui/menubar.js`). Full screen in place in `src/ui/editor.js` (research R7). `preview` relabelled "Open in browser tab". T015 passes.

---

## Phase 7: User Story 5 — The editor's own dialogs (P1)

**Independent Test**: spec US5; quickstart §5.

- [X] T017 [P] [US5] Write failing Rust tests:
  - **`src/images.rs` units:** name reduced to `[A-Za-z0-9._-]` (no path parts); a taken name gets `-1`, `-2`; a type outside png/jpg/jpeg/gif/svg/webp, or magic bytes that don't match it, is `E_INVALID_ARG`; more than 20 MB is refused; listing walks ≤3 levels, skips hidden folders, qd's cache and `export/`, and sorts newest first.
  - **`tests/actions.rs`:** the static scan fails on any `prompt(`, `confirm(` or `alert(` in `src/ui/*.js`.
- [X] T018 [P] [US5] Write failing tests:
  - **`tests/cli.rs` `images_list_and_add`:** `qd images`; `qd image add chart.png` (path only); `qd image add chart.png --slide growth --end` (the block inserted, one undo step); adding the same file twice gives `images/chart-1.png`.
  - **`tests/serve.rs` `images_over_http`:** `GET /api/images`; `POST /api/images?name=…` gives 403 without the token or with another Origin, 400 for a bad type or bad magic bytes, and 200 with a free name; nothing is overwritten.
- [X] T019 [US5] Implement `src/images.rs` (list, add), `qd images` and `qd image add` in `src/cli.rs` (plus `qd schema` entries), and `GET`/`POST /api/images` in `src/server.rs` (raw body, `application/octet-stream`, `authorized()`, 20 MB limit) per contracts/http.md and contracts/cli.md. T017 and T018 pass.
- [X] T020 [P] [US5] Write failing browser test `own_sheets_and_image_drop` in `tests/onslide.rs`:
  - with `window.prompt`/`confirm`/`alert` replaced by functions that record a call, Insert ▸ Image… opens a sheet listing `chart.svg` with a preview, and choosing it inserts the image;
  - a PNG dropped on the canvas (a synthetic `DataTransfer` holding a `File`) lands where dropped, and `images/` holds it;
  - the link sheet refuses `not a url` with a message;
  - the background colour sheet offers None and it removes the colour;
  - none of the native functions was called.
- [X] T021 [US5] `src/ui/sheet.js` (research R8, data-model "Sheet"): an anchored `<dialog>` with fields (text, swatches with None and Custom…, file list with previews, check), inline validation, OK/Cancel, Escape and Enter.

  Replace all nine native boxes:
  - `src/ui/actions.js`: custom class, background colour, background image, delete slide;
  - `src/ui/run.js`: packages;
  - `src/ui/toolbar.js`: link;
  - `src/ui/editor.js`: image, About;
  - `src/ui/settings.js`: reset.

  Image drop on the canvas in `src/ui/editor.js` (upload, then `block add` at the drop point). T017's scan and T020 pass.
- [X] T022 [US5] Security: add `POST /api/images` to `another_origin_cannot_run_code_or_edit` (no file appears in `images/`). Write `specs/008-usability/checklists/security.md`: the upload's defences, the mutation check (with the token check removed, which test catches it), and the SVG note from research R9.

---

## Phase 8: User Story 6 — Plain words and one top bar (P2)

**Independent Test**: spec US6; quickstart §6.

- [X] T023 [P] [US6] Write failing browser test `one_top_bar_in_plain_words` in `tests/onslide.rs`:
  - one bar above the slide area holds the menus, the deck's title, the status, Present and Source;
  - `document.title` is the deck's title, and the file name shows;
  - the status reads "Up to date" after a render and "Saving…"/"Saved" around an edit, with the version as its tooltip;
  - framed (`?host=vscode`), ☰ is in that same row;
  - none of the audit's jargon labels appears in any menu or bar.
- [X] T024 [US6] One top bar in `src/ui/index.html`, `src/ui/menubar.js` and `src/ui/editor.css` (menus or ☰ · title · file name · status · Present · Source). Status words and `document.title` in `src/ui/editor.js` (research R11).
- [X] T025 [US6] Plain labels in `src/ui/actions.json`, the bars and the right-click menus (research R11's list); "Edit markdown" for an element's source. Update `specs/007-app-shell/checklists/action-inventory.md` and every test that clicks an old label. T023 passes.

---

## Phase 9: User Story 7 — Tidier bars and a fuller Insert (P2)

**Independent Test**: spec US7; quickstart §7.

- [ ] T026 [P] [US7] Write failing browser test `tidier_bars_and_fuller_insert` in `tests/onslide.rs`:
  - the slide bar's controls share one height and each has a label or title, and its colour menu offers None;
  - the formatting bar shows bold, italic, underline, strikethrough, link, list, colour and "Aa", and "Aa" opens the rest;
  - the source bar shows ▶ Run all, ■ Stop, "Python · stopped" and ···, and ··· holds Restart, Restart & run all, Add Python package…, Set up Python…;
  - Insert's Code cell, Callout, Columns, Speaker notes, Table and Quote each add their markdown (checked in the file);
  - a positioned box's handles have targets ≥20 px.
- [ ] T027 [US7] The slide bar in `src/ui/editor.js`/`src/ui/index.html`: the colour as a swatch menu through `sheet.js` (None, Custom…), one control height, a label or title on every control (research R12).
- [ ] T028 [US7] The formatting bar in `src/ui/toolbar.js`: the common formats plus "Aa" (code, numbered list, indent, outdent, size, alignment as a segmented control); colour names in titles.
- [ ] T029 [US7] The source bar in `src/ui/run.js`: ▶ Run all, ■ Stop, the session state in words, a ··· menu for the rest; Full width and Properties as small toggles at the pane's top right; the gutter ▶ ≥20 px.
- [ ] T030 [US7] Insert's new blocks: `insert-code`, `insert-callout`, `insert-columns`, `insert-notes`, `insert-table`, `insert-quote` in `src/ui/actions.json` and the editor's insert list, each a markdown snippet through `block add`. Add them to the action inventory.
- [ ] T031 [US7] Box handles: visible 10 px squares with 20 px hit areas in `src/ui/editor.js` (`PREVIEW_CSS` and overlay). T026 passes.

---

## Phase 10: User Story 8 — A narrow window still works (P2)

**Independent Test**: spec US8; quickstart §8.

- [ ] T032 [P] [US8] Write failing browser test `narrow_window` in `tests/onslide.rs` at 420×760:
  - a slide picker in the top bar reaches every slide;
  - the slide's properties are in ···;
  - no element overflows the window (`scrollWidth <= clientWidth` for the body and each bar);
  - editing text works.
- [ ] T033 [US8] Under 600 px (research R13): a media query in `src/ui/editor.css` hides the list; the slide picker (current slide's number and title, a menu of thumbnails) goes in `src/ui/list.js`; the slide bar's controls move into ···. T032 passes.

---

## Phase 11: User Story 9 — Finishing touches (P3)

**Independent Test**: spec US9; quickstart §9.

- [ ] T034 [P] [US9] Write failing tests:
  - **`tests/ops.rs`:** `move` with three slides `--after x` keeps their order and is one patch; `rm` with two slides is one patch.
  - **`tests/cli.rs`:** `qd move a b --after c` and `qd rm a b`, each undone in one step.
- [ ] T035 [US9] Several slides for `ops::mv` and `ops::rm` in `src/ops.rs`, and `qd move`/`qd rm` in `src/cli.rs` (positional slides, one or more); skill table updated. T034 passes.
- [ ] T036 [P] [US9] Write failing browser test `finishing_touches` in `tests/onslide.rs`:
  - ⇧-click selects three slides, dragging them moves all three, and one undo restores them;
  - ⌘-click, then Delete, removes two;
  - Settings has Theme first and its controls share one width;
  - an on-slide edit shows "Saved" near the element;
  - undoing an on-screen duplicate leaves the canvas on the slide it was copied from.
- [ ] T037 [US9] Multi-select in `src/ui/list.js` (move, copy, cut, delete apply to the set; one op each). Theme first in `src/settings.rs` `DEFS` and one control width in `src/ui/editor.css`. "Saved" label in `src/ui/editor.js`. Undo-after-duplicate goes to the source slide (`goAfterRender`). T036 passes.

---

## Phase 12: Polish & cross-cutting

- [ ] T038 [P] Docs: the README (the slide list, Present, images and the image sheet, plain menu names); the skill (`qd images`, `qd image add`, several slides for `move`/`rm`); `vscode/README.md` (the one top bar); `specs/ROADMAP.md` 008 status.
- [ ] T039 Rerun `uv run specs/008-usability/audit/shoot.py` (P1 and P2 states). In `audit/status.md`, mark each finding resolved, with the new screenshot and measurement as evidence, or say why not (SC-010).
- [ ] T040 Run quickstart §1–§9 in the browser pane and record it in `specs/008-usability/checklists/quickstart-run.md`.
- [ ] T041 Final constitution re-check in `plan.md`; fmt, clippy, `cargo test` (with `audit_measures` at 0 failures) and `node --test vscode/test/*.test.js` all green.
- [ ] T042 Concision audit (`/pr-audit`), the last task before the PR is marked ready.

---

## Dependencies & Execution Order

- **Setup (T001–T002)** first; both [P].
- **Foundational:** T003 (list module) blocks US1, US8 and US9's list work. T004 (measurement test) is the yardstick that US2, US3 and US7 make pass.
- **US1 (T005–T010):** after T003. The MVP.
- **US2 (T011–T013)**, **US3 (T014)** and **US4 (T015–T016)**: independent of US1 and of each other (different code areas; US3 is CSS).
- **US5 (T017–T022):** independent. T019 (server/CLI) before T021's upload. T021's `sheet.js` is used by US7's colour menu (T027).
- **US6 (T023–T025):** after US4 (the Present button moves into the one bar). T025's relabels touch tests other stories also edit, so it goes after US1–US5 have merged.
- **US7 (T026–T031):** after US5 (sheet.js) and US3 (the control height token).
- **US8 (T032–T033):** after US1 (list.js) and US6 (the top bar it puts the picker in).
- **US9 (T034–T037):** T034–T035 (CLI) any time; T036–T037 after US1.
- **Polish (T038–T042)** last; T039 needs all P1/P2 stories.

## Parallel opportunities

- T001 ∥ T002 ∥ T004.
- After Foundational: US1 ∥ US2 ∥ US3 ∥ US4 ∥ US5 (different files: list.js / placement in editor.js and toolbar.js / CSS / menubar and present / images, sheet.js). Within US5: T017 ∥ T018 ∥ T020.
- T034–T035 (Rust) ∥ any editor story.

## Implementation strategy

1. **MVP:** Setup, Foundational, US1. Ship as its own PR: the slide list the person asked for first.
2. **P1 rest:** US2 and US3 (one PR: the canvas and contrast, judged by `audit_measures`), US4, US5 (one PR each).
3. **P2:** US6, then US7, then US8.
4. **P3:** US9.
5. **Polish:** the audit rerun and the status table close the feature.
