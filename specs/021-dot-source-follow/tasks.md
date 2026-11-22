---
description: "Tasks for 021 diagram source follows the cursor"
---

# Tasks: Diagram source follows the cursor

**Input**: `specs/021-dot-source-follow/` (spec.md, plan.md, research.md,
data-model.md, contracts/cli.md, contracts/editor.md, quickstart.md)

**Tests**: required (constitution VI, test-first). Each story's tests are
written first and seen failing.
- **Slowed Quarto:** the preview tests run Quarto through `slow_quarto`
  (spec 020's helper in `tests/onslide.rs`), so "before the render" is
  visible.
- **Skips:** tests that need Quarto, Graphviz or a browser skip without them.

**Organization**: by user story, in the spec's priority order. Each phase is
one PR:
1. Foundational + US1, P1: the outline (PR 1);
2. US2, P2: the preview (PR 2);
3. US3, P3, and polish (PR 3).

Paths are relative to the repo root. **Privacy:** tests and docs use made-up
decks only; trying it on the person's talk (T020) leaves nothing about the
talk in the repository.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on unfinished tasks)
- **[Story]**: US1–US3

---

## Phase 1: Foundational, `qd dot at` (blocks every story)

- [X] T001 [P] Write failing unit tests for `at(text, offset)` in `crates/qd/src/dot.rs` (contracts/cli.md § Tests):
  - **What it finds:** each of these at the start, middle and end of its statement: a box, an arrow, each link of `a -> b -> c` and the chain's own `[...]`, a `subgraph cluster_x {` line, `layout=neato;`, `node [shape=box];`, a blank line, a comment and `digraph {`;
  - **Names:** a quoted name with spaces, an HTML label, a port (`a:e -> b`);
  - **Arrow refs:** a repeated arrow's ref is `a->b#2`, matching `dot show`'s `index`;
  - **Stops:** an unclosed quote gives a stop with line and column inside the body.
- [X] T002 Implement `pub fn at(text: &str, offset: usize) -> Result<Value>` in `crates/qd/src/dot.rs` on `parse_cell` (research R1, R2):
  - walk the statements (into subgraphs) for the innermost span holding `offset`;
  - name a box by its name, an arrow by `from->to#k` with the edge numbering `Graph::edges()` already uses, and the link under the cursor when the offset falls in one operand pair;
  - answer `{"ok": true, "at": {...}}`, `{"ok": true, "at": null}`, or `{"ok": true, "at": null, "stopped": {reason, at}}` (data-model "Statement at the cursor");
  - an offset past the end or inside a character gives `E_INVALID_ARG`.
  T001 passes.
- [X] T003 Add `qd dot at --text BODY --at OFFSET` in `crates/qd/src/cli.rs` (`DotCmd::At`): `--text -` reads stdin, the schema line goes in `qd schema`, and the op is read-only. Write a failing test first in `crates/qd/tests/dot.rs`:
  - the CLI answers a box at an offset;
  - `/api/op` with `{"op": "dot at", "args": {"text": …, "at": …}}` gives the same answer;
  - a bad offset is `E_INVALID_ARG`.

**Checkpoint**: an agent can ask what is at any point of a diagram's text.

---

## Phase 2: US1, P1: the slide shows which box a line is (PR 1)

**Goal**: the cursor's box or arrow is outlined on the slide, and only it.

**Independent test**: spec US1's independent test.

- [X] T004 [US1] Write failing browser tests in `crates/qd/tests/onslide.rs` (contracts/editor.md § Tests), on a made-up deck with one diagram of three boxes and a chain:
  - **`source_follow_outlines`:**
    - in the source pane, the caret on each box's line, then each link of the chain, then `layout=…`, then a line outside the cell;
    - after each, exactly the matching `g.node`/`g.edge` carries `qd-dot-cursor`, or none does;
    - an element selected beforehand (a text box) stays selected, with its bar shown;
  - **`source_follow_hover`:** the pointer resting over another box's line moves the outline there; a `mouseleave` gives it back to the caret's box.
- [X] T005 [US1] Implement `follow(cell, text, offset)` and `unfollow()` in `crates/qd/src/ui/dot.js` (research R4):
  - call `dot at` and keep the last good answer (data-model "Follow state");
  - put `qd-dot-cursor` on the matching element through `nodeNamed`/`edgeNamed` (several for a whole chain), or a cluster by its `<title>`, and take it off everywhere else;
  - leave `sel`, the bars and the panel alone;
  - re-apply after `mark()` (a preview reload);
  - do nothing while presenting.
- [X] T006 [US1] Report the cursor from `crates/qd/src/ui/pane.js` (research R5):
  - on caret moves and typing, debounced 60 ms, find the ```` ```{dot} ```` fence in `ta.value` that holds the caret;
  - send its body and the caret's offset in it, with the cell's ref from `onCaret`'s block lookup in `editor.js`;
  - otherwise call `unfollow()`;
  - Esc and blur clear;
  - hover: a `mousemove` resting 300 ms maps the pointer to a line (the highlight layer's line elements) and a column (character width), then takes the same path; `mouseleave` gives it back to the caret.
- [X] T007 [P] [US1] Style `qd-dot-cursor` in the CSS `dot.js` injects into the preview: a dashed accent outline for boxes and clusters, a wider dashed stroke for arrows, in light, dark and high contrast. T004 passes.

**Checkpoint**: PR 1. Moving through a diagram's source shows which element each line is.

---

## Phase 3: US2, P2: a preview of the box beside the line (PR 2)

**Goal**: a small drawing of the element alone, beside the line, following typing.

**Independent test**: spec US2's independent test.

- [X] T008 [P] [US2] Write failing unit tests for `alone` in `crates/qd/src/dot.rs` (contracts/cli.md):
  - a box's graph keeps its merged attributes and the `node` defaults in force at it, and drops `pos`, `layout`, `splines`, `size`;
  - an arrow's graph has its two ends' boxes and the arrow with its attributes;
  - defaults written after the box don't apply;
  - with Graphviz installed, `dot -Tsvg` draws it and the label is in the SVG.
- [X] T009 [US2] Add `alone` to `at()`'s answer for boxes and arrows in `crates/qd/src/dot.rs`, writing statements with `node_stmt` and the existing attribute merging (research R3). T008 passes.
- [X] T010 [P] [US2] Write failing browser tests in `crates/qd/tests/onslide.rs`, with Quarto slowed by 3 s:
  - **`source_follow_preview`:**
    - the caret on a box's line shows `#dot-peek` beside it, not over it, with the box's label;
    - typing a new `fillcolor` redraws it with that fill before the slide's render lands;
    - typing an unclosed quote leaves the last drawing with `.stale`;
    - Esc closes it;
  - **`source_follow_setting_off`:** with `diagrams.source-preview` false, no `#dot-peek`, and the outline still there.
- [X] T011 [US2] Implement the pop-over:
  - `#dot-peek` in `crates/qd/src/ui/index.html`, styled in `crates/qd/src/ui/editor.css` (at most 240 × 160, the panel's colours, `aria-hidden`);
  - in `dot.js`, lay out `alone` with `graphviz()`, add pictures with `pictures()`, and place it beside the line's rectangle (from `pane.js`), kept inside the window;
  - `.stale` on a stop; hidden for `setting`, `defaults`, `cluster` and nothing.
- [X] T012 [US2] Add `diagrams.source-preview` (default `true`) to `crates/qd/src/settings.rs` with its test, and read it in `dot.js` (research R7). T010 passes.

**Checkpoint**: PR 2.

---

## Phase 4: US3, P3: the same in the cell editor on the slide (PR 3)

**Goal**: editing a diagram's source over the slide follows the cursor too.

**Independent test**: spec US3's independent test.

- [ ] T013 [US3] Write a failing browser test **`source_follow_cell_editor`** in `crates/qd/tests/onslide.rs`: open a diagram cell's source on the slide, move the caret to a box's line, and check the outline and the pop-over.
- [ ] T014 [US3] Report the cursor from `crates/qd/src/ui/source.js` (research R6) when the edit is a `{dot}` cell: the body is the textarea's text inside its fences, and the pop-over sits beside the textarea's line. T013 passes.

**Checkpoint**: every place a diagram's source is typed follows the cursor.

---

## Phase 5: Polish (PR 3)

- [ ] T015 [P] Docs: `README.md` and `skills/quartz-deck/SKILL.md` describe `qd dot at`, the outline, the preview and its setting.
- [ ] T016 [P] Add the `qd dot at` line to `qd schema`'s test.
- [ ] T017 Check SC-003: in `crates/qd/tests/onslide.rs`, 200 keystrokes in a 20-box diagram's source each appear within one frame with following on, as with it off (or record the measurement in the quickstart run if a test is too flaky under load).
- [ ] T018 Run quickstart §1–§3 and record it in `specs/021-dot-source-follow/checklists/quickstart-run.md`, with the SC-002 timing.
- [ ] T019 Final constitution re-check in `plan.md`; fmt, clippy (`--workspace --all-targets -D warnings`) and `cargo test --workspace --no-fail-fast` green.
- [ ] T020 With the person: on a copy of their talk's diagram, check SC-005 (they can tell which line is which box, and adjust a box's look from the preview). Nothing about the talk goes into the repository.
- [ ] T021 Concision audit (`/pr-audit`), the last task before each PR is marked ready.

---

## Dependencies & Execution Order

- **Phase 1** (T001–T003) blocks everything: every story asks `dot at`.
- **US1** (T004–T007) needs Phase 1. T007 can go alongside T005–T006.
- **US2** (T008–T012) needs Phase 1 and US1's `follow()` (T005), which it extends. T008 and T010 can be written alongside each other.
- **US3** (T013–T014) needs US1 and US2.
- **Polish** last. T015 and T016 can be done in parallel.

### Within each story

Tests first, seen failing; then the Rust; then the editor; then the story's
tests pass.

## Parallel examples

- Phase 1: T001 (unit tests) while reading the CLI wiring for T003.
- US1: T007 (CSS) alongside T005–T006.
- US2: T008 (Rust tests) alongside T010 (browser tests).

## Implementation strategy

- **MVP**: Phase 1 + US1 (PR 1). The outline alone answers "which box is
  this line", and an agent gets `qd dot at`.
- **Then**: US2 (PR 2), then US3 and polish (PR 3).
- Each PR passes `/pr-audit` (T021), and is merged only on the person's say-so.
