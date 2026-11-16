---
description: "Tasks for 015 diagrams you edit on the slide"
---

# Tasks: Diagrams you edit on the slide

**Input**: `specs/015-graphviz-diagrams/` (spec.md, plan.md, research.md,
data-model.md, contracts/, quickstart.md)

**Tests**: required (constitution VI, test-first). Each story's tests are
written first and seen failing. Tests that need Quarto or a browser skip
without them (`common::have`, `qd browser`).

**Organization**: by user story, in the spec's priority order. Each phase is
one PR:
1. Foundational: the DOT reader and `qd dot show` (PR 1);
2. US1, P1: move and pin (PR 2);
3. US2, P1: build and change the diagram on the slide (PR 3);
4. US3, P2: pictures in boxes (PR 4);
5. US4, P3: live routing (PR 5);
6. Polish.

Paths are relative to the repo root. **Privacy:** fixtures, examples and
PR text use made-up diagrams only, never the person's own diagrams, paths
or names.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on unfinished tasks)
- **[Story]**: US1–US4

---

## Phase 1: Setup

- [X] T001 Add the empty `crates/qd/src/dot.rs` (declared in `crates/qd/src/lib.rs`) and `crates/qd/tests/dot.rs` (with `mod common;`), the `crates/qd/tests/fixtures/dot/` folder, and a `Dot` subcommand group in `crates/qd/src/cli.rs` with no subcommands yet.

---

## Phase 2: Foundational: the DOT reader and `qd dot show` (PR 1, blocks every story)

- [X] T002 [P] Add a DOT corpus in `crates/qd/tests/fixtures/dot/`. About 15 small files, written for the test and made up:
  - digraph and graph;
  - `strict`;
  - graph `ID = ID` and `graph|node|edge [..]` statements;
  - quoted IDs with escapes, HTML labels with nested `<>`;
  - chains `a -> b -> c`, ports `a:e`;
  - subgraphs and clusters;
  - all three comment styles;
  - CRLF line endings;
  - a pinned neato + ortho diagram.

  Plus three unreadable ones: an unclosed brace, an unknown keyword position, and two graphs in one cell.
- [X] T003 Write failing unit tests in `crates/qd/src/dot.rs`:
  - every corpus file parses, and printing it back gives identical bytes;
  - each node, edge and attribute has the right name, value and span;
  - chains expand to edges with their chain position;
  - subgraph contents are marked read-only;
  - the three unreadable files give `E_UNSAFE_EDIT` with the place they stopped;
  - `pinned` is true only when `layout` is neato or fdp and every node has `pos` ending in `!`;
  - repeated edges `A->B` get indexes 1, 2 in source order.
- [X] T004 Implement the tokenizer and parser in `crates/qd/src/dot.rs` (research R4; data-model "Diagram", "Box", "Arrow"): tokens with spans, comments as trivia, the statement grammar, the model. T003 passes.
- [X] T005 [P] Write failing CLI tests in `crates/qd/tests/dot.rs`:
  - **`show_lists_the_diagram`:** a deck with a pinned `{dot}` cell; `qd dot show d/0` matches `contracts/cli.md` (nodes with label, look, url, image, pos, size; edges with index; `pinned`; `editable: true`);
  - **`show_explains_why_not`:** an unreadable cell gives `editable: {reason, at}`;
  - **`show_refuses_non_dot`:** a `{python}` cell or a non-cell gives `E_NOT_FOUND`.
- [X] T006 Implement `qd dot show` in `crates/qd/src/cli.rs` and `crates/qd/src/dot.rs`: finding the cell by block ref, the body span between the fences, the JSON. Add it to `qd schema`'s output docs. T005 passes.

**Checkpoint**: PR 1. qd reads diagrams exactly; nothing writes yet.

---

## Phase 3: User Story 1 — Move a box, and its arrows follow (P1, PR 2) 🎯 MVP

**Goal**: drag a box on the slide, its `pos` changes, and the arrows re-route after the drop; "Pin layout" makes any diagram placeable.

**Independent Test**: spec US1; quickstart §1 (move), §2 step 1, §3.

- [X] T007 [P] [US1] Write failing unit tests in `crates/qd/src/dot.rs` for the patches move needs (research R7):
  - set an existing attribute's value in place (quoted and bare);
  - add an attribute to an existing `[...]` list, and add `[...]` to a statement without one;
  - add a node statement for a node that appears only in edges;
  - `pos` formatting (at most two decimals, `!`);
  - snapping to 0.25 in;
  - CRLF kept;
  - every byte outside the patch unchanged.
- [X] T008 [P] [US1] Write failing CLI tests in `crates/qd/tests/dot.rs`:
  - **`move_at_and_by`:** `--at` and `--by` give the same file, one line changed; `--no-snap` keeps 3.13; `--dry-run` writes nothing; a stale `--expect` gives `E_CONFLICT`; `qd undo` restores the deck.
  - **`move_unpinned_refused`:** `E_INVALID_ARG` with a hint to `qd dot pin`.
  - **`pin_with_positions`:** `qd dot pin d/0 --positions {...}` writes `layout=neato`, `splines=ortho` and every `pos`; then `quarto render` puts the nodes where pinned, with axis-aligned edge paths (skip without quarto).
  - **`pin_without_positions`:** with no preview render and no `dot` on PATH, it fails with `E_NOT_FOUND` and the hint.
- [X] T009 [US1] Implement the attribute patches and `qd dot move` (`--at`, `--by`, `--no-snap`) and `qd dot pin`, with positions in contract order, in `crates/qd/src/dot.rs` and `crates/qd/src/cli.rs`. Positions come from:
  - `--positions`;
  - else the last preview render's SVG in the store's cache: node centres in points / 72, y negated, offset to y ≥ 0 (research R2);
  - else `dot -Tjson`.

  T007 and T008 pass.
- [X] T010 [P] [US1] Write failing browser tests in `crates/qd/tests/onslide.rs`:
  - **`diagram_drag_moves_a_box`:** in a deck with a pinned three-box diagram, clicking a node selects it (a selection outline); dragging it 1 in right shows it moving with straight provisional lines; dropping changes the file in that node's `pos` only (+1.0 x) and the next render shows the node there; Escape mid-drag changes nothing.
  - **`diagram_pin_prompt`:** dragging a node of an unpinned diagram offers "Pin layout"; accepting pins it and the slide looks the same (node centres within 2 px before and after).
  - **`diagram_arrow_keys_nudge`:** an arrow key moves a selected node by 0.25 in.
- [X] T011 [US1] Implement `crates/qd/src/ui/dot.js` (contracts/editor.md), loaded by the editor like the other modules:
  - find editable diagrams (`div[data-qd-lang=dot]` plus `qd dot show` through the op route);
  - hit-test nodes and edges by `<title>`;
  - selection outline, drag with a CSS transform and straight provisional lines;
  - the screen delta → graph units via `getScreenCTM` → inches → `move --by`;
  - Escape, arrow-key nudge;
  - the "Pin layout" prompt with positions read from the SVG.

  T010 passes.
- [X] T012 [US1] Add `dot-pin` to `crates/qd/src/ui/actions.json` (element menu, where `diagram`) and the graph's "Pin layout" button in the Properties panel (`crates/qd/src/ui/sheet.js`). Built as Arrange ▸ Pin layout on the element menu (`where: element`, shown for an unpinned diagram), since the Properties panel edits the source form and has no per-element buttons; the drag prompt is the other way in.

**Checkpoint**: PR 2. Diagrams can be placed by hand, in the editor and from the CLI.

---

## Phase 4: User Story 2 — Build and change the diagram on the slide (P1, PR 3)

**Goal**: connect, add, rename, delete, flip and arrowheads, looks, link and size; Insert ▸ Diagram and Box; all on the slide and as commands.

**Independent Test**: spec US2; quickstart §1, §2 steps 2–6.

- [X] T013 [P] [US2] Write failing unit tests in `crates/qd/src/dot.rs`:
  - add a statement before the closing `}` with the neighbours' indentation and terminator;
  - remove a whole-line statement, and one sharing a line;
  - remove an edge from a chain, which splits it keeping its attributes;
  - reverse an edge;
  - fresh names `n1`, `n2` skipping used ones;
  - `graph` uses `--`;
  - the looks found in a diagram, and the two starter looks (data-model "Look");
  - the starter diagram's text;
  - edits inside a subgraph give `E_UNSAFE_EDIT`.
- [X] T014 [P] [US2] Write failing CLI tests in `crates/qd/tests/dot.rs`, one per command in `contracts/cli.md`:
  - `add` (with `--at`, `--look`, `--look-of`, `--from`, `--to`);
  - `connect` (and `#2` for a repeat);
  - `rename`;
  - `set` (several attributes; `--look-of`);
  - `rm` of a node, which takes its edges;
  - `rm` of an edge from a chain;
  - `flip` (`E_INVALID_ARG` on `graph`);
  - `new --on` / `--after`.

  Each: JSON as in the contract, only the touched lines changed, undo restores. One test renders the result with plain Quarto (skip without quarto).
- [X] T015 [US2] Implement `add`, `connect`, `rename`, `set` (with `--look`, `--look-of`), `rm`, `flip` and `new` in `crates/qd/src/dot.rs` and `crates/qd/src/cli.rs`. T013 and T014 pass.
- [X] T016 [P] [US2] Write failing browser tests in `crates/qd/tests/onslide.rs`:
  - **`diagram_connect_and_add`:** dragging from a node's right handle onto another node adds `a -> b`; onto empty space adds a connected node at the drop point (snapped) in the first node's look.
  - **`diagram_rename_in_place`:** double-click, type, Enter changes only the label; Escape cancels.
  - **`diagram_delete_and_edge_properties`:** Delete on a node removes it and its edges; on a selected edge, removes it; the Properties panel flips an edge and turns its arrowhead off.
  - **`diagram_properties_look_link_size`:** picking the "artifact" look, a link and a width in Properties writes those attributes; the rendered node links there.
  - **`diagram_insert_menus`:** Insert ▸ Diagram on an empty slide adds the starter diagram; with it selected, Insert ▸ Box adds a node.
  - **`diagram_keyboard_and_names`:** handles have accessible names ("Connect from …").
- [X] T017 [US2] Implement in `crates/qd/src/ui/dot.js`:
  - connect handles and the follow line;
  - drop on a node or on empty space;
  - in-place rename;
  - Delete and Backspace;
  - edge selection.

  T016 passes, except the Properties and Insert tests.
- [X] T018 [US2] Add the Properties fields in `crates/qd/src/ui/sheet.js`:
  - for a node: label, look picker (the diagram's looks plus the starters), link, image, width/height;
  - for an edge: direction, arrowhead.

  Add the actions `dot-new`, `dot-add`, `dot-rm`, `dot-flip` in `crates/qd/src/ui/actions.json` (Insert menu and element menu, `where: diagram`). The rest of T016 passes.

  As built: the fields are in the Properties panel (`dot.panel` in `dot.js`, shown above the source form), not a sheet; rename on double-click uses the editor's sheet. Insert ▸ Diagram is `insert-diagram`, beside the other Insert items, and Insert ▸ Box in diagram is `dot-add`. Turning an arrowhead back on removes `arrowhead=none` rather than writing `arrowhead=normal`. The image field (Picture) came with US3.

**Checkpoint**: PR 3. Diagrams are built and changed on the slide, Keynote-style.

---

## Phase 5: User Story 3 — Pictures in boxes (P2, PR 4)

**Goal**: `image=` shows inside the box at its left, clear of the label, offline too.

**Independent Test**: spec US3; quickstart §4.

- [X] T019 [P] [US3] Write failing unit and CLI tests:
  - **`crates/qd/src/dot.rs` units:** `set image=PATH` writes the spacer label (data-model "Spacer label") around the existing label text; `rename` then edits only the text cell; `set image=` clears it back to a plain label.
  - **`crates/qd/tests/dot.rs`, `image_installs_dot_image`:** the first `set image=` installs `_extensions/dot-image` and names it in the deck's `filters:`.
- [X] T020 [US3] Implement the spacer-label handling in `crates/qd/src/dot.rs`, and the install trigger in `crates/qd/src/cli.rs` via `crates/qd/src/plugins.rs` (an `Extension` entry for `dot-image`, installed only when a diagram uses `image=`). T019 passes.
- [X] T021 [P] [US3] Add `plugins/dot-image/example.qmd` and its made-up image (a small generated PNG committed as a fixture): a pinned diagram with one box using `image=` and a spacer label, and one without. Write a failing test in `crates/qd/tests/plugins.rs`, **`dot_image_example`**. Rendering with plain Quarto, it checks that:
  - the `<image class="dot-image">` sits inside the right node's `<g>`, left of the label's `<text>`, with no overlap of their boxes;
  - a missing file gives no image and a warning naming it on stderr;
  - an `{{< include >}}`d cell gives the one-time warning.
- [X] T022 [US3] Write `plugins/dot-image/_extensions/dot-image/` (`_extension.yml` with a `post-quarto` filter, `dot-image.lua`) (research R5):
  - read the source via `quarto.doc.input_file`;
  - scan `{dot}` cells for node `image=`;
  - pair cells with rendered SVGs in order;
  - insert the `<image>`.

  Add `plugins/dot-image/README.md`. T021 passes.
- [X] T023 [US3] An offline check and a one-file export of the example, in `crates/qd/tests/plugins.rs`: the image is embedded, and no network is used beyond reveal.js's MathJax.

  As built: the picture takes the spacer's size from the label's `WIDTH`, and sits just left of the label's text. A one-file export embeds it as a `data:` URI with no extra work.

**Checkpoint**: PR 4. Boxes can carry pictures.

---

## Phase 6: User Story 4 — Arrows that re-route while dragging (P3, PR 5)

**Goal**: the `diagrams.live-layout` setting routes arrows live, using Quarto's own Graphviz, else a pinned download.

**Independent Test**: spec US4; quickstart §5.

- [X] T024 [US4] Spike (research R6): does Quarto's `share/js/graphviz-wasm.js` load as a browser ES module served by qd, and lay out a cell to SVG? Record the result and the entry point in `specs/015-graphviz-diagrams/research.md` R6. If it doesn't load, record the pinned `@hpcc-js/wasm-graphviz` version and SHA-256 as the main path.
- [X] T025 [P] [US4] Write failing tests:
  - **`crates/qd/src/settings.rs` unit:** `diagrams.live-layout` defaults to `false`.
  - **`crates/qd/tests/licenses.rs`:** the downloaded-asset list includes the engine with EPL-1.0 (Graphviz) and its wrapper's licence, both allowed.
  - **`crates/qd/tests/onslide.rs`, `diagram_live_routing`:** with the setting on, dragging a node shows routed (axis-aligned) edge paths that change during the drag; on drop the saved text equals the result with the setting off; with the engine made unavailable, dragging shows straight lines and no error.
- [X] T026 [US4] Implement the setting in `crates/qd/src/settings.rs` and Settings ▸ Diagrams in `crates/qd/src/ui/settings.js`. In `crates/qd/src/server.rs`, serve the engine from the Quarto install that `render::quarto()` finds, or from the pinned fallback in `crates/qd/src/assets.rs`. Add the per-frame layout and the fallback in `crates/qd/src/ui/dot.js`. T025 passes.

  **As built:** Quarto's build doesn't load in a browser (R6), so the server's `/lib/graphviz.js` always serves the pinned `assets::GRAPHVIZ`, fetched into qd's cache on first use (`QD_NO_DOWNLOAD` respected; not found otherwise). `Pinned` gained a `license` field, which the licence test checks. `qd dot show` returns the cell's `text`; while dragging, the editor appends the box's provisional `pos` to it, lays it out at most once a frame, and lines the new layout up with the live one on a box that stayed. The setting appears in the Settings window's list; that window has no sections, so there's no "Diagrams" heading.

**Checkpoint**: PR 5. Live routing on demand.

---

## Phase 7: Polish & cross-cutting

- [X] T027 [P] Docs:
  - `skills/quartz-deck/SKILL.md`: `qd dot` commands, editing diagrams, the setting, `dot-image`;
  - `README.md`: diagrams;
  - `plugins/README.md`: `dot-image`.
- [X] T028 Run quickstart §1–§5 and record it in `specs/015-graphviz-diagrams/checklists/quickstart-run.md`.
- [ ] T029 With the person: rebuild their pipeline diagram as a pinned `{dot}` cell, as the proving case (SC-006), and record what needed changing. Nothing about their diagram goes into the repository.
- [X] T030 Final constitution re-check in `plan.md`. fmt, clippy (`--workspace --all-targets -D warnings`) and `cargo test --workspace --no-fail-fast` green.
- [X] T031 Concision audit (`/pr-audit`), the last task before each PR is marked ready.

---

## Dependencies & Execution Order

- **Setup (T001) → Foundational (T002–T006)** → every story.
- **US1 (T007–T012)** after Foundational.
- **US2 (T013–T018)** after US1: it reuses US1's patches (T009) and overlay (T011).
- **US3 (T019–T023)** after US2: it needs `set` (T015) and the image field (T018).
- **US4 (T024–T026)** after US1: it needs the drag (T011). It's independent of US2 and US3.
- **Polish (T027–T031)** at the end. T031 runs before every PR. T029 needs the person.

## Parallel opportunities

- T002 ∥ T003 (corpus and unit tests). T005 ∥ T004.
- T007 ∥ T008, and T010 alongside T009.
- T013 ∥ T014 ∥ T016.
- After US1: US4 can be built alongside US2 (`dot.js` is shared, so serialize the edits to it).
- T019 ∥ T021.

## Implementation strategy

- **MVP:** PR 1 and PR 2 (US1). A diagram can be pinned and its boxes
  placed by hand, on the slide and from the CLI.
- **Then:** PR 3 (US2, the Keynote-style editing the person asked for),
  PR 4 (pictures), PR 5 (live routing).

Each PR passes `/pr-audit` (T031), and is merged only after the person's
"merge when green".
