# Implementation Plan: Arrows on the slide

**Branch**: `016-slide-arrows` | **Date**: 2026-11-17 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/016-slide-arrows/spec.md`

## Summary

An arrow is a slide paragraph holding one quarto-arrows shortcode with
`position="absolute"`, placed in the slide's own pixels (research R1).
- **qd's core** gets a small module (`arrows.rs`) that reads an arrow's
  options with their spans, and the `qd arrow …` commands. Each makes a
  minimal patch to the arrow's line (R4).
- **Getting the extension:** the first arrow downloads quarto-arrows
  v0.3.0's six files once, pinned to its commit and checked by SHA-256, into
  qd's cache, and installs them into the deck. qd doesn't ship the
  extension, because its repository has no licence file (R3). No `filters:`
  entry is needed (R2).
- **The editor** gets an overlay (`ui/arrows.js`): draw, select, drag ends
  and the line, bend by a middle handle, label by double-click, Properties,
  "Appears on click", with the existing snap guides (R5–R8). It calls only
  the commands.

## Technical Context

**Language/Version**: Rust stable (edition 2024); plain ES modules for the editor

**Primary Dependencies**: no new crates. At render time, Quarto plus quarto-arrows v0.3.0 in the deck's `_extensions/`; downloaded once with qd's existing `download_verified`.

**Storage**: the `.qmd` and the deck's `_extensions/emilhvitfeldt/arrows/`; the downloaded files in qd's cache.

**Testing**:
- `cargo test`:
  - unit tests in `arrows.rs`: reading options with spans (quoted, single-quoted, bare), read-only reasons, each patch keeping every other byte, the bend formula;
  - CLI tests (`tests/arrows.rs`): every command's JSON and file, `--dry-run`, `--expect`, undo, offline `E_DOWNLOAD` with nothing changed;
  - browser tests in `tests/onslide.rs`: draw, select, drag ends and line with snapping, nudge, Delete, Escape, bend, label, Properties, "Appears on click";
  - a plain `quarto render` of an example deck with arrows: ends land in slide pixels (SC-002); offline check and one-file export (SC-004).
- Tests that need the extension use one shared download, as the live-routing test did, and skip when offline.

**Target Platform**: macOS first; nothing Linux-hostile

**Project Type**: CLI + local web editor (existing); uses a third-party Quarto extension

**Performance Goals**: an arrow's drop shows the rendered arrow within the preview's usual re-render; provisional drawing at the pointer's rate

**Constraints**: byte-exact patches; plain Quarto plus quarto-arrows renders every arrow; offline after the first download; no writes outside the deck and qd's cache

**Scale/Scope**: up to about 20 arrows on a slide

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status |
|---|---|
| I. Clean room | ✅ quarto-arrows' public docs and behaviour; its code is downloaded and run by Quarto, not copied into qd. |
| II. `.qmd` is the truth | ✅ An arrow is a shortcode line in the slide; every option lives in it. No sidecar. Patches change only the edited options (SC-003). |
| III. CLI first | ✅ Every gesture is a `qd arrow` command (or `qd block rm`), with JSON and stable codes, in `qd schema` and the skill. |
| IV. Live preview | ✅ A drop re-renders through the existing preview loop; the provisional drawing is never what's saved. |
| V. Safe edits | ✅ Ordinary edits: `--expect`, atomic writes, undo. Arrows qd can't read safely stay text-only. |
| VI. Test first | ✅ The reader and each patch are unit-tested first; each command has a CLI test; each gesture a browser test. |
| VII. Simplicity, plugins | ✅ The existing library (the person's choice) rather than our own filter; no new crates; one new core module and one editor module. |
| Licences | ✅ quarto-arrows says MIT in its README; qd downloads it, checked by SHA-256, and doesn't redistribute it (no licence file). Listed in the licence test's downloaded assets. |

Gate: **pass**.

## Project Structure

### Documentation (this feature)

```text
specs/016-slide-arrows/
├── plan.md, research.md, data-model.md, quickstart.md
├── contracts/
│   ├── cli.md         # qd arrow … commands, JSON, errors
│   └── editor.md      # overlay gestures, Properties, menus
└── tasks.md           # /speckit-tasks
```

### Source Code (repository root)

```text
crates/qd/src/
├── arrows.rs           # new: arrow reader (option spans), patches, commands
├── assets.rs           # quarto-arrows' six files pinned by SHA-256; a multi-file ensure
├── plugins.rs          # install an extension from files on disk (the downloaded one), version-aware
├── cli.rs              # `qd arrow` subcommands; schema; API_OPS
└── ui/
    ├── arrows.js       # new: overlay (draw, select, drag, bend, label, snapping)
    ├── guides.js       # shared nearest/SNAP_PX (spec 015 T037)
    ├── actions.json    # Insert ▸ Arrow
    └── editor.js       # Properties fields, element bar's fragment for arrows, guide lines
crates/qd/tests/
├── arrows.rs           # new: CLI tests
├── onslide.rs          # browser tests
├── plugins.rs          # example render with arrows
└── licenses.rs         # downloaded assets: quarto-arrows
skills/quartz-deck/SKILL.md   # qd arrow commands
```

**Structure Decision**: one core module and one editor module, as for spec
015. No plugin folder: the extension is downloaded, not ours.

## PRs (order of work)

1. **Arrow reader + `qd arrow show/list` + the download and install:** the
   reader's unit tests, read-only reasons, the pinned download, installing
   into a deck, offline `E_DOWNLOAD`.
2. **US1:** `add`, `move`, `rm`; the editor's Insert ▸ Arrow, select, drag
   ends and the line with snapping, nudge, Delete, Escape.
3. **US2:** `bend`, `set` (style); the middle handle and the Properties
   fields.
4. **US3 + US4:** labels (double-click sheet, label position) and "Appears
   on click".
5. **Polish:** docs, quickstart run, the person's real slide (SC-006), final
   re-check.

Each PR passes `/pr-audit`.

## Complexity Tracking

None: no principle is bent.
