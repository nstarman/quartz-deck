# Implementation Plan: Diagrams you edit on the slide

**Branch**: `015-graphviz-diagrams` | **Date**: 2026-11-15 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/015-graphviz-diagrams/spec.md`

## Summary

A `{dot}` cell pinned with `layout=neato`, `splines=ortho` and `pos="x,y!"`
becomes a diagram the presenter edits on the slide.
- **qd's core** gets a span-keeping DOT reader (`dot.rs`) and the
  `qd dot …` commands. Each command makes a minimal patch to the cell's
  text.
- **The editor** gets an overlay (`dot.js`) on the rendered SVG: select,
  drag, connect, rename in place, delete, Properties, Insert menus. It
  calls only those commands. Moves are sent as deltas in inches, measured
  on the SVG (research R2).
- **The `dot-image` render plugin** draws Graphviz's `image=`, which
  Quarto's Graphviz ignores. It finds the images in the deck's source,
  because no filter sees `{dot}` before Graphviz (R1, R5).
- **The live routing setting** lays the cell out in the browser while
  dragging, preferably with the Graphviz build from the person's own
  Quarto install (R6).

## Technical Context

**Language/Version**: Rust stable (edition 2024); plain ES modules for the editor; Lua for the `dot-image` filter

**Primary Dependencies**: no new crates. At run time:
- Quarto, for rendering, with its own Graphviz;
- for live routing only: Quarto's `graphviz-wasm.js`, else a pinned
  `@hpcc-js/wasm-graphviz` download.

**Storage**: the `.qmd` only. Live routing's engine, if downloaded, goes in qd's asset cache.

**Testing**:
- `cargo test`:
  - unit tests for the DOT reader (round trip over a corpus) and for each
    patch;
  - CLI integration tests for `qd dot` (JSON, byte-exact files,
    `--dry-run`, `--expect`, undo);
  - browser tests in `tests/onslide.rs` for drag, connect, rename,
    delete, Properties and Insert;
  - `plugins.rs` renders the `dot-image` example with plain Quarto.
- The offline check on the example.

**Target Platform**: macOS first; nothing Linux-hostile

**Project Type**: CLI + local web editor (existing), plus a Quarto extension

**Performance Goals**:
- After a drop, routed arrows within 3 s (SC-004); this is the preview's
  normal re-render.
- Live layout at most once per frame, with a 50-node diagram staying
  interactive.

**Constraints**:
- Byte-exact patches.
- Plain Quarto renders every diagram.
- Offline (live routing's engine is local or downloaded once).
- No writes outside the deck.

**Scale/Scope**: diagrams of up to about 50 nodes and 80 edges

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status |
|---|---|
| I. Clean room | ✅ Graphviz's public docs and Quarto's behaviour only. |
| II. `.qmd` is the truth | ✅ Positions, styles and images are DOT attributes in the cell. Patches are minimal and the round trip is byte-exact (SC-003). No sidecar. |
| III. CLI first | ✅ Every editor action is a `qd dot` command with JSON and stable codes (`E_UNSAFE_EDIT`, `E_NOT_FOUND`, `E_CONFLICT`, `E_INVALID_ARG`), listed in `qd schema` and the skill. |
| IV. Live preview | ✅ A drop re-renders through the existing preview loop. Live routing is a preview of the route, and never what's saved. |
| V. Safe edits | ✅ Ordinary edits: `--expect`, atomic writes, undo. A drop over a changed cell is refused. |
| VI. Test first | ✅ The reader's round-trip corpus and each patch are tested first; each command has a CLI test; each editor interaction has a browser test. |
| VII. Simplicity, plugins | ✅ No new crates. One render plugin (1.4.0's render-plugin kind). Live routing reuses Quarto's own Graphviz before downloading anything. |
| Licences | ✅ Our code, plus Graphviz EPL-1.0 (allowed for this engine by the person) in an Apache-2.0 wrapper, listed in the licence test's asset list. |

Gate: **pass**.

## Project Structure

### Documentation (this feature)

```text
specs/015-graphviz-diagrams/
├── plan.md, research.md, data-model.md, quickstart.md
├── contracts/
│   ├── cli.md         # qd dot … commands, JSON, errors
│   └── editor.md      # overlay interactions, Properties, menus, setting
└── tasks.md           # /speckit-tasks
```

### Source Code (repository root)

```text
crates/qd/src/
├── dot.rs              # new: DOT reader (spans), model, patches
├── cli.rs              # `qd dot` subcommands; schema
├── server.rs           # serves Quarto's graphviz-wasm for live routing; dot ops via the existing op route
├── settings.rs         # diagrams.live-layout
├── assets.rs           # pinned @hpcc-js/wasm-graphviz (fallback)
├── plugins.rs          # dot-image compiled in; installed when a diagram uses image=
└── ui/
    ├── dot.js          # new: diagram overlay (select, drag, connect, rename, delete, live routing)
    ├── actions.json    # Insert ▸ Diagram, Insert ▸ Box, diagram actions
    └── editor.js / sheet.js   # Properties fields for nodes, edges, graph
crates/qd/tests/
├── dot.rs              # new: CLI tests
├── fixtures/dot/       # new: DOT corpus for the round trip
├── onslide.rs          # browser tests for the overlay
└── plugins.rs          # dot-image example render
plugins/dot-image/      # _extensions/dot-image/{_extension.yml, dot-image.lua}, example.qmd, README.md
skills/quartz-deck/SKILL.md   # qd dot commands
```

**Structure Decision**: the existing crate gains one module (`dot.rs`) and
one editor module (`dot.js`). The plugin follows `plugins/<name>/` and is
compiled into qd like cite-links.

## PRs (order of work)

The PRs follow the stories, as in tasks.md:
1. **DOT reader + `qd dot show`:** the round-trip corpus, read-only
   reasons.
2. **US1:** `move` (`--at`/`--by`) and `pin`; the editor's select, drag
   with re-render on drop, Escape, nudge, "Pin layout".
3. **US2:** `add`, `connect`, `rename`, `set` (looks), `rm`, `flip`,
   `new`; the editor's connect handles, add on empty drop, rename in
   place, Delete, Properties, Insert menus.
4. **US3:** `image=` with a spacer label, and the `dot-image` plugin with
   its example, render and offline tests.
5. **US4:** live routing (Quarto's engine served, or the pinned
   fallback).

Then, with the person: their pipeline diagram rebuilt as the proving case
(SC-006). Each PR passes `/pr-audit`.

## Complexity Tracking

None. No constitution violations.
