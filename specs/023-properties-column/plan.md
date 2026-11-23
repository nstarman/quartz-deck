# Implementation Plan: One properties column

**Branch**: `023-properties-column` | **Date**: 2026-11-23 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/023-properties-column/spec.md`

## Summary

Every slide, element and deck setting moves into one column with three
views, Format, Animate and Deck. The slide bar over the slide and the
element bar beside a selection go. The text toolbar still floats beside
text being typed.

- **The column**: spec 019's inspector (`#panel`), always shown, with the
  views above its tabs (R1, R2).
- **Animate**: transitions and builds as Quarto and reveal.js write them:
  - slide attributes;
  - fragment classes;
  - `fragment-index` for the order.

  New ops are `qd build set`, `qd build list` and `qd build order` (R3).
- **Deck**: `qd meta set` takes dotted keys under `format.revealjs`, and
  `qd meta size` resizes the slide while keeping placed boxes in place (R4).
- **Placement**: spec 022's docking, made to take a second pane, with a
  `properties.side` setting (R5).

## Technical Context

**Language/Version**: Rust stable (edition 2024) for the ops; plain ES modules for the editor

**Primary Dependencies**: none new

**Storage**: the deck's `.qmd` (attributes, classes, front matter); settings for the column's place and size

**Testing**:
- `cargo test`: CLI contract tests (`tests/cli.rs`) and `ops.rs` unit tests for the new ops and dotted keys;
- `tests/onslide.rs` browser tests per contracts/editor.md, including the bar tests moved to the column, and a presenting-mode check of build order;
- tests needing Quarto or a browser skip without them.

**Target Platform**: macOS first; nothing Linux-hostile

**Project Type**: CLI + local web editor (existing)

**Performance Goals**: switching views or selection updates the column within one frame (no fetch beyond what the panel already does); a setting change writes as today

**Constraints**: deck text written by a control equals the op's (SC-003); no proprietary look or names (constitution I)

**Scale/Scope**: about 45 existing controls rehomed; 3 new ops; 2 changed ops; about 8 browser tests moved or added

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status |
|---|---|
| I. Clean room | ✅ Our own design, layout and wording. The effects and transitions are reveal.js's own, as Quarto documents them. |
| II. `.qmd` is the truth | ✅ Every setting is in the deck as Quarto reads it: attributes, classes, `fragment-index` and front matter. The editor keeps only its own place and size. |
| III. CLI first | ✅ Each control is a CLI op. Builds and deck size get new ops (`qd build …`, `qd meta size`), and `qd meta set` gains dotted keys, so agents can do everything the column does. |
| IV. Live preview | ✅ Changes render through Quarto as today. Presenting shows the transitions and build order. |
| V. Safe edits | ✅ The existing handlers, with `--expect` and one undo step per op. A build-order change is one patch. |
| VI. Test first | ✅ The contracts' tests come first in each story. The existing bar tests move to the column with the same checks on the deck's text. |
| VII. Simplicity | ✅ No new dependency. The inspector, `dock.js` and `toolbar.js` are reused, and the slide bar, element bar and wide-window rule are removed. The new ops are thin over existing class, attribute and meta edits. |
| Licences | ✅ Nothing new. |

Gate: **pass**.

**Final re-check (T023, PR 4)**: still passes.
- II: the column's place is the person's setting (`properties.side`), not the deck's.
- III: everything the Deck and Animate views set has an op (`qd build set/list/order`, `qd meta set format.revealjs.KEY`, `qd meta size`).
- V: a size change, with its boxes and arrows scaled, is one patch and one undo step.
- VII: the docking reuses `dock.js` for both panes. No new dependency.

## Project Structure

### Documentation (this feature)

```text
specs/023-properties-column/
├── spec.md, plan.md, research.md, data-model.md, quickstart.md
├── contracts/
│   ├── cli.md         # qd build set/list/order, dotted meta set, meta size
│   └── editor.md      # the column, its views, docking, tests
└── tasks.md           # /speckit-tasks
```

### Source Code (repository root)

```text
crates/qd/src/
├── ops.rs              # build_set, build_order, build_list; dotted meta set/unset; meta size with places
├── cli.rs              # BuildCmd, MetaCmd::Size, schema lines
├── settings.rs         # properties.side, ui.properties_width/height
└── ui/
    ├── inspector.js    # always shown; views (Format/Animate/Deck) above tabs
    ├── props/
    │   ├── format.js   # slide, title slide, element Look/Text/Arrange, footer actions
    │   ├── animate.js  # transition, speed, morph; appear/leave; build list
    │   └── deck.js     # theme, size, numbers, default transition, navigation
    ├── dock.js         # takes a pane and its setting (source pane, properties column)
    ├── toolbar.js      # the text buttons, renderable into the Text tab too
    ├── editor.js       # drops #slidebar/#elbar; selection → column
    ├── editor.css      # main[data-props] layout; the column's look
    └── index.html      # #slidebar removed; #panel's views
crates/qd/tests/
├── cli.rs, onslide.rs  # contracts' tests; bar tests moved to the column
examples/showcase/      # builds with effects, a build order, transitions with speed
README.md, skills/quartz-deck/SKILL.md
```

**Structure Decision**: the column's three views get a small module each
under `ui/props/`, because `editor.js` is already 2,300 lines. The
inspector keeps the tabs, focus and keyboard. The ops sit in `ops.rs` with
the class and meta ops they build on.

## PRs (order of work)

1. **US1, Format (P1):**
   - the column always shown;
   - views and tabs;
   - Format for the slide, the title slide and elements;
   - the Text tab;
   - the slide bar and element bar gone;
   - the bar tests moved.
2. **US2, Animate (P2):**
   - `qd build set/list/order`;
   - the Animate view;
   - build-order presenting test;
   - showcase builds.
3. **US3, Deck (P3):** dotted `qd meta set`, `qd meta size`, and the Deck view.
4. **US4, placement, and polish (P4):**
   - `properties.side` and docking;
   - the narrow-window layout;
   - README, skill, quickstart run, final re-check;
   - the person's talk (SC-006).

Each PR passes `/pr-audit`.

## Complexity Tracking

None: no principle is bent.
