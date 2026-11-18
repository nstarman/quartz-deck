# Implementation Plan: An inspector on the right when the window is wide

**Branch**: `019-mac-inspector` | **Date**: 2026-11-17 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/019-mac-inspector/spec.md`

## Summary

On a wide window, today's Properties panel becomes an inspector that is
always there, with tabs that follow the selection: Slide (or Title), and
for an element Element, Text and Position.
- **Same controls**: the slide bar, element bar and formatting bar move
  into its tabs as they are, so nothing floats over the slide, and every
  control keeps its handler (research R2).
- **When it shows**: one width check decides, from the window and the
  panes (R3).
- **Choice**: a setting and ⌥⌘I let the person hide it (R5).
- **Clean room**: names and look are our own (R1).

This is editor-only work, plus one setting in the core.

## Technical Context

**Language/Version**: Rust stable (edition 2024) for the setting; plain ES modules and CSS for the editor

**Primary Dependencies**: none new

**Storage**: the person's settings file (`ui.inspector`); nothing in the deck

**Testing**:
- `cargo test`:
  - a unit test in `settings.rs` for the new key;
  - browser tests in `tests/onslide.rs` at 1440 px and 1000 px, one per acceptance scenario: tabs follow the selection; no floating bar while it shows; each tab's control makes the same file change as the bar it replaces; resizing keeps the selection and typing; ⌥⌘I and the reload; F6 and the arrow keys;
  - the existing bar and panel tests keep passing below 1200 px, where nothing changes.
- The browser tests already set the window size. The default test window is 800 × 600, so existing tests run without the inspector unless a test widens the window.

**Target Platform**: macOS first; nothing Linux-hostile; the same page in VS Code's panel

**Project Type**: CLI + local web editor (existing)

**Performance Goals**: switching tabs or selection updates the inspector within one frame of today's panel render; no extra render of the deck

**Constraints**: no new edits (FR-004); the slide keeps at least 640 px; nothing changes below the width

**Scale/Scope**: five tabs; about 6 editor files touched

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status |
|---|---|
| I. Clean room | ✅ Only the general idea, common to many editors and allowed by `CLEANROOM.md`; our own tab names, layout and look; no other app's screenshots, icons or wording in the repo (R1). |
| II. `.qmd` is the truth | ✅ No deck state; every control makes the same patch as today. |
| III. CLI first | ✅ No new edit. The one new preference is in `qd settings`. |
| IV. Live preview | ✅ Unchanged: edits re-render through the preview loop. |
| V. Safe edits | ✅ Unchanged handlers, `--expect`, undo. A resize never drops an open edit (SC-003, tested). |
| VI. Test first | ✅ The browser tests for each scenario come before the code in each PR. |
| VII. Simplicity | ✅ Moves the existing controls into tabs instead of rebuilding them; one new setting; no new module beyond a small `inspector.js`. |
| Licences | ✅ No new dependency. |

Gate: **pass**.

## Project Structure

### Documentation (this feature)

```text
specs/019-mac-inspector/
├── plan.md, research.md, data-model.md, quickstart.md
├── contracts/
│   └── editor.md      # tabs, what each holds, actions and keys
└── tasks.md           # /speckit-tasks
```

### Source Code (repository root)

```text
crates/qd/src/
├── settings.rs         # ui.inspector
└── ui/
    ├── inspector.js    # new: width check, tabs, moving the bars in and out
    ├── editor.js       # panels split into tabs; bars skip floating while inside; Properties action
    ├── toolbar.js      # place/placeNear skip while the bar is in the inspector
    ├── actions.json    # View ▸ Inspector (Alt+Mod+I)
    ├── keys.js         # F6 between the slide and the inspector
    ├── editor.css      # the inspector's column, tabs and sections
    └── index.html      # tablist and tabpanels in #panel
crates/qd/tests/
└── onslide.rs          # browser tests at 1440 and 1000 px
skills/quartz-deck/SKILL.md, README.md   # the inspector, briefly
```

**Structure Decision**: one small new editor module for the inspector
itself. Everything it shows stays in the module that owns it today.

## PRs (order of work)

1. **US1, the element's tabs:** the width check, `ui.inspector`, the tab
   structure; the Element, Text and Position tabs with the element bar,
   formatting bar and element panel moved in. View ▸ Inspector and ⌥⌘I.
2. **US2, the slide's tab:** the slide bar and slide panel in the Slide
   tab; the title panel in the Title tab; the slide bar's row gone while
   the inspector shows.
3. **US3, adapting:** the source and History panes in the width check;
   keeping the selection and typing across resizes; F6 and the arrow keys;
   VS Code's panel.
4. **Polish:** docs, quickstart run, final re-check.

Each PR passes `/pr-audit`.

## Complexity Tracking

None: no principle is bent.
