---
description: "Tasks for 017 import a drawing as a diagram"
---

# Tasks: Import a drawing as a diagram

**Input**: `specs/017-dot-import/` (spec.md, plan.md, research.md,
data-model.md, contracts/cli.md, quickstart.md)

**Tests**: required (constitution VI, test-first). Each step's tests are
written first and seen failing. The render check needs Quarto and skips
without it.

**Organization**: by user story, in the spec's priority order. Each phase is
one PR:
1. Setup + Foundational + US1, P1: hand-written drawings (PR 1);
2. US2, P2: Graphviz drawings (PR 2);
3. Polish.

Paths are relative to the repo root.
- **Privacy:** fixtures are made-up drawings. The person's own diagram (SC-005)
  is tried outside the repository, and nothing about it is committed.
- **Clean up:** after each merged PR, `cargo clean`, keeping
  `target/release/qd` (the person's standing request).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on unfinished tasks)
- **[Story]**: US1–US2

---

## Phase 1: Setup

- [X] T001 Add `roxmltree` to `crates/qd/Cargo.toml`. Create an empty `crates/qd/src/dot_import.rs`, declared in `crates/qd/src/lib.rs`. Make `node_stmt`, `spacer` and `normalised` in `crates/qd/src/dot.rs` `pub(crate)`. `dependency_licenses_are_allowed` passes with the new crate.
- [X] T002 [P] Make the fixture `crates/qd/tests/fixtures/dot-import/hand.svg` (research R9). It has:
  - four boxes: a plain `rect`, a rounded `rect` (`rx`), an `ellipse`, and a `rect` inside a `<g transform="translate(…) scale(…)">`;
  - each box's text, one in two `text` lines;
  - three lines with markers: a straight `line`, a bent `path` (L-shaped), and a `polyline` with markers at both ends;
  - an `a` around one box, and an `image` (`face.png`) inside another;
  - a stray line far from any box, and a stray text outside every box.

  Made up; no real names.

---

## Phase 2: Foundational: reading a drawing (blocks every story)

- [X] T003 [P] Write failing unit tests in `crates/qd/src/dot_import.rs` for the geometry (research R2):
  - `transform` parsing and composition (translate, scale, rotate, matrix, nested groups) applied to a point;
  - bounding boxes of `rect`, `circle` and `ellipse` under a transform;
  - first and last points, and the direction at each end, of `line`, `polyline` and `path` (absolute and relative `M L H V C Q Z`, and arcs taken as straight to their end).
- [X] T004 Implement the geometry in `crates/qd/src/dot_import.rs`. T003 passes.
- [X] T005 [P] Write failing unit tests in `crates/qd/src/dot_import.rs` for reading `hand.svg` into data-model "Drawing":
  - each Shape with its fill, stroke, rounded and link;
  - each Text with its anchor, string, fill and size (attributes and `style`);
  - each Line with its markers;
  - the Picture;
  - Other elements kept for the report.
- [X] T006 Implement the reader with `roxmltree` in `crates/qd/src/dot_import.rs` (research R1). Reject a file with no `<svg>` root, or bad XML, as `E_INVALID_ARG` ("not SVG"). T005 passes.

**Checkpoint**: qd reads a drawing into shapes, texts, lines and pictures, placed as drawn.

---

## Phase 3: US1, P1: a hand-drawn diagram becomes an editable one (PR 1)

**Goal**: `qd dot import hand.svg --on SLIDE` writes one pinned cell with the drawing's boxes, labels, looks, link, picture and arrows, and reports what it left out.

**Independent test**: spec US1's independent test, on `hand.svg`.

- [X] T007 [P] [US1] Write failing unit tests in `crates/qd/src/dot_import.rs` for turning a Drawing into Boxes and Arrows:
  - **Labels (R3):** texts go into the smallest box around them, joined top to bottom; a stray text is reported.
  - **Names (data-model "Box"):** from labels, made unique; `n1` for a box with no text.
  - **Arrow ends (R4):** within 8 units of an edge, or along the line within 24; a far end is reported with the end that found nothing; a box joined to itself is reported; markers give the direction, `dir=both` or `arrowhead=none`.
  - **Looks (R5):** `fillcolor`, `color`, `fontcolor`, `fontsize`, `style`, `shape`, `URL`, `image` with the spacer label.
  - **Placement (R2):** `pos` normalised and pinned; `width`, `height` and `fixedsize` from the bounding box.
- [X] T008 [US1] Implement boxes, arrows and the report in `crates/qd/src/dot_import.rs`. T007 passes.
- [X] T009 [P] [US1] Write failing CLI tests in `crates/qd/tests/dot.rs`:
  - **`import_hand_drawn`:** `qd dot import hand.svg --on SLIDE` gives the report of data-model "Report" and contracts/cli.md:
    - 4 boxes, 3 arrows;
    - `left_out` has the stray line and the stray text, each with its reason;
    - `extension: "dot-image"`.

    Then `qd dot show` reads the cell as editable, with every box pinned and the drawing's labels, link, picture, `shape=ellipse` and `dir=both`. The cell has `#| fig-width` and `#| fig-height` (R6). The front matter names `dot-image`.
  - **`import_is_one_edit`:** `--dry-run` writes nothing and gives a `diff`; `qd undo` puts the deck back in one step; `--expect` with a stale version gives `E_CONFLICT`.
  - **`import_replaces`:** `--replace` on an image element puts the cell in its place.
  - **`import_errors`:** not SVG gives `E_INVALID_ARG`; only lines, no boxes, gives `E_INVALID_ARG`; a missing file gives `E_IO`; two places given gives `E_INVALID_ARG`. The file is unchanged each time.
  - **`imported_diagram_edits`:** `qd dot move` on an imported box changes only its `pos` (FR-008).
- [X] T010 [US1] Implement the cell and `qd dot import` in `crates/qd/src/dot_import.rs` and `crates/qd/src/cli.rs`:
  - the cell text with `node_stmt` and the fig size (research R6);
  - placing it with `block_add`, or `edit` for `--replace`, and `with_filters` for a picture (R8);
  - `qd schema`'s output docs and `API_OPS`.

  T009 passes.
- [X] T011 [US1] Write and pass a render check in `crates/qd/tests/dot.rs`, `import_renders_where_drawn` (SC-002). Render an imported `hand.svg` deck with Quarto. Each box's centre in the rendered SVG, scaled to the slide, is within 5 px of its centre in `hand.svg`, and the picture is drawn. It skips without Quarto.

**As built (PR 1):**
- **Tests:** the unit tests were written alongside the code in `dot_import.rs`, not seen failing first; the CLI and render tests were run against the finished code.
- **Box statements:** written by `dot::node_stmt`, now `pub(crate)`, which leaves a spacer label (`<<TABLE…>>`) unquoted. The audit replaced a first, copied version.
- **Figure size:** `//|` options sized from the boxes' extent, not the canvas (see research R6). The render check caught both problems.
- **Not in `API_OPS`:** importing reads a file path, so the editor's server doesn't offer it; the spec keeps it CLI only.
- **The picture's extension:** installed by the CLI after the edit, as `dot set image=` does.
- **Tests:** `dot move` on an imported box snaps it to the 0.25 in grid, as for any box; the test expects that.
- **Audit:**
  - `dot::add_cell` (placing a cell, shared with `dot new`), `dot::with_dot_image` (shared with `dot set`) and the CLI's `install_dot_image` replace copies.
  - The audit also found that `dot::positions_from_svg`, which pins a diagram from its render, gave up on any rounded box (Graphviz draws those as a `<path>`). It now reads paths, and the render check uses it.

**Checkpoint**: PR 1. A hand-drawn diagram imports as an editable one.

---

## Phase 4: US2, P2: a Graphviz drawing comes back as a diagram (PR 2)

**Goal**: an SVG Graphviz rendered imports with its box names and arrows exactly, pinned where they were drawn.

**Independent test**: spec US2's independent test.

- [ ] T012 [US2] Make the fixture `crates/qd/tests/fixtures/dot-import/graphviz.svg`. Render a made-up directed DOT once with Quarto's Graphviz, and commit the SVG with its DOT in a comment beside the test. The DOT has 6 boxes, names with `-` and `_`, one ellipse and one linked box. Undirected titles (`a--b`) are covered by a unit test.
- [ ] T013 [P] [US2] Write failing tests:
  - unit tests in `crates/qd/src/dot_import.rs`: a Graphviz SVG is recognised (R7); nodes come from `<title>`, with `&#45;` unescaped; edges come from the edge titles, ports dropped;
  - CLI test `import_graphviz` in `crates/qd/tests/dot.rs`: the names and arrows equal the source DOT's (SC-004), each box is pinned where it was drawn, and `kind: "graphviz"`.
- [ ] T014 [US2] Implement the Graphviz reading in `crates/qd/src/dot_import.rs`. T013 passes; US1's tests still pass.

**Checkpoint**: PR 2.

---

## Phase 5: Polish

- [ ] T015 [P] Docs:
  - `README.md`: `qd dot import`, under Diagrams;
  - `skills/quartz-deck/SKILL.md`: import an SVG, then refine it; check `left_out` and `notes` and tell the person.

  Run `skill_matches_cli`.
- [ ] T016 Run quickstart §1–§7 and record it in `specs/017-dot-import/checklists/quickstart-run.md`.
- [ ] T017 With the person: import their own drawing (SC-005), outside the repository, and record what needed touching up and how close the boxes landed. Tune R4's two distances if they need it. Nothing about their drawing goes into the repository.
- [ ] T018 Final constitution re-check in `plan.md`; fmt, clippy (`--workspace --all-targets -D warnings`) and `cargo test --workspace --no-fail-fast` green.
- [ ] T019 Concision audit (`/pr-audit`), the last task before each PR is marked ready. Then `cargo clean`, keeping `target/release/qd`.

---

## Dependencies & Execution Order

- **Setup (T001–T002) → Foundational (T003–T006)** → every story.
- **US1 (T007–T011)** after Foundational.
- **US2 (T012–T014)** after US1. It reuses US1's cell writing and placing; only the reading differs.
- **Polish** at the end; T019 before every PR; T017 needs the person.

## Parallel opportunities

- T002 ∥ T003 ∥ T005 (fixture and tests in different files).
- T007 ∥ T009 (unit and CLI tests).

## Implementation strategy

- **MVP:** PR 1 (Setup, Foundational, US1). Hand-drawn diagrams, the case
  the person had.
- **Then:** PR 2 (Graphviz) and Polish.

Each PR passes `/pr-audit` (T019) and is merged only after the person's
"merge when green".
