# Implementation Plan: Export Formats

**Branch**: `011-export-formats` | **Date**: 2026-11-12 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/011-export-formats/spec.md`

## Summary

`qd export --to <format>` and an Export ▾ menu cover every format Quarto
and its Pandoc can write. A new module `src/formats.rs` holds the format
table (name, label, group, extension, route, tool, carry-over column). It
checks for the tool, renders a temp copy of the deck with Quarto (or Quarto
to Markdown, then Pandoc), applies one embedded Lua filter that adapts
slides for other formats, moves the output into place only on success,
and reports losses computed from the deck model. Plain `qd export` and its
PDF stay as they are. The code starts after PR #43 merges (research R8).

## Technical Context

**Language/Version**: Rust stable (edition as in Cargo.toml); plain ES modules for the editor
**Primary Dependencies**: existing crates only (clap, serde_json, axum, yaml-rust2); no new ones
**Storage**: files: the output next to the deck in `export/`; nothing persistent besides it
**Testing**: `cargo test` (unit tests in `formats.rs`, CLI tests in `tests/cli.rs`, Quarto-backed tests that run on CI as other render tests do, browser tests in `tests/onslide.rs`)
**Target Platform**: macOS first, Linux
**Project Type**: CLI plus a localhost web editor (single binary)
**Performance Goals**: the tool check fails in under 2 s (SC-004); an export takes the render time (10–45 s measured, research)
**Constraints**: deck file byte-for-byte unchanged; no partial output; token required for code-running ops
**Scale/Scope**: 11 menu formats + ~60 other Pandoc writers; one new module (~250 lines), one Lua filter (~20 lines), small changes in cli.rs, server.rs, editor.js, actions.json

External tools: Quarto 1.11.9 (bundles Pandoc 3.10 and Typst 0.15.1);
TeX (TeX Live or TinyTeX) only for `pdf` and `beamer`; uv/pixi as 004 for
decks with code; the 002 browser for `revealjs-pdf`. Third-party
licences: no new dependencies.

## Constitution Check

| Principle | Check |
|---|---|
| I. Clean room | Only Quarto/Pandoc docs and our own measurements (research R1). Pass. |
| II. `.qmd` is the truth | Exports read the deck and write a temp copy; the deck is never written (FR-003, SC-005 hash test). Pass. |
| III. CLI first | `qd export --to`, `qd export --list` come first; the menu calls them through `/api/op`. JSON result, `E_NO_TEX` error code, `qd schema` updated. Pass. |
| IV. Live preview | Unaffected; exports use the same Quarto lock, so they wait for a preview render instead of breaking it. Pass. |
| V. Safe concurrent edits | Exports don't edit. The export uses a copy of the text read at start; the result says if the deck changed meanwhile. Pass. |
| VI. Test first | Unit tests for the table, losses and route choice; CLI tests for each format group with the fixture deck; a browser test for the menu. Pass. |
| VII. Simplicity | One module, one filter, no new crates; losses from the deck model instead of parsers per output format (R6); no still-image converter (R5). Pass. |
| Constraints: token for code | `/api/op` starts requiring the token for `render`, `export`, `pdf`, which closes an existing gap (R7). Pass. |

Re-check after design: unchanged. No Complexity Tracking entries.

## Project Structure

### Documentation (this feature)

```text
specs/011-export-formats/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── cli.md
│   └── http.md
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
src/
├── formats.rs           # NEW: Format table, list(), check_tool(), losses(), run()
├── formats.lua          # NEW: the export filter (include_str!)
├── cli.rs               # Export: --to, --list; schema; API_OPS unchanged
├── render.rs            # quarto_cmd/run_quarto/failed made pub(crate)
├── server.rs            # /api/op: token for render/export/pdf; href for new extensions
├── error.rs             # NoTex → E_NO_TEX (like E_NO_BROWSER)
└── ui/
    ├── actions.json     # export-to-<name> actions, export-other; File ▸ Export submenu
    ├── editor.js        # Export ▾ button and dialog, op() sends the token, result with losses
    └── editor.css       # loss list
tests/
├── fixtures/export/carry.qmd, bg.png   # the research deck
├── cli.rs               # --list, tool missing, --to unknown, bad names
├── export_formats.rs    # NEW: Quarto-backed: pptx, beamer, docx, typst, gfm, ipynb, rst; deck hash
└── onslide.rs           # Export ▾ menu, disabled reason, result dialog
```

**Structure Decision**: single crate, as for every spec so far. New
code goes in `formats.rs` so PR #43's changes to `export.rs` don't
conflict (R8).

## Design

### Formats (`formats.rs`)

A `const FORMATS: &[Format]` covers the menu formats. `Format` has
`name`, `label`, `group` (Slides, Documents), `ext`, `tool` (None, Tex,
Browser) and `keeps: fn(Feature) -> Keep`, written as one match per
format, which is the spec's table column. `revealjs` and `revealjs-pdf`
appear in the list but route to the existing `export::run` (with
`pdf: true` for the PDF).

`list()` returns the menu formats followed by every writer from
`quarto pandoc --list-output-formats` (cached for the process) that isn't
already there, in group Other, with ext from a small map (`rst→rst`,
`org→org`, `latex→tex`, `asciidoc→adoc`, …, default `txt`), plus `ready`
and `needs`.

### Route and run

`run(store, name, out) -> Result<Value>`:

1. Look up the format (unknown → `InvalidArg` with a hint to run
   `qd export --list`). Check the tool (`NoTex`), before any file is
   written.
2. Read the deck text once and compute the losses from it (`losses(d,
   fmt)`).
3. Write the copy `.<stem>.qd-export-<nanos>.qmd`, the filter
   `.qd-export-<nanos>.lua` and the metadata `.qd-export-<nanos>.yml` next
   to the deck. The metadata holds `echo: false` (unless the deck sets
   `echo`) and the filter.
4. Quarto route: `quarto render <copy> --to <name> --metadata-file <yml>
   -o <tmp-out-name>`. Pandoc route: render `--to md`, then
   `quarto pandoc <copy>.md -t <name> -o <tmp-out>`. Both go through
   `render::quarto_cmd`/`run_quarto`, so the environment and lock apply.
5. Move `<tmp-out>` to `out` (default `export/<stem>.<ext>`) and move the
   `<stem>_files/` folder Quarto writes for Markdown next to it. Remove
   the copy, the filter and the metadata in every case (a guard struct
   with Drop).
6. Return the result (data-model.md). If the deck's text changed during
   the run, add the note "the deck changed while exporting; this is the
   version from <time>".

### Losses

`losses(d, fmt)` walks the slides once. For each slide it finds which
`Feature`s it uses: Notes, Pause, Fragment, Columns, Placed, BgImage,
BgMedia, Interactive. Theme is checked once on the deck meta. For each
feature where `fmt.keeps(f)` is not Kept, it adds or extends
`{feature, slides: [ref…], what}`, where `what` is the table's wording
("in the flow", "not in the file", "all steps shown at once",
"not shown (Beamer hides notes)"). Detection reuses the deck model:
`BlockKind::Notes`, `Pause`, `Div` classes, `Box`, slide attrs, and
`export::uses_plotly` plus 004's outputs for interactive cells.

### Editor

- An **Export ▾** button in the header opens a menu built from
  `qd export --list` (fetched once through `/api/op`, refreshed on open):
  the groups Slides and Documents, then "Other format…", which opens a
  sheet with a filterable list. Formats that aren't ready are disabled,
  with `needs` as the tooltip.
- File ▸ Export becomes a submenu with the same entries. The existing
  `export` and `export-pdf` actions keep their ids and keys, as Slides ▸
  "Web page (works offline)" and "Slides PDF".
- Picking a format opens the export sheet: the path (editable), the losses
  that the menu's dry-run already knows (`qd export --to X --dry-run`
  returns the losses without rendering), with a warning line when notes
  would be lost (FR-014), and Export. While the export runs, the status
  shows "Exporting PowerPoint…" and editing continues. The result sheet
  shows the file with a "Show in Finder" button (an existing `reveal`
  helper if there is one, otherwise a new `/api/reveal`, token-checked)
  and the losses by slide, each slide name a link that selects it.
- `op()` sends `X-QD-Token`.

### Server

`/api/op` checks `authorized()` when `op` is `render`, `export` or `pdf`.
The `href` mapper links any output inside the deck folder, not only html
and pdf, so the result can link the file.

## Phasing (for tasks)

- Setup: wait for #43, rebase onto `origin/main`; fixture deck.
- Foundational: `render.rs` helpers made `pub(crate)`; the `NoTex`
  error; the token on `/api/op` and in `op()`.
- US1 (P1): `formats.rs` with pptx and beamer, the filter, losses, `--to`,
  the result; tests.
- US2 (P1): the tool check, `--list`, `ready`/`needs`; the Export ▾ menu
  with disabled formats; tests.
- US3 (P2): the document formats (table columns and tests); notes in
  documents; extra files for Markdown.
- US4 (P3): the Pandoc route and "Other format…".
- Polish: README, ROADMAP, action inventory, quickstart run, concision
  audit.
