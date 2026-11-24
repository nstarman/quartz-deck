# Tasks: Arrange elements inside columns

**Input**: Design documents from `specs/025-arrange-in-columns/`

**Tests**: requested (spec SC-003; constitution VI).

## Phase 1: Setup

- [X] T001 After #137 merges, branch `025-arrange-in-columns` from `origin/main`; commit spec, plan and tasks.

## Phase 2: Foundational (PR 1)

- [X] T002 Tests first in `crates/qd/src/deck.rs`: `Deck::siblings` gives the list and index for a top-level ref, a column's element, a column, and an `#id` inside a column.
- [X] T003 `Deck::siblings(r)` in `crates/qd/src/deck.rs` (research R1); `rearrange` and `on_slide` in `crates/qd/src/ops.rs` take a list instead of the slide (R2); `cut` and `insert` span helpers (R3).

## Phase 3: User Story 6 — the CLI (P1, PR 1)

**Independent test**: `cargo test --test cli` on a two-column fixture.

- [X] T004 [US6] Tests in `crates/qd/tests/cli.rs`: inside a column `block move --before`, `dup`, `rm`, `add --after` (US1 1–4); across lists: column to column, column to top level, top level to column, `--into` an empty column, a target inside the moving block refused, another slide refused (US2 1–4, 6); a column's last element removed leaves `::: {.column}` / `:::`; `block place` on a nested ref makes a box after the slide's blocks and removes it from the column (US4 1); `block dup` of an element with a build puts it last in `build list` (FR-011); each result's `target` is the new ref.
- [X] T005 [US6] `crates/qd/src/ops.rs`: `block_move` (same list: `rearrange`; other list: cut + insert; `--into`), `block_add` (`--into`, nested `--before/--after`), `block_rm` and `block_dup` on any list; `top_element` callers that become free of it; `block_place` for a nested element (R6); `block_dup` with a build reorders (R7). `crates/qd/src/cli.rs`: `--into` on `block move` and `block add`; help text.
- [X] T006 [P] [US6] `skills/quartz-deck/SKILL.md`: arranging inside columns, `--into`.
- [X] T007 [US6] fmt, clippy, `cargo test`; `/pr-audit`; open PR 1.

## Phase 4: User Stories 1–4 — the editor (P1–P3, PR 2)

**Independent test**: browser tests in `crates/qd/tests/onslide.rs`.

- [X] T008 [US1] Browser test: ⌥↑ on the second element of a column swaps them; Duplicate and Delete in the menu; Insert after adds into the column; Move up disabled on a column's first element.
- [X] T009 [US2] Browser test: drag the picture to the other column (drop line shown there), then below the columns block; drop on an empty column fills it.
- [X] T010 [US3] Browser test: an emptied column shows "+ Add content" (not when presenting); choosing Paragraph adds it in that column.
- [X] T011 [US1] `crates/qd/src/ui/actions.js`: remove the `SLIDE_LEVEL` gate; Move up/down name `up(ref)/i±1`; Insert before/after and paste pass the nested ref.
- [X] T012 [US2] `crates/qd/src/ui/editor.js`: hover bar and drag for nested elements; drop targets any `[data-qd]` of the slide, an empty column as `--into`; keep the moved element selected by the op's `target`.
- [X] T013 [US3] `crates/qd/src/ui/editor.js`: "+ Add content" in an empty `.column[data-qd]` (editor only), opening the insert menu with `{ into: ref }`; `api.insert` passes `into`.
- [X] T014 [US4] Place freely enabled for nested elements (its `why` for divs stays).
- [X] T015 [P] Showcase `examples/showcase/showcase.qmd`: a slide to rearrange (text and a picture in two columns); README line.
- [X] T016 fmt, clippy, `cargo test`; `/pr-audit`; open PR 2.

## Phase 5: User Story 5 — column widths and number (P4, PR 3)

- [ ] T017 [US5] Tests in `crates/qd/tests/cli.rs`: `qd block attr r/0/0 width=40%` changes only that attribute; `qd column add r/0` appends an empty column; `qd block rm r/0/1` removes a column with its contents.
- [ ] T018 [US5] `qd block attr` (`with_attrs` setting/unsetting keys) and `qd column add` in `crates/qd/src/ops.rs` and `cli.rs`.
- [ ] T019 [US5] Format view: Width for a selected column; Add column for a selected columns block. Browser test.
- [ ] T020 fmt, clippy, `cargo test`; `/pr-audit`; open PR 3.

## Phase 6: Polish

- [ ] T021 Quickstart run; rebuild release, restart the showcase (port 4880), step through the new slide in the browser pane; `cargo clean` keeping `target/release/qd` after the last merge.

---

## Dependencies & Execution Order

- #137 (fresh ids in copies) → T001 → T002–T003 → US6 (PR 1) → US1–US4 editor (PR 2) → US5 (PR 3) → Polish.
- The editor stories need PR 1's ops; US5 needs nothing from PR 2 but is built last as P4.

## Parallel examples

- T006 (skill) alongside T005.
- T015 (showcase) alongside T011–T014.

## Implementation strategy

- **PR 1**: every arrangement from the CLI.
- **PR 2**: the same on the slide, plus the showcase. Spec 025's core is done here.
- **PR 3**: widths and column count, if the person wants them now.
