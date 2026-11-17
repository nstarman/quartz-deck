---
description: "Tasks for 016 arrows on the slide"
---

# Tasks: Arrows on the slide

**Input**: `specs/016-slide-arrows/` (spec.md, plan.md, research.md,
data-model.md, contracts/, quickstart.md)

**Tests**: required (constitution VI, test-first). Each story's tests are
written first and seen failing. Tests that need Quarto, a browser or the
network skip without them; tests that need quarto-arrows share one
download.

**Organization**: by user story, in the spec's priority order. Each phase is
one PR:
1. Foundational: the arrow reader, `qd arrow show/list`, the download and
   install (PR 1);
2. US1, P1: draw and move (PR 2);
3. US2, P2: curve and style (PR 3);
4. US3 + US4, P2/P3: labels and appearing on click (PR 4);
5. Polish.

Paths are relative to the repo root. **Privacy:** fixtures, examples and PR
text use made-up slides only, never the person's own talks, paths or names.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on unfinished tasks)
- **[Story]**: US1–US4

---

## Phase 1: Setup

- [X] T001 Add the empty `crates/qd/src/arrows.rs` (declared in `crates/qd/src/lib.rs`), `crates/qd/tests/arrows.rs` (with `mod common;`), and an `Arrow` subcommand group in `crates/qd/src/cli.rs` with no subcommands yet.

---

## Phase 2: Foundational: reading arrows, getting the extension (PR 1, blocks every story)

- [X] T002 [P] Write failing unit tests in `crates/qd/src/arrows.rs`:
  - a paragraph that is exactly one `{{< arrow … >}}` reads as an arrow: `from`, `to`, `control1/2`, and every option with its value and span; values quoted with `"`, `'` or bare; whitespace and option order as written;
  - read-only reasons: no `position="absolute"` ("in the text's flow"), unbalanced quotes, `>}}` inside a value, two shortcodes or text beside one, a bad `from`/`to`;
  - a paragraph without an arrow shortcode isn't an arrow at all.
- [X] T003 Implement the reader in `crates/qd/src/arrows.rs` (research R4; data-model "Arrow"). T002 passes.
- [X] T004 [P] Write failing tests:
  - `crates/qd/src/assets.rs` unit: `QUARTO_ARROWS` lists the six files of research R3 with their SHA-256s and URLs at commit `957a61bc1ef13ec913baefce0f7eee61226c140d`; a multi-file ensure with downloads off and nothing cached fails with `E_DOWNLOAD` and leaves no partial folder;
  - `crates/qd/src/plugins.rs` unit: an extension installed from files on disk into `_extensions/emilhvitfeldt/arrows/` (version-aware: absent and older are written, an edited same-version copy is kept with a warning);
  - `crates/qd/tests/licenses.rs`: the downloaded assets include quarto-arrows, "MIT" (from its README).
- [X] T005 Implement the pinned multi-file download in `crates/qd/src/assets.rs` (reusing `download_verified`, cached under `<cache>/lib/quarto-arrows-0.3.0/`) and installing an extension from disk in `crates/qd/src/plugins.rs`. T004 passes.

  **As built:** the six files are six `assets::Pinned` entries (`QUARTO_ARROWS`), so `ensure` and `download_verified` are reused as they are; `plugins::from_dir` reads files on disk into an `Extension` for the existing `install_these`. The loader that downloads and installs into a deck (`plugins::quarto_arrows`) comes with its first caller, `qd arrow add` (T010).
- [X] T006 [P] Write failing CLI tests in `crates/qd/tests/arrows.rs`:
  - **`show_reads_an_arrow`:** `qd arrow show a/1` matches contracts/cli.md (from, to, control, options, `editable: true`);
  - **`show_explains_why_not`:** an inline arrow and an unreadable one give `editable: {reason, at}`;
  - **`show_refuses_non_arrows`:** a plain paragraph gives `E_NOT_FOUND`;
  - **`list_arrows`:** `qd arrow list a` lists the slide's arrows in order.
- [X] T007 Implement `qd arrow show` and `qd arrow list` in `crates/qd/src/cli.rs` and `crates/qd/src/arrows.rs`, in `qd schema`'s output docs and `API_OPS`. T006 passes.

**Checkpoint**: PR 1. qd reads arrows exactly and can fetch the extension; nothing on the slide yet.

---

## Phase 3: User Story 1 — Draw an arrow and move it (P1, PR 2) 🎯 MVP

**Goal**: Insert ▸ Arrow and a drag draw an arrow; select, drag ends and line with snap guides, nudge, Delete, Escape; `qd arrow add/move/rm`.

**Independent Test**: spec US1; quickstart §1 (add, move, undo) and §2.

- [X] T008 [P] [US1] Write failing unit tests in `crates/qd/src/arrows.rs` for the patches: set an option's value in place (each quoting kept); add an option before `>}}`; remove an option and its spacing; a new arrow's line (options in a fixed order, defaults left out); its place in the slide (after the last content block, before notes); CRLF kept; every other byte unchanged.
- [X] T009 [P] [US1] Write failing CLI tests in `crates/qd/tests/arrows.rs`:
  - **`add_installs_and_adds`:** the first `add` installs `_extensions/emilhvitfeldt/arrows/` from the shared download and adds one line before the notes; `--dry-run` writes nothing; undo removes the line;
  - **`add_offline_changes_nothing`:** with `QD_NO_DOWNLOAD=1` and an empty cache, `E_DOWNLOAD`, deck and folder unchanged;
  - **`move_from_to_by`:** `--from`, `--to` and `--by` each change only those options (`--by` also moves control points); a stale `--expect` gives `E_CONFLICT`;
  - **`move_refuses_read_only`:** an inline arrow gives `E_UNSAFE_EDIT`;
  - **`rm_removes`:** the line goes, undo brings it back;
  - **`rendered_where_given`:** `quarto render` of a deck with an arrow puts its line's ends within 1 px of `from`/`to` in slide pixels (skip without quarto or the network).
- [X] T010 [US1] Implement `qd arrow add`, `move` and `rm` in `crates/qd/src/arrows.rs` and `crates/qd/src/cli.rs`, with the install on the first `add`. T008 and T009 pass.
- [X] T011 [P] [US1] Write failing browser tests in `crates/qd/tests/onslide.rs`:
  - **`arrow_draw`:** Insert ▸ Arrow and a drag add an arrow with the dragged ends; an end near a box's edge snaps exactly, with a guide; ⌘ held places freely;
  - **`arrow_select_and_drag`:** a click on the line selects it (handles shown); dragging an end changes only that end; dragging the line moves both; Escape mid-drag changes nothing;
  - **`arrow_keys_and_delete`:** arrow keys nudge 1 px (⇧ 10 px); Delete removes; undo brings it back;
  - **`arrow_click_beside_line`:** a click beside the line selects the element underneath, not the arrow.
- [X] T012 [US1] Implement the overlay in `crates/qd/src/ui/arrows.js` (hit path, handles, drawing mode, drags with provisional drawing, snapping via `ui/guides.js` and the editor's guide lines, keys), Insert ▸ Arrow in `crates/qd/src/ui/actions.json`, and its wiring in `crates/qd/src/ui/editor.js` and `UI_MODULES`. T011 passes.

**Checkpoint**: PR 2. Arrows are drawn and placed on the slide.

---

## Phase 4: User Story 2 — Curve and style (P2, PR 3)

- [X] T013 [P] [US2] Write failing tests: unit tests for the bend formula (research R6) and `set` (a default value removes the option); CLI tests **`bend_through_and_straight`** and **`set_style`**; browser tests **`arrow_bend_handle`** (the curve passes through the drop point; dropping near the line straightens it) and **`arrow_properties_style`** (colour, width, dash, head, heads).
- [X] T014 [US2] Implement `qd arrow bend` and `qd arrow set`, the middle handle, and the Properties fields for an arrow. T013 passes.

---

## Phase 5: User Stories 3 and 4 — Labels and appearing on click (P2/P3, PR 4)

- [ ] T015 [P] [US3] [US4] Write failing tests: CLI tests for `label`, `label-position` and `fragment`; browser tests **`arrow_label_by_double_click`** (the sheet sets and clears the label) and **`arrow_appears_on_click`** (the element bar's toggle writes `fragment="true"`, and presenting hides it until the click).
- [ ] T016 [US3] [US4] Implement the label sheet on double-click, the label-position field, and the element bar's "Appears on click" for arrows. T015 passes.

---

## Phase 6: Polish

- [ ] T017 [P] Docs: `skills/quartz-deck/SKILL.md` (`qd arrow` commands), `README.md` (arrows), and an example deck for the render test.
- [ ] T018 Run quickstart §1–§5 and record it in `specs/016-slide-arrows/checklists/quickstart-run.md`.
- [ ] T019 With the person: redraw the arrows of one of their real slides (SC-006) and record what needed changing. Nothing about their slide goes into the repository.
- [ ] T020 Final constitution re-check in `plan.md`; fmt, clippy (`--workspace --all-targets -D warnings`) and `cargo test --workspace --no-fail-fast` green.
- [ ] T021 Concision audit (`/pr-audit`), the last task before each PR is marked ready.

---

## Dependencies & Execution Order

- **Setup (T001) → Foundational (T002–T007)** → every story.
- **US1 (T008–T012)** after Foundational.
- **US2 (T013–T014)** and **US3/US4 (T015–T016)** after US1: they need `set` patches (T010) and the overlay (T012); they're independent of each other (serialize edits to `arrows.js`).
- **Polish** at the end; T021 before every PR; T019 needs the person.

## Parallel opportunities

- T002 ∥ T004 ∥ T006 (tests in different files); T008 ∥ T009 ∥ T011.

## Implementation strategy

- **MVP:** PR 1 and PR 2 (US1): arrows drawn and placed on the slide and
  from the CLI.
- **Then:** PR 3 (curve and style), PR 4 (labels, appearing on click).

Each PR passes `/pr-audit` (T021) and is merged only after the person's
"merge when green".
