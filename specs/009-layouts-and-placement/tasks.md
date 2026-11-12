---
description: "Tasks for 009 layouts and placement"
---

# Tasks: Layouts and placement — start from a layout, place anything anywhere

**Input**: `specs/009-layouts-and-placement/` (spec.md, plan.md, research.md,
data-model.md, contracts/, quickstart.md)

**Tests**: required (constitution VI, test-first). Each story's tests are
written first and seen failing.

**Organization**: by user story, in the spec's priority order. P1: US1
(layouts), US2 (free placement). P2: US3 (own layouts), US4 (guides). P3: US5
(stacking). Paths are relative to the repo root.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on unfinished tasks)
- **[Story]**: US1–US5

---

## Phase 1: Setup

- [X] T001 Add `src/layouts.rs` (empty module, `pub mod layouts;` in `src/lib.rs`) and the folder `src/layouts/` for the built-in layout files.

---

## Phase 2: Foundational (blocks US1 and US3)

- [X] T002 Write failing unit tests in `src/layouts.rs`:
  - a layout file parses into name, description and slide markdown;
  - name rule `[a-z0-9-]{1,40}`;
  - its boxes (kind from `ph=`, purpose from the comment, `left/top/width/height`) are listed;
  - scaling 1050×700 → 1280×720 multiplies each coordinate and rounds;
  - lookup order is deck, then the person's, then built-in.
- [X] T003 Implement the layout file format in `src/layouts.rs` (data-model "Layout"): `parse`, `boxes`, `scale`, `list(deck_dir, config_dir)`, `find(name, …)`. T002 passes.

**Checkpoint**: layouts can be read; no deck change yet.

---

## Phase 3: User Story 1 — A new slide from a layout (P1) 🎯 MVP

**Independent Test**: spec US1; quickstart §1–§2.

- [X] T004 [P] [US1] Write the 12 built-in layouts in `src/layouts/` (title, title-content, two-columns, three-columns, image-text, text-image, full-image, section, big-number, quote, title-notes, blank) per research R1/R2, each with a description and a purpose comment per placeholder; a unit test parses all 12 and checks each has the placeholders its name promises.
- [X] T005 [P] [US1] Write failing tests:
  - **`src/ops.rs` units:** `add_layout` inserts one slide (one patch); each box gets a fresh id; it returns placeholder refs with kind and purpose; `--title` fills the title.
  - **More `src/ops.rs` units:** `edit` of a block marked `.qd-ph` drops `.qd-ph` and `ph=` in the same patch; a placeholder is listed in `show_json` as `"placeholder"`.
  - **`tests/cli.rs` `layouts_list_and_add`:** `qd layouts`; `qd add --layout two-columns --after intro --title X` then `qd edit <ref> --md …` fills it; undo removes the slide in one step; an unknown layout is `E_NOT_FOUND`.
  - **SC-002 render check:** every built-in added unfilled renders with Quarto (when present) and the output holds no placeholder words.
- [X] T006 [US1] Implement `ops::add_layout` and placeholder fill in `src/ops.rs`; `qd layouts` and `qd add --layout` in `src/cli.rs` (and `qd schema`); `GET /api/layouts` in `src/server.rs`; `placeholder` in `show_json`. T005 passes.
- [X] T007 [P] [US1] Write failing browser test `slide_from_a_layout` in `tests/onslide.rs`:
  - the + opens the gallery with previews and the last used preselected;
  - Enter adds "two-columns";
  - both text placeholders show "Click to add text" on the canvas only;
  - typing into each fills it;
  - an image dropped on an image placeholder fills it;
  - ⌘Z once removes the slide.
- [X] T008 [US1] The gallery: a gallery field in `src/ui/sheet.js` (previews drawn from the layout's boxes, arrows/Enter/Esc); Insert ▸ Slide after…/before…, the list's + and ⇧⌘N open it (`src/ui/editor.js`, `src/ui/list.js`, `src/ui/actions.json`); `ui.last_layout` in `src/settings.rs`. Placeholder prompts in `PREVIEW_CSS`; typing into an empty box; an image dropped on an image placeholder replaces it (`src/ui/editor.js`). T007 passes.

**Checkpoint**: US1 works alone (MVP).

---

## Phase 4: User Story 2 — Place any element anywhere (P1)

**Independent Test**: spec US2; quickstart §3–§4.

- [X] T009 [P] [US2] Write failing tests:
  - **`src/ops.rs` units:** `block_place` wraps a paragraph, list, quote, equation, code cell and table, and puts `.absolute` attributes on an image, with the block's bytes unchanged; `block_unplace` restores each byte for byte; refusals (div, notes, box, a box with extra attributes for unplace) give their reasons; a fragment keeps its class.
  - **`tests/cli.rs` `place_and_unplace`:** `qd block place results/1 --at 600,400`, then `qd box move --by`, then `qd block unplace` → the file equals the original apart from the move-free round trip; one undo each.
- [X] T010 [US2] Implement `ops::block_place` and `ops::block_unplace` (research R4) and `qd block place/unplace` in `src/cli.rs`. T009 passes.
- [X] T011 [P] [US2] Write failing browser test `place_freely` in `tests/onslide.rs`:
  - ⌥-drag a list: a copy follows the pointer and the drop places it there (within 2 px);
  - the handles resize it;
  - arrow keys nudge (⇧ by 10);
  - "Put back in the flow" restores the file;
  - "Place freely" is disabled on the columns block, with its reason as the tooltip.
- [X] T012 [US2] Editor: ⌥-drag to place, Arrange menu (Place freely, Put back in the flow) and right-click entries, nudge actions `box-nudge-*` in `src/ui/actions.json`, `src/ui/editor.js`; an off-slide element gets a dashed outline. T011 passes.

**Checkpoint**: US1 + US2 = the P1 scope.

---

## Phase 5: User Story 3 — The person's own layouts (P2)

**Independent Test**: spec US3; quickstart §5.

- [X] T013 [P] [US3] Write failing tests:
  - **`src/layouts.rs` units:** `save` turns a slide into a layout: the title becomes a placeholder; boxes become placeholders at their place; flow blocks become placeholders at the `at` given (or stacked full width); notes stay.
  - **More `src/layouts.rs` units:** name rules; no overwrite without `replace`; `rm`/`rename` refuse built-ins.
  - **`tests/cli.rs` `own_layouts`:** `qd layout save`, `--deck`/`--mine`, `rm`, `rename`.
  - **`tests/serve.rs` `layouts_over_http`:** 403 without the token, 409 on a taken name.
- [X] T014 [US3] Implement `layouts::save/rm/rename`, `qd layout …` in `src/cli.rs`, `POST /api/layouts` in `src/server.rs` (`authorized()`; writes only `<config>/layouts/` or `_layouts/`). T013 passes.
- [X] T015 [P] [US3] Write failing browser test `save_a_layout`: Slide ▸ Save slide as layout… asks a name and where (this deck / mine); the gallery lists it under its heading; rename and delete from the gallery; a slide from it has the placeholders in place.
- [X] T016 [US3] Editor: the save sheet (measures flow elements' places for `at`), the gallery's sections and rename/delete (`src/ui/sheet.js`, `src/ui/editor.js`, `src/ui/actions.json`). T015 passes.
- [X] T017 [US3] Security: add `POST /api/layouts` to `another_origin_cannot_run_code_or_edit` (no file in `_layouts/` afterwards); a note in `specs/009-layouts-and-placement/checklists/security.md` (names, folders, mutation check).

---

## Phase 6: User Story 4 — Guides while dragging (P2)

**Independent Test**: spec US4; quickstart §6.

- [X] T018 [P] [US4] Write failing browser test `guides_and_snapping`:
  - dragging a box near the slide's centre shows a guide and lands it exactly centred;
  - near another box's edge, it aligns;
  - ⌘ held, or the setting off: no guide, no snap.
- [X] T019 [US4] Guides and snapping in `src/ui/editor.js` (research R5: candidate lines, 6 px, a 1 px guide), `placement.snap` in `src/settings.rs` with View ▸ Snap to guides. T018 passes.

---

## Phase 7: User Story 5 — Front and back (P3)

**Independent Test**: spec US5; quickstart §7.

- [X] T020 [P] [US5] Write failing tests:
  - **`src/ops.rs` units:** `box_order` front/back/forward/backward moves only that box among the slide's boxes.
  - **`tests/cli.rs`:** `qd box front` one undo.
  - **Browser:** Arrange ▸ Bring to front puts the lower box on top on the canvas.
- [X] T021 [US5] Implement `ops::box_order`, `qd box front|back|forward|backward`, the four Arrange actions with keys in `src/ui/actions.json` (checked by the node test). T020 passes.

---

## Phase 8: Polish & cross-cutting

- [X] T022 [P] Docs: README (layouts, placing, own layouts), the skill (`qd layouts`, `qd add --layout`, filling placeholders, `qd block place/unplace`, `qd box front…`), `specs/ROADMAP.md` 009 status, the 007 action inventory.
- [X] T023 Run quickstart §1–§7 in the browser pane and record it in `specs/009-layouts-and-placement/checklists/quickstart-run.md`.
- [X] T024 Final constitution re-check in `plan.md`; fmt, clippy, `cargo test` (with `audit_measures` at 0 failures, the gallery and Arrange menu included) and `node --test vscode/test/*.test.js` green.
- [X] T025 Concision audit (`/pr-audit`), the last task before each PR is marked ready.

---

## Dependencies & Execution Order

- **Setup (T001) → Foundational (T002–T003)** → US1 and US3, which read layouts.
- **US1 (T004–T008):** after Foundational.
- **US2 (T009–T012):** independent of US1 (box ops only); can run alongside it.
- **US3 (T013–T017):** after US1 (it adds slides from saved layouts through `add_layout`); US2 helps but isn't needed.
- **US4 (T018–T019) and US5 (T020–T021):** after US2 (they act on placed elements).
- **Polish:** after the stories it documents.

Within each story: tests first (seen failing), then the core and CLI, then
the server, then the editor.

## Parallel opportunities

- T004 (layout files) ∥ T005 (tests) ∥ T009 (placement tests).
- US1 (`src/layouts*`, `add_layout`, gallery) ∥ US2 (`block_place/unplace`,
  the drag): different functions and editor areas.
- US4 ∥ US5 once US2 lands.

## Implementation strategy

1. **MVP:** Setup, Foundational, US1. A new slide from a layout, from the
   editor and from `qd`. One PR.
2. **P1 rest:** US2, placing freely. One PR.
3. **P2:** US3 (own layouts) and US4 (guides), one PR each.
4. **P3:** US5. Then the polish PR.

Each PR passes the concision audit before it is marked ready (T025).
