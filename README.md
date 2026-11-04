# quartz-deck

A local GUI + CLI editor for [Quarto](https://quarto.org) reveal.js
presentations, written in Rust.

- The `.qmd` file is the source of truth. The GUI and the CLI both edit it
  with minimal text patches, so it stays readable and diff-friendly.
- `qd serve` opens an editor in your browser where the slide itself is the
  editing surface: type into titles and text with a formatting toolbar, edit
  math/code as source in place, right-click for every action, drag to
  reorder, drag/resize boxes, undo/redo.
- Every editing action is also a CLI command with JSON output, so an LLM
  (e.g. Claude Code) can edit the deck in-session and watch the same
  browser preview.

Status: specs [001](specs/001-core-editor/spec.md) (editor),
[002](specs/002-offline-export/spec.md) (offline export + PDF) and
[003](specs/003-on-slide-editing/spec.md) (on-slide editing) implemented; what
comes next is in [`specs/ROADMAP.md`](specs/ROADMAP.md). See also the
[constitution](.specify/memory/constitution.md).

## Install

Needs Rust ≥ 1.89; for preview/render [Quarto](https://quarto.org/docs/get-started/) ≥ 1.4;
for offline check/export/PDF a Chrome or Chromium (or qd downloads Chrome for Testing).

```bash
cargo install --path .
qd skill --install ~/.claude/skills/quartz-deck   # teach Claude Code to use qd
```

## Use

```bash
qd new talk.qmd --title "My talk"                # or work on an existing deck
qd add --title "Results" --md $'- one\n- two'    # JSON out; target = "results"
qd set results --attr background-color=aquamarine
qd box add results --image figures/plot.png --at 100,150 --size 600,400
qd --human slides
qd check                                         # --render also runs Quarto
qd serve --open                                  # browser editor + live preview
qd render                                        # final output; reports its path
qd undo
```

The deck is the only `*.qmd` in the current folder, or `--deck PATH` /
`$QD_DECK`. `qd schema` describes every command and output; errors are JSON on
stderr with a stable `code` and a `hint`. Decks inside a Quarto project
(`_quarto.yml`, e.g. the `quarto-revealjs` skill's offline setup) work as is.

### Editing on the slide (spec [003](specs/003-on-slide-editing/spec.md))

- **Type:** click a title, paragraph, list, quote or text box and type; the
  toolbar (⌘B/⌘I/⌘U/⌘K…) formats. Enter (titles) or ⌘Enter / clicking
  elsewhere saves, Esc cancels. Only the bytes you changed are written: line
  wrapping, quotes, indentation and untouched markup stay as they were.
- **Source in place:** math, code, callouts and anything else that can't be
  typed into open their markdown over the element (or Alt+click any element;
  Alt+click the slide background for the whole slide; ⌥Enter while typing).
- **Actions:** right-click an element, a slide in the list or an empty slide
  area for the full menu (Shift+F10 from the keyboard); the slide bar above
  the preview sets background, transition and hidden; the element bar on a
  selection toggles "appears on click". Hover an element for **+** (insert)
  and **⠿** (drag). Tab / Shift+Tab in the slide move the selection; ⌘C/⌘X/
  ⌘V/⌘D, ⌫, ⌥↑/⌥↓ act on it. The old form panel is the **Source** button.
- **With an agent:** you and Claude can edit at the same time. Saves are
  checked per element (`qd edit … --expect-text <hash>`, hashes in `qd
  show`); if both change the same element you see both versions and choose.
  Re-renders wait while you type.
- CLI equivalents: `qd block add|move|rm|dup|class`, `qd edit`, `qd title`,
  `qd set` — see `qd schema`.

### Offline export (spec [002](specs/002-offline-export/spec.md))

```bash
qd check --offline        # will it work with no network? problems per slide, with hints
qd export --pdf           # export/<deck>.html (self-contained) + .pdf, checked offline
qd pdf --notes            # re-print with speaker notes on separate pages
qd browser                # which Chrome is used (qd can download a pinned one)
```

Export renders a copy of the deck with MathJax (and plotly.js) loaded from
local, SHA-256-verified copies, ships background videos/iframes beside the
HTML, and only replaces the previous export if the offline check passes. The
deck and `_quarto.yml` are never modified. The first export needs the network
once (MathJax; Chrome for Testing if no Chrome is installed).

The editor and the CLI can be used at the same time; edits carry a version
(`--expect`, or per element `--expect-text`) and are refused rather than lost
if the file changed underneath.

## Develop

```bash
cargo test                                      # ~150 tests; Quarto/Chrome-backed ones skip without them
cargo test --release -- --ignored perf_100 export_30 fifty   # slow checks (as in CI)
cargo clippy --all-targets -- -D warnings
```

Built with [spec-kit](https://github.com/github/spec-kit): `/speckit-plan`,
`/speckit-tasks`, `/speckit-implement` from Claude Code; every PR gets a
concision audit (`/pr-audit`). Verification notes are in each spec's
`checklists/`.

## License

Proprietary, personal use only. See [LICENSE](LICENSE) and
[CLEANROOM.md](CLEANROOM.md).
