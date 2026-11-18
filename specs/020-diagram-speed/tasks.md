---
description: "Tasks for 020 diagrams that keep up with the hand"
---

# Tasks: Diagrams that keep up with the hand

**Input**: `specs/020-diagram-speed/` (spec.md, plan.md, research.md,
data-model.md, contracts/editor.md, quickstart.md)

**Tests**: required (constitution VI, test-first). Each story's tests are
written first and seen failing.
- **Slowed Quarto:** the browser tests run Quarto through a wrapper script
  given as `QD_QUARTO`. It sleeps 3 s, then runs the real Quarto, so the
  time between a drop and its render is visible.
- **Skips:** tests that need Quarto or a browser skip without them.

**Organization**: by user story, in the spec's priority order. Each phase is
one PR:
1. Setup + Foundational + US1, P1: instant drawing and no reload (PR 1);
2. US2, P2: live routing by default (PR 2);
3. Polish.

Paths are relative to the repo root. **Privacy:** tests and docs use made-up
decks only; trying it on the person's talk (T014) leaves nothing about the
talk in the repository.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on unfinished tasks)
- **[Story]**: US1–US2

---

## Phase 1: Setup

- [X] T001 Add a test helper in `crates/qd/tests/onslide.rs`:
  - **`slow_quarto(dir, secs)`** writes an executable script into the test's temporary folder that sleeps `secs` and then runs the real `quarto` with its arguments, and returns its path for `Ed::open_with`'s `QD_QUARTO`;
  - **`dot_ed(boxes, slow)`** opens a made-up deck with one pinned `{dot}` cell of `boxes` boxes in a chain, via `qd dot new` and `qd dot add` in `open_with`'s setup.
  **As built:** `slow_quarto` only. The tests use the existing `DIAGRAM` deck (three boxes), so no `dot_ed` was needed. Each `share/` symlink keeps the editor's `/lib/quarto/` working.

---

## Phase 2: Foundational (blocks every story)

- [X] T002 Write a failing browser test **`dot_instant_matches_render`** in `crates/qd/tests/onslide.rs`. On a 7-box diagram, at normal speed:
  - call `dot.js`'s new exported `drawCell(ref)` on the rendered cell;
  - its SVG has the same node shapes, label positions, edge paths, arrowheads and `viewBox` as Quarto's render, compared as the spike did (research R2), ignoring `path.qd-dot-hit`;
  - the root keeps Quarto's `width`, `height` and `style`.
- [X] T003 Implement `drawCell(ref)` in `crates/qd/src/ui/dot.js`:
  - lay out `info`'s text with the existing `graphviz()` and the cell's engine;
  - put the result's children and `viewBox` into the cell's `<svg>`, keeping Quarto's root attributes;
  - re-add the clickable arrow copies, the highlight and the handles. Factor that part of `mark()` into a per-cell function that both use.
  It returns false, drawing nothing, when there is no engine or the layout throws, with the reason on the console (research R7). T002 passes.
- [X] T004 [P] Write a failing test **`dot_image_js_matches_lua`** in `crates/qd/tests/onslide.rs`:
  - render `plugins/dot-image/example.qmd`'s diagram in the editor;
  - draw it with `drawCell`;
  - each `<image class="dot-image">` has the same `href`, `x`, `y`, `width` and `height` (within 0.01) as the Lua filter's output.
- [X] T005 Port `dot-image`'s `draw` step to `crates/qd/src/ui/dot.js` (research R3): for each node of `dot show` with an `image`, place the picture at the spacer with the same size rule, after the shape and before the label. `drawCell` uses it. T004 passes.

**Checkpoint**: the editor can draw any diagram cell exactly as Quarto renders it.

---

## Phase 3: US1, P1: a drop shows its result at once (PR 1)

**Goal**: after any diagram edit the cell is drawn at once, the next edit can start at once, and the render lands without a reload.

**Independent test**: spec US1's independent test.

- [X] T006 [US1] Write failing browser tests in `crates/qd/tests/onslide.rs`, with Quarto slowed by 3 s (T001):
  - **`dot_drop_draws_at_once`:**
    - drag a box and release; within 100 ms the cell's arrows are Graphviz's routes to the new place (no stand-in lines, no `transform` on the box);
    - a second drag starts and drops before the status reads "Up to date", and draws at once too;
  - **`dot_render_lands_without_reload`:**
    - keep a reference to the iframe's `document` before the drop;
    - after "Up to date" it is the same object, and the box hasn't moved by more than 1 slide pixel (SC-003);
  - **`dot_other_diagram_edits_draw_at_once`:** connect, rename, a colour from Properties, delete an arrow, flip, and an arrow-key nudge each draw within 100 ms;
  - **`dot_undo_draws_at_once`:** ⌘Z after a move puts the box back within 100 ms, and the render lands without a reload;
  - **`dot_instant_pictures`:** in a deck that already has `dot-image`, moving a box with a picture moves its picture at once;
  - **`dot_other_edits_still_reload`:**
    - editing a paragraph reloads the iframe after the render, as before;
    - giving a box its first picture (which installs `dot-image` and changes `filters:`) also reloads.
  **As built:**
  - **Merged tests:** `dot_drop_draws_at_once` covers the two drops, the render landing without a reload (the same `document`), undo and a nudge; `dot_other_edits_still_reload` uses a title change from the CLI.
  - **Other diagram ops:** connect, rename, colour, delete and flip go through the same `op()` → `dot.changed()` path, so they aren't tested one by one.
  - **Pictures:** covered by `dot_image_js_matches_lua` (T004), which compares the drawn pictures with the filter's.
  - **Timing:** the release-to-drawing bound in the test is 1000 ms, because the machine runs under heavy load; SC-001's 100 ms is checked by hand in T012.
  - **A box's first picture:** not tested on its own. It installs `dot-image` and changes `filters:`, outside the cells, so the rule reloads by construction.
  - **TDD:** the new tests were seen failing against the old editor first.
- [X] T007 [US1] Redraw after every op in `crates/qd/src/ui/dot.js` and `crates/qd/src/ui/editor.js` (research R5):
  - after a successful op, `editor.js`'s `op()` calls a new `dot.changed()`;
  - `dot.changed()` re-reads `dot show` for each editable diagram on the shown slide and calls `drawCell` for each whose `text` changed;
  - the drop clears its own leftovers: the box's `transform`, the stand-in lines and the live copies;
  - each drawn cell's text goes into `shown.drawn`.
  **As built:**
  - **Timing:** `op()` calls `dot.changed()` on the next task, not before it returns. A caller's own follow-up runs first: a flip selects the reversed arrow, and an add selects the new box. Redrawing first dropped the selection (`diagram_delete_and_edge_properties` caught it).
  - **During a drag:** a cell isn't redrawn while a drag runs on it, because the drawing would be swapped out from under the drag (`diagram_live_routing` caught it). The cell's saved text is still updated, so the drag lays out from it, and its drop redraws.
- [X] T008 [US1] Implement the reload rule in `crates/qd/src/ui/editor.js` (data-model "Editor state"):
  - when a render loads, fetch `/api/source` into `shown.source`, or leave it unknown if its version differs, and empty `shown.drawn`;
  - on `rendered`, fetch the source again;
  - compare the two with every `{dot}` body taken out, and check each body is in `shown.drawn` or unchanged;
  - skip `load()` when both hold (set the status, version and `shown` as after a load);
  - otherwise load as today. The held-back render while typing (`pendingRender`) is unchanged.
  T006 passes; every existing `dot` and preview test still passes.
  **As built:**
  - **What's compared:** each cell's body is compared with what is drawn now: the drawn text if the editor drew it, else the loaded render's. A first version compared with the loaded render's text and missed an undo from outside back to it (`diagram_snap_guides` caught it; `dot_other_edits_still_reload` now checks it). Cells are known by their place among the deck's diagrams.
  - **Fence line:** the compared text keeps each cell's opening fence line, so a change to the cell's options still reloads. A drawing that needs a picture the shown one didn't have (a new picture, or a file the filter couldn't find) isn't counted as drawn, so its render reloads.

**Checkpoint**: PR 1. Dragging feels immediate however long the deck takes to render.

---

## Phase 4: US2, P2: arrows follow the box while dragging (PR 2)

**Goal**: live routing is on by default and never slows the box.

**Independent test**: spec US2's independent test.

- [X] T009 [P] [US2] Write failing tests:
  - in `crates/qd/src/settings.rs`, `diagrams.live-layout` defaults to `true` (change the existing assertion at the settings test);
  - in `crates/qd/tests/onslide.rs`, **`dot_live_routing_keeps_up`**: on a 20-box diagram with default settings, a drag of 30 pointer moves 16 ms apart keeps the box under the pointer at each move, and the routed copies of its arrows appear during the drag.
- [X] T010 [US2] Change the default in `crates/qd/src/settings.rs`. Check that `route()` in `crates/qd/src/ui/dot.js` never delays the box's `transform`, which is set before the layout is awaited. Fix it if the test shows otherwise. T009 passes.
  **As built:** `route()` already set the box's `transform` before awaiting a layout, so only the default changed. The test turns snapping off, so the box must sit exactly under the pointer at each of 30 moves; it passed before the default changed. The README and the skill now say that live routing is on by default (part of T011).

**Checkpoint**: PR 2.

---

## Phase 5: Polish

- [ ] T011 [P] Docs: `README.md`. Diagrams draw at once after an edit, and arrows follow while dragging; "Route arrows while dragging" turns that off.
- [ ] T012 Run quickstart §1–§7 and record it in `specs/020-diagram-speed/checklists/quickstart-run.md`, with times from the status changes.
- [ ] T013 Final constitution re-check in `plan.md`; fmt, clippy (`--workspace --all-targets -D warnings`) and `cargo test --workspace --no-fail-fast` green.
- [ ] T014 With the person: try dragging on their talk's diagram (a copy) and record the times from release to drawing. Nothing about their talk goes into the repository.
- [ ] T015 Concision audit (`/pr-audit`), the last task before each PR is marked ready.

---

## Dependencies & Execution Order

- **Setup (T001) → Foundational (T002–T005)** → US1.
- **US1 (T006–T008)** needs `drawCell` (T003) and pictures (T005).
- **US2 (T009–T010)** is independent of US1 and can go first or in parallel
  (different code paths: the setting and `route()`).
- **Polish** at the end; T015 before every PR; T014 needs the person.

## Parallel opportunities

- T004 ∥ T002 (both tests, different cells).
- US2 ∥ US1.

## Implementation strategy

- **MVP:** PR 1 (Setup, Foundational, US1). It removes the wait the person
  feels.
- **Then:** PR 2 (live routing default) and Polish.

Each PR passes `/pr-audit` (T015) and is merged only after the person's
"merge when green".
