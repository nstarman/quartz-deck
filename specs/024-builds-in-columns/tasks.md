# Tasks: Builds inside columns

**Input**: Design documents from `specs/024-builds-in-columns/`

**Tests**: requested (spec SC-003; constitution VI: test-first core).

## Phase 1: Setup

- [ ] T001 Branch `024-builds-in-columns` from `origin/main`; spec, plan and tasks committed in `specs/024-builds-in-columns/`.

## Phase 2: Foundational — nested blocks in the deck model (PR 1)

- [ ] T002 Tests first in `crates/qd/src/deck.rs`: a two-column slide gives `r/0` children `r/0/0`, `r/0/1`, each with its elements (`r/0/0/0` list, `r/0/1/0` image); shorthand `:::: columns` too; a `::: {.fragment}` wrapper and a callout have no children; columns in a column nest; an explicit `{#id}` in a column keeps it; round trip stays byte-identical; `element()` resolves `r/0/1/0`, `#id` at depth, and gives `E_NOT_FOUND` for `r/0/9/0` and `r/1/0` into a non-columns block; `E_AMBIGUOUS` for an id twice at any depth.
- [ ] T003 `Block.children: Vec<Block>` in `crates/qd/src/deck.rs`, filled for a div whose first class is `columns` or `column` by running `scan` + `classify` over its inner lines; child ids are `<parent id>/<i>` unless explicit. One function builds blocks from chunks for both the slide and a div. `Deck::element` splits at the first `/` and walks the path; the id search and `explicit_ids` cover every depth. `Slide.blocks` and `Deck::blocks()` stay top-level.

## Phase 3: User Story 2 — the CLI with nested refs (P2, PR 1)

**Goal**: build set/list/order, block class and edit take nested refs; rearranging ops refuse them.

**Independent test**: `cargo test` — `crates/qd/tests/cli.rs` scenarios on a two-column fixture.

- [ ] T004 [US2] Tests in `crates/qd/tests/cli.rs` for spec US2 scenarios 1–6: `qd show` lists `children` with refs; `build set` on `r/0/0/0` and `r/0/1/0` changes only those lines (the list wrapped in `::: {.fragment}` inside its column); `build list` gives both refs; `build order r r/0/1/0 r/0/0/0` sets `fragment-index`, source order removes it; `block class` on a nested ref; `block move/dup/rm/place` on a nested ref fail with `E_INVALID_ARG` "inside columns" and leave the file unchanged; a ref past a column's end is `E_NOT_FOUND`.
- [ ] T005 [US2] `crates/qd/src/ops.rs`: `build_list` and `build_order`'s source order walk blocks in pre-order (a `fn walk(blocks) -> impl Iterator` in `deck.rs`); `Ctx::new` collects ids at every depth; `top_element(d, r)` returns `E_INVALID_ARG` "'<ref>' is inside columns: <verb> it in the source for now" for a block not in `slide.blocks`, used by `on_slide`, `block_move`, `block_rm`, `block_dup`, `block_place`, `block_unplace`, `edit_box`, `box_rm`, `box_order`, and restore/run of an element where they look up by position.
- [ ] T006 [US2] `crates/qd/src/cli.rs`: `block_json` adds `"children"` when non-empty; schema text for `show` and the element ref help (`<slide>/<index>[/<index>…] or #id`).
- [ ] T007 [P] [US2] `skills/quartz-deck/SKILL.md`: nested refs, which ops take them, an example of building a two-column slide.
- [ ] T008 [US2] fmt, clippy `--all-targets -D warnings`, `cargo test`; `/pr-audit`; open PR 1.

## Phase 4: User Story 1 — select and build in the editor (P1, PR 2)

**Goal**: click the list and the picture in two columns, give each a build, reorder, present in that order.

**Independent test**: browser tests in `crates/qd/tests/onslide.rs`.

- [ ] T009 [US1] Tests in `crates/qd/src/render.rs`: `preview_copy` tags `r/0`, `r/0/0`, `r/0/0/0`, `r/0/1/0`; a cell in a column is copied verbatim; slides without columns are unchanged (existing tests).
- [ ] T010 [US1] Browser tests in `crates/qd/tests/onslide.rs`: clicking the list selects `r/0/0/0` and only it has `.qd-sel`; Appear in the Animate view writes the build; both builds listed; ⌥↑ reorders; presenting shows the picture then the list; Escape goes `r/0/0/0` → `r/0/0` → `r/0` → none; Delete and Move up are disabled for a nested element with the reason.
- [ ] T011 [US1] `crates/qd/src/render.rs`: the per-block tagging loop becomes a function over a block list that, for a block with children, tags the opening fence and recurses over the inner text; nested `Cell` blocks verbatim.
- [ ] T012 [US1] `crates/qd/src/ui/editor.js`: `elementTarget(ref)` splits at the first `/`, walks `blocks`/`children`, and returns `nested` and `parent`; the `deselect` key selects `parent` when there is one.
- [ ] T013 [US1] `crates/qd/src/ui/actions.js`: cut, delete, dup, paste, move up/down, insert before/after, place, unplace, box order and restore are disabled for `t.nested`, with `why` "Inside columns: edit the source to move it".

## Phase 5: User Story 3 — the showcase (P3, PR 2)

- [ ] T014 [US3] `examples/showcase/showcase.qmd`: a "Builds inside columns" slide, a list in the left column and a picture in the right, each with a build, the picture first by `fragment-index`.
- [ ] T015 [US3] Rebuild `target/release/qd`, restart `qd-showcase` (port 4880), step through the slide in the browser pane.

## Phase 6: Polish

- [ ] T016 Quickstart run (`quickstart.md`), constitution re-check; fmt, clippy, `cargo test` green.
- [ ] T017 Concision audit (`/pr-audit`) before each PR is marked ready.

---

## Dependencies & Execution Order

- T001 → T002–T003 (model) → US2 (T004–T008, PR 1) → US1 (T009–T013) → US3 (T014–T015) → Polish (PR 2).
- US1 needs US2's ops: every editor control writes through them.

## Parallel examples

- T007 (skill) alongside T005–T006.
- T009 (render tests) alongside T012–T013 (editor JS).

## Implementation strategy

- **PR 1**: model + CLI (US2). The editor is unchanged in behaviour.
- **PR 2**: editor (US1) + showcase (US3). Merged only on the person's say-so, after green CI.
