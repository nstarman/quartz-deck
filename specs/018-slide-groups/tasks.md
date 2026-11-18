---
description: "Tasks for 018 slide groups"
---

# Tasks: Slide groups

**Input**: `specs/018-slide-groups/` (spec.md, plan.md, research.md,
data-model.md, contracts/, quickstart.md)

**Tests**: required (constitution VI, test-first). Each story's tests are
written first and seen failing. Tests that need Quarto or a browser skip
without them. Browser tests run headless (`tests/onslide.rs`); never a
visible Chrome window.

**Organization**: by user story, in the spec's priority order. Each phase is
one PR (plan.md "PRs"):
1. Foundational: reading groups, `head`/`in` in the deck JSON (PR 1);
2. US1, P1: the list shows and collapses groups (PR 2);
3. US2, P1: indent and outdent, by key, menu and drag (PR 3);
4. US3, P2: a collapsed group moves, copies and deletes as one (PR 4);
5. US4, P3 + Polish: navigation mode, docs, real-talk run, re-check (PR 5).

Paths are relative to the repo root. **Privacy:** fixtures, examples and PR
text use made-up slides only, never the person's own talks, paths or names.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on unfinished tasks)
- **[Story]**: US1–US4

---

## Phase 1: Setup

- [ ] T001 Once PR #92 (title slide menu in the list) has merged, rebase `018-slide-groups` onto `origin/main` and push with `--force-with-lease` (never merge main into the branch; if the force push is refused, stop and ask). `crates/qd/src/ui/list.js` must then have the title slide's context menu and no "+" between slides.

---

## Phase 2: Foundational: reading groups (PR 1, blocks every story)

- [ ] T002 [P] Write failing unit tests in `crates/qd/src/deck.rs`:
  - `group_level`: `slide-level: 2` with `#` → 1; `slide-level: 2` with no `#` → 1 (the default for new heads, `slide_level - 1`); `slide-level: 3` with `#` and `##` → 1; `slide-level: 3` with only `##` → 2; `slide-level: 1` → `None` (research R1, data-model "Deck");
  - `head`/`parent`: slides before the first head have no parent; a head has `head: true` and no parent; `##`, `---` and (at slide-level 3) `##` section slides after a head have it as parent; a head followed by a head is a group of one.
- [ ] T003 Implement `Deck::group_level` and `Slide.head`, `Slide.parent` in one pass in `crates/qd/src/deck.rs`, computed after segmentation. T002 passes; `crates/qd/tests/roundtrip.rs` still passes byte-identical.
- [ ] T004 [P] Write a failing CLI test in `crates/qd/tests/cli.rs`, **`slides_report_groups`**: `qd slides` on a made-up deck with two sections gives each slide `head` and, for sub-slides, `in` (the head's id), and the deck `group_level` (data-model "Deck JSON").
- [ ] T005 Add `head`, `in` and `group_level` to `slides_json` and its schema string in `crates/qd/src/cli.rs`. T004 passes.

**Checkpoint**: the editor receives the groups; nothing visible changes yet.

---

## Phase 3: User Story 1: see a deck's parts in the list (P1, PR 2) 🎯 MVP

**Goal**: sub-slides indented under their head; heads collapse and expand; collapse is remembered per deck in the browser and never written to the file.

**Independent test**: a deck with two `#` sections of three `##` slides shows two heads with three indented rows each; collapsing one leaves four visible rows; reload keeps it collapsed; the `.qmd` is byte-identical.

- [ ] T006 [P] [US1] Write failing headless browser tests in `crates/qd/tests/onslide.rs`:
  - **`list_indents_sub_slides`**: sub-slide rows are indented, heads and top-level slides aren't, the title slide stays first; numbers run 1, 2, 3 over all slides;
  - **`list_collapses_groups`**: the disclosure button (`aria-expanded`) hides the sub-slides and shows "+3"; ← on a focused open head collapses, → expands; ↑/↓ skip hidden rows;
  - **`collapse_survives_reload`**: after reload the head is still collapsed (`localStorage` key `qd.collapsed:<deck path>`), and the file's bytes are unchanged;
  - **`shown_slide_opens_its_group`**: `qd focus --set <sub-slide>` to a hidden sub-slide expands its head (FR-008).
- [ ] T007 [US1] In `crates/qd/src/ui/list.js`: indent rows with `in`; a disclosure button on heads with sub-slides (label "Collapse <name>"/"Expand <name>", `aria-expanded`); hide sub-slides of collapsed heads with `hidden` and show "+N" on the head; keep the collapsed set in `localStorage` under `qd.collapsed:<deck path>` (wrapped in try/catch; drop ids no longer heads); expand the head of the shown slide on render; ← → on rows; ↑/↓ over visible rows only (contracts/editor.md "The list", "Keys").
- [ ] T008 [P] [US1] Styles for the indent (16 px), the disclosure button and "+N" in `crates/qd/src/ui/editor.css`, in light and dark themes.
- [ ] T009 [US1] The narrow window's slide picker shows sub-slides indented in its menu (`pickable()` in `crates/qd/src/ui/list.js`). T006 passes.

**Checkpoint**: US1 works on any deck that already has sections.

---

## Phase 4: User Story 2: indent and outdent (P1, PR 3)

**Goal**: Tab/⇧Tab, ⌘]/⌘[, menu items and a sideways drag make and undo groups, changing only heading marks.

**Independent test**: on `##` slides A, B, C, D, `qd indent c` makes B `#` with C and D under it; `qd undo` restores the file byte for byte.

- [ ] T010 [P] [US2] Write failing unit tests in `crates/qd/src/ops.rs` for the level patch (research R2): ATX marks replaced with title, attributes and spacing kept; setext `=`↔`-` with the underline's length kept; setext to level 3 written as ATX; a `---` slide becoming a head gets a `#` line alone; an implicit slide refused ("give the slide a title first"); auto and explicit ids unchanged; every byte outside the changed marks identical.
- [ ] T011 [P] [US2] Write failing unit tests in `crates/qd/src/ops.rs` for `indent` and `outdent`, one per row of research R3's table, including: indenting a head whose group above exists (its sub-slides join that group); indenting a top-level slide under another top-level slide (both change, `changed` lists both); outdenting a sub-slide in the middle of a group (the later sub-slides follow it); refusals with their reasons for the first slide, a slide under only the title slide, a sub-slide on indent, a head on outdent, `slide-level: 1`, and opaque slides (`E_UNSAFE_EDIT`); slide-level 3 moves between `group_level` and `slide_level` only.
- [ ] T012 [US2] Implement the level patch and `ops::indent`, `ops::outdent` in `crates/qd/src/ops.rs`, each one patch returning the `changed` ids. T010 and T011 pass.
- [ ] T013 [P] [US2] Write failing CLI tests in `crates/qd/tests/cli.rs`: **`indent_and_outdent`** (JSON per contracts/cli.md, the file, `--dry-run`, `--expect`, one undo each, `E_INVALID_ARG` with the reason); **`move_lands_indented`** (`qd move d --after a --indent` changes position and level in one patch and one undo; `--outdent` likewise; the two flags conflict).
- [ ] T014 [US2] Add `--indent`/`--outdent` to `ops::mv` in `crates/qd/src/ops.rs` (research R4: R3 applied to the first moved slide in the same patch), and the `Indent`, `Outdent` subcommands and the `Move` flags in `crates/qd/src/cli.rs`, in `qd schema` and `API_OPS`. T013 passes.
- [ ] T015 [P] [US2] Write failing headless browser tests in `crates/qd/tests/onslide.rs`: **`tab_indents_in_list`** (Tab and ⌘] indent, ⇧Tab and ⌘[ outdent; when refused nothing changes and the reason shows; Escape moves focus to the canvas); **`menu_indent_items`** (Slide menu and row menu "Indent slide"/"Outdent slide", disabled with the reason as tooltip); **`drag_sideways_sets_depth`** (a drag 16 px or more to the right lands indented, to the left outdented, less keeps the depth; the drop marker shows the depth; one undo); **`regroup_message`** (indenting under a top-level slide shows "… is now a group head: it shows as a section slide.").
- [ ] T016 [US2] Add `slide-indent` and `slide-outdent` (keys Tab/⇧Tab in the list, ⌘]/⌘[) to `crates/qd/src/ui/actions.json` and their run functions, with disabled reasons computed from the deck JSON, in `crates/qd/src/ui/actions.js`.
- [ ] T017 [US2] In `crates/qd/src/ui/list.js`: Tab/⇧Tab in the list's keydown (research R7), Escape to the canvas; in `drag()`, the landing depth from horizontal offset (research R8, vertical list only), the drop marker at that depth, and `api.op('move', {..., indent|outdent: true})`; after an op, the message of contracts/editor.md "Messages" when `changed` names other slides or a `---` slide became a head. T015 passes.

**Checkpoint**: a flat deck can be grouped and ungrouped without touching the source.

---

## Phase 5: User Story 3: a group as one (P2, PR 4)

**Goal**: drag, copy, cut, duplicate and Delete on a collapsed head act on the whole group; an open head acts alone.

**Independent test**: collapse a group of four and drag it after the next group; one move, the rest of the file byte-identical, one undo puts it back.

- [ ] T018 [P] [US3] Write failing headless browser tests in `crates/qd/tests/onslide.rs`: **`drag_collapsed_group`** (ghost shows the head with "+N"; drop moves head and sub-slides in order; one undo); **`group_lands_between_groups`** (no gap opens between two sub-slides of another group; it lands before or after that group); **`copy_delete_collapsed_group`** (copy then paste puts the whole group back; Delete removes all as one undo; menu labels read "Copy group (4 slides)" and so on); **`open_head_moves_alone`** (moving or deleting an open head leaves its sub-slides, now in the group before, with the message "Its 3 slides now belong to …"); **`pick_across_groups`** (⇧-click picks across groups; a pick including a collapsed head includes its sub-slides).
- [ ] T019 [US3] In `crates/qd/src/ui/list.js`: `selected(id)` expands a collapsed head into its sub-slides (and does so for each collapsed head in a pick); the drag's items, gaps and midpoints use visible rows only, a group drag skips gaps inside other groups (FR-017), and its ghost shows "+N".
- [ ] T020 [US3] Group labels on a collapsed head ("Copy group (N slides)", "Cut group…", "Duplicate group", "Delete group…" with the confirm sheet naming the count) in `crates/qd/src/ui/actions.js`; duplicate of a group goes through `selected()` like copy. The open-head message after `move`/`rm` of a head alone. T018 passes.

**Checkpoint**: parts of a talk can be reordered in one drag.

---

## Phase 6: User Story 4: navigation mode (P3, PR 5)

**Goal**: one deck setting writes Quarto's `navigation-mode`.

**Independent test**: `qd navigation vertical` adds the option where research R6 says; `qd navigation linear` removes it.

- [ ] T021 [P] [US4] Write failing unit tests in `crates/qd/src/ops.rs` for `ops::navigation` (research R6): no front matter key and `format: revealjs` as a string → a top-level line; a `format: revealjs:` mapping → a line at the end of that mapping with its indentation; an existing key (top level or under `format: revealjs:`) edited where it is; `linear` removes it; an unknown mode is `E_INVALID_ARG`; every other front-matter byte identical. Also a unit test in `crates/qd/src/deck.rs` that `navigation_mode` reads `linear` by default, the `format.revealjs` value over a top-level one.
- [ ] T022 [P] [US4] Write a failing CLI test in `crates/qd/tests/cli.rs`, **`navigation_mode`**: JSON per contracts/cli.md, the file, undo; `qd slides` reports `navigation_mode`.
- [ ] T023 [US4] Implement `Deck::navigation_mode` in `crates/qd/src/deck.rs`, `ops::navigation` in `crates/qd/src/ops.rs` (top-level writes through the existing `replace_key`), the `Navigation` subcommand, `navigation_mode` in `slides_json`, `qd schema` and `API_OPS` in `crates/qd/src/cli.rs`. T021 and T022 pass.
- [ ] T024 [US4] Slide menu ▸ "Presentation navigation" ▸ Linear / Vertical / Grid, the deck's mode checked, running `qd navigation`, in `crates/qd/src/ui/actions.json` and `crates/qd/src/ui/actions.js` (next to Transition), with a headless browser test **`navigation_submenu`** in `crates/qd/tests/onslide.rs`.

---

## Phase 7: Polish & cross-cutting (PR 5)

- [ ] T025 [P] A render test in `crates/qd/tests/plugins.rs` (skips without Quarto): a made-up grouped deck renders vertical stacks matching `head`/`in`, in linear and vertical modes (SC-004).
- [ ] T026 [P] A corpus test in `crates/qd/tests/roundtrip.rs`: for every corpus deck, each possible indent followed by its outdent (and the reverse) leaves the file byte-identical to the start (SC-003).
- [ ] T027 [P] Document groups, `qd indent`, `qd outdent`, `qd move --indent/--outdent` and `qd navigation` (and that a group moves by naming its slides) in `skills/quartz-deck/SKILL.md` and `README.md`.
- [ ] T028 Run `specs/018-slide-groups/quickstart.md` in a scratch folder; record the run in `specs/018-slide-groups/checklists/quickstart-run.md`.
- [ ] T029 Regroup one of the person's real talks with several parts in the list (SC-006), on a copy outside the repo; nothing of it is committed or named in PR text.
- [ ] T030 Final constitution re-check written into `specs/018-slide-groups/plan.md`, mark `specs/ROADMAP.md`'s 018 line done, and `/pr-audit` before each PR is marked ready.

---

## Dependencies

- T001 before everything (the list changes of PR #92).
- Phase 2 (T002–T005) blocks every story.
- US1 (Phase 3) and US2's core (T010–T014) are independent; US2's editor part (T015–T017) needs US1's rows (T007).
- US3 needs US1 (collapse) and uses US2's drag depth (T017).
- US4 needs only Phase 2.
- Polish after the stories it checks.

Within each story: tests first, seen failing; then the code that makes them pass.

## Parallel examples

- Phase 2: T002 and T004 together (different files).
- US1: T006 with T008.
- US2: T010, T011, T013 and T015 together (tests in four places); then T012 → T014; T016 with T017's keydown part.
- US4 alongside US1–US3 once Phase 2 is in: T021 and T022 together, then T023 → T024.
- Polish: T025, T026 and T027 together.

## Implementation strategy

MVP is Phase 2 + US1: groups that already exist in a deck are shown and can
be folded, with no write to the file. US2 makes groups from the list, US3
makes them move as one, US4 adds the navigation setting. Each phase is one
PR on `origin/main`, passing `/pr-audit`, and is usable on its own.
