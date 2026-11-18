# Implementation Plan: Slide groups

**Branch**: `018-slide-groups` | **Date**: 2026-11-17 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/018-slide-groups/spec.md`

## Summary

A group is a Quarto section: a heading at the deck's top section level
(`#` by default) and the slides after it, which Quarto already renders as
one reveal.js vertical stack (research R1).
- **qd's core** reads each slide's group from heading levels (`head`,
  `parent`) and reports it in the deck JSON. New ops `indent` and
  `outdent` patch only heading marks (R2, R3); `move` gains `--indent` and
  `--outdent` so a drop that changes depth is one undo step (R4);
  `navigation` sets the deck's `navigation-mode` (R6).
- **The editor's slide list** indents sub-slides, collapses heads (state in
  `localStorage`, never in the file; R5), takes Tab/⇧Tab and ← →, sets
  depth by dragging sideways (R8), and treats a collapsed head as its whole
  group for drag, copy, cut, duplicate and Delete through the list's
  existing `selected(id)`.

## Technical Context

**Language/Version**: Rust stable (edition 2024); plain ES modules for the editor

**Primary Dependencies**: none new

**Storage**: the `.qmd` (heading levels, `navigation-mode`); the browser's `localStorage` for collapse state

**Testing**:
- `cargo test`:
  - unit tests in `deck.rs`: `group_level` and `head`/`parent` for slide levels 1, 2 and 3, decks with and without `#`, `---` slides, implicit first slides;
  - unit tests in `ops.rs`: each R3 case; ATX, setext and `---` level patches; ids and attributes kept; every other byte identical; refusals with their reasons; `navigation` in each front-matter shape (none, top-level key, under `format: revealjs:`, `format: revealjs` as a string);
  - CLI tests (`tests/cli.rs`): `indent`, `outdent`, `move --indent/--outdent`, `navigation`: JSON, file, `--dry-run`, `--expect`, undo;
  - the round-trip corpus: parsing still byte-identical; an indent then outdent on every corpus deck's slides leaves the rest of the file identical (SC-003);
  - browser tests in `tests/onslide.rs` (headless): indentation, collapse kept across reload with the file unchanged, ← →, Tab/⇧Tab, drag a collapsed group, drag sideways, copy/delete a collapsed group, the reveal-on-show of FR-008;
  - one `quarto render` of a grouped deck: stacks match `head`/`in` (SC-004).

**Target Platform**: macOS first; nothing Linux-hostile

**Project Type**: CLI + local web editor (existing)

**Performance Goals**: the list redraws as it does today; collapse and expand are instant (no render)

**Constraints**: byte-exact patches; plain Quarto renders the groups; no sidecar state in the deck

**Scale/Scope**: decks up to about 150 slides and 20 groups

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status |
|---|---|
| I. Clean room | ✅ Quarto's and reveal.js's documented behaviour, checked by rendering made-up decks. Keynote is named as a familiar model of behaviour only; nothing from it is copied. |
| II. `.qmd` is the truth | ✅ Groups are heading levels, which the file already holds. Patches change only heading marks (R2). Collapse state is view state in the browser, never in the deck, as the editor already keeps other view state outside the deck (`ui.source_width`, `ui.form_panel` in the settings file). |
| III. CLI first | ✅ `qd indent`, `qd outdent`, `qd move --indent/--outdent`, `qd navigation`, all in `API_OPS`, `qd schema` and the skill. A group is moved or removed by naming its slides, which `qd slides` lists. Collapse is not an edit, so it has no command. |
| IV. Live preview | ✅ Each op re-renders through the existing loop; collapse doesn't touch the render. |
| V. Safe edits | ✅ Ordinary ops with `--expect`, atomic writes, undo; opaque slides refuse. |
| VI. Test first | ✅ Grouping and each patch are unit-tested first; each command has a CLI test; each gesture a browser test. |
| VII. Simplicity | ✅ No new crates, no new module: two fields in `deck.rs`, a few ops in `ops.rs`, the list in `list.js`. No group command beyond what `move`/`rm` already do. |

Gate: **pass**.

## Project Structure

### Documentation (this feature)

```text
specs/018-slide-groups/
├── plan.md, research.md, data-model.md, quickstart.md
├── contracts/
│   ├── cli.md         # qd indent / outdent / move flags / navigation
│   └── editor.md      # list rows, keys, menus, drag, messages
└── tasks.md           # /speckit-tasks
```

### Source Code (repository root)

```text
crates/qd/src/
├── deck.rs             # group_level; Slide.head, Slide.parent; navigation_mode read
├── ops.rs              # level patch; indent, outdent; mv's depth flag; navigation
├── cli.rs              # Indent, Outdent, Navigation subcommands; Move flags; slides_json fields; schema; API_OPS
└── ui/
    ├── list.js         # indent, disclosure, collapse in localStorage, keys, sideways drag, selected() expands groups
    ├── editor.css      # row indent, disclosure button, "+N"
    ├── actions.json    # Indent / Outdent / Presentation navigation entries and keys
    └── actions.js      # their run functions; group labels on a collapsed head
crates/qd/tests/
├── cli.rs              # the new commands
├── roundtrip.rs        # indent/outdent leave the rest identical
└── onslide.rs          # list gestures
skills/quartz-deck/SKILL.md   # groups, the new commands
```

**Structure Decision**: no new files in `src/`; the feature is a property of
slides the core already reads, and a behaviour of the list that already
exists.

**Depends on**: PR #92 (the title slide's menu, rendered names, no "+"
between slides) changes `list.js`. Build on `origin/main` after it merges,
rebasing this branch onto it.

## PRs (order of work)

1. **Core:** `group_level`, `head`/`in` in the deck JSON; the level patch;
   `qd indent`, `qd outdent`; unit, CLI and corpus tests.
2. **US1:** the list shows groups; collapse and expand with `localStorage`;
   ← →; FR-008; ↑/↓ over visible rows.
3. **US2:** Tab/⇧Tab, ⌘]/⌘[, menu items with reasons; `qd move
   --indent/--outdent` and the sideways drag; messages.
4. **US3:** a collapsed head as its group in `selected()`: drag (ghost
   "+N", landing rule), copy, cut, duplicate, Delete, and their labels.
5. **US4 + polish:** `qd navigation` and the Slide menu submenu; docs and
   skill; quickstart run; regrouping one of the person's real talks
   (SC-006), done on a copy and never committed; final re-check.

Each PR passes `/pr-audit`.

## Open points for the person's review

- **Tab in the list** (R7): Tab and ⇧Tab indent and outdent, so Tab no
  longer leaves the list; Escape does. ⌘] and ⌘[ are offered too.
- **Navigation mode's home** (R6): the Slide menu's "Presentation
  navigation" submenu, next to Transition.
- **Group commands** (R4): no `qd group …`; agents name the head and its
  sub-slides with the existing `qd move` and `qd rm`.

## Complexity Tracking

None: no principle is bent.
