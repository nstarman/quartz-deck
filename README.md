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
[002](specs/002-offline-export/spec.md) (offline export + PDF),
[003](specs/003-on-slide-editing/spec.md) (on-slide editing),
[004](specs/004-source-and-run/spec.md) (source pane, running code) and
[005](specs/005-shared-focus/spec.md) (shared focus) implemented; what
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
qd env init && qd env add matplotlib             # per-deck locked Python env (uv; or --tool pixi)
qd render                                        # final output; runs code cells in that env
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

### Shared focus (spec [005](specs/005-shared-focus/spec.md))

- **"This" means what you selected:** click an element, select a few words
  (on the slide or in the source pane) and tell an agent "make this shorter".
  It runs `qd focus` and gets the slide, the element(s) and the selected text,
  so it edits exactly that.
- **Agents can show you things:** `qd focus --set results/1` takes your editor
  to that slide and flashes the element. While you are typing nothing moves;
  a "Go there" offer waits instead.
- The focus stays in the running editor's memory, behind the same session
  token as running code; no other page can read or set it.

### Source and run (spec [004](specs/004-source-and-run/spec.md))

- **Source pane:** the **Source** button shows the `.qmd` beside the slides
  (resizable, or full width), with highlighting. Typing saves as you pause,
  under the same conflict rules as every other edit; the caret shows its
  slide, and clicking a slide element scrolls the source to it.
- **Environments:** each deck has its own locked Python environment —
  `qd env init` (uv; `--tool pixi` for pixi with PyPI packages), `qd env add
  numpy matplotlib`, `qd env sync` on a new machine. The manifest and lockfile
  sit beside the deck (or at the Quarto project root): commit them. Never
  `pip install`. `qd render` runs the cells inside that environment.
- **Run:** a ▶ beside each Python cell, ⇧Enter on the cell under the caret,
  Run all / Stop / Restart / Restart & run all in the source bar. Text,
  figures, tables and tracebacks show in the dock below the source and on the
  slide. Outputs live in qd's cache, never in the `.qmd`; they come back when
  the deck is reopened and are dimmed as stale once their cell changes.
- **Agents:** `qd run results/1` (or `--all`, `--fresh`, `--timeout`) returns
  each output as JSON, figures as file paths; with the editor open it uses the
  same live session and you see the outputs. `qd outputs` lists the stored
  ones.
- **Safety:** code runs only when you click Run (or press ⇧Enter) or call
  `qd run`. The server answers only to `127.0.0.1`/`localhost`, and running
  code needs the session token, which comes in the editor link that `qd
  serve` prints / `qd url` returns, never in the page. No other web page or
  link can make it run anything.

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
