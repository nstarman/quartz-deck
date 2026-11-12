# quartz-deck

> [!WARNING]
> **Alpha software. Please don't use it.** quartz-deck is an unfinished
> personal project, published so that it can be read. It changes and breaks
> without notice, it may damage the decks it edits, and it is not licensed
> for use by anyone else (see [LICENSE](LICENSE)). No support, no issues or
> pull requests, please.

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
[004](specs/004-source-and-run/spec.md) (source pane, running code),
[005](specs/005-shared-focus/spec.md) (shared focus),
[006](specs/006-vscode/spec.md) (VS Code),
[007](specs/007-app-shell/spec.md) (menus, settings, shortcuts) and
[008](specs/008-usability/spec.md) (usability) and
[010](specs/010-one-file-deck/spec.md) (one-file deck) implemented; what
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
qd add --layout two-columns --after results --title "Before and after"
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
  toolbar (⌘B/⌘I/⌘U/⌘K…) formats. Enter (titles), ⌘Enter, Esc or clicking
  elsewhere saves; Undo takes it back. Only the bytes you changed are written: line
  wrapping, quotes, indentation and untouched markup stay as they were.
- **Source in place:** math, code, callouts and anything else that can't be
  typed into open their markdown over the element (or Alt+click any element;
  Alt+click the slide background for the whole slide; ⌥Enter while typing).
  ⌘Enter or Esc saves it; its **Cancel** button discards.
- **Actions:** right-click an element, a slide in the list or an empty slide
  area for the full menu (Shift+F10 from the keyboard); the slide bar above
  the preview sets background, transition and hidden; the element bar on a
  selection toggles "appears on click". Hover an element for **+** (insert)
  and **⠿** (drag). Tab / Shift+Tab in the slide move the selection; ⌘C/⌘X/
  ⌘V/⌘D, ⌫, ⌥↑/⌥↓ act on it. The old form panel is View ▸ Properties panel.
- **With an agent:** you and Claude can edit at the same time. Saves are
  checked per element (`qd edit … --expect-text <hash>`, hashes in `qd
  show`); if both change the same element you see both versions and choose.
  Re-renders wait while you type.
- CLI equivalents: `qd block add|move|rm|dup|class`, `qd edit`, `qd title`,
  `qd set` — see `qd schema`.

### Menus, settings and shortcuts (spec [007](specs/007-app-shell/spec.md))

- **Menu bar:** File, Edit, Insert, Slide, View, Run and Help hold every
  action the editor has, each with its shortcut, so nothing needs a
  right-click or a hidden key. F10 moves the keyboard to it; arrows, Enter
  and Esc work as in any app. When the window is too narrow it folds into
  one **☰**.
- **Shortcuts:** **?** (or Help ▸ Keyboard shortcuts) lists every key, by
  menu, with a search box. It comes from the same list as the menus and the
  keys themselves, so it can't be out of date.
- **Settings** (File ▸ Settings…, ⌘,): source pane side and whether it starts
  open, when the source pane saves, theme, confirm before deleting a slide,
  shortcuts in tooltips. They are yours, not the deck's: kept in qd's config
  folder, the same in every deck, after a restart and in VS Code's panel, and
  applied at once in every open editor.
- **The same rules everywhere:** Esc backs out one level (an edit is saved,
  then the selection cleared); Enter goes in. ⌘X/⌘C/⌘V cut, copy and paste
  elements, and in the slide list whole slides (pasted slides get fresh ids,
  so they paste into another deck too). Text from another app pastes as a
  plain paragraph. Every edit is one undo step, and focus always shows.
- CLI: `qd actions` (every action, its menu and keys), `qd settings`, `qd
  settings set source.side right`, `qd settings reset`, `qd paste --after
  intro --md -`.

### Made easy to use (spec [008](specs/008-usability/spec.md))

A design audit against Apple's Human Interface Guidelines
([audit](specs/008-usability/audit/README.md), [status](specs/008-usability/audit/status.md))
and its fixes:

- **The slide list** shows thumbnails of every slide, the title slide first.
  A **+** sits between every two slides and after the last (Enter on a slide
  adds one after it); new slides start untitled, with "Click to add title" on
  the canvas only. Drag a slide by its **⠿** to move it: the others open a gap
  where it will land, and Esc cancels. ⇧-click picks a range and ⌘-click one
  more; dragging, Delete, copy and cut then act on all of them.
- **▶ Present** (top right) shows the deck full screen from the slide you're
  on; its **▾** starts from the beginning or opens it in a browser tab.
- **Images:** Insert ▸ Image… lists the images beside the deck with
  previews; dropping an image file on a slide copies it into `images/` and
  places it where it fell. CLI: `qd images`, `qd image add chart.png --slide
  results --end`.
- **The editor's own small sheets** ask for a link, a colour (with None), a
  class or packages, and confirm a delete, instead of the browser's pop-ups.
- **One top row:** the menus, the deck's title and file, and Present. The
  status speaks in words and only shows when it needs you. Menu names are
  plain ("Render with Quarto", "Edit markdown", "Properties").
- **Tidier bars:** the formatting bar shows the common formats with the rest
  under **Aa**; the source pane's bar is ▶ Run all, ■ Stop, Python's state and
  ···. Insert adds code cells, callouts, columns, speaker notes, tables and
  quotes. Bars never cover the slide's content, and every control meets the
  contrast and size minimums in light, dark and high contrast.
- **A narrow window** (under 600 px) swaps the list for a slide picker in the
  top row, and the slide's properties go under ···.

### Layouts and placement (spec [009](specs/009-layouts-and-placement/spec.md))

Start a slide from a layout, then put anything anywhere on it. The deck stays
plain Quarto: a layout's areas and a placed element are Quarto's own
`.absolute` boxes.

- **Layouts:** the slide list's **+**, Insert ▸ Slide after… and ⇧⌘N open a
  gallery of 12 layouts (title + content, two or three columns, image + text,
  full image, big number, quote, section, blank…), each drawn from its own
  boxes. The empty areas say "Click to add text" or "Drop an image" on the
  canvas only; a deck rendered with Quarto shows nothing for them. CLI: `qd
  layouts`; `qd add --layout NAME` returns the placeholders' refs to fill
  with `qd edit`.
- **Place anything freely:** ⌥-drag an element out of the flow, or Arrange ▸
  Place freely. It keeps its bytes inside a positioned box; drag it, resize
  it with the handles, nudge it with the arrow keys (⇧ for 10 px). Arrange ▸
  Put back in the flow leaves the file exactly as it was. Columns, callouts
  and speaker notes can't be placed, and the menu says why. CLI: `qd block
  place REF --at X,Y`, `qd block unplace REF`.
- **Guides:** while you drag or resize a placed box, it snaps to the slide's
  edges and centre and to other elements, with a guide line. Hold ⌘ to place
  it freely; View ▸ Snap to guides turns it off.
- **Front and back:** Arrange ▸ Bring to front (⌥⇧⌘F), Bring forward,
  Send backward, Send to back (⌥⇧⌘B). Boxes stack in the file's order, so
  this also changes the order fragments appear in. CLI: `qd box front REF` …
- **Your own layouts:** Slide ▸ Save slide as layout… keeps the slide's
  structure (never its text) with the deck in `_layouts/`, or in your own
  layouts for every deck. They head the gallery, with Rename and Delete.
  CLI: `qd layout save SLIDE NAME [--mine]`, `qd layout rename`, `qd layout rm`.

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

### Offline export (specs [002](specs/002-offline-export/spec.md), [010](specs/010-one-file-deck/spec.md))

```bash
qd check --offline        # will it work with no network? problems per slide, with hints
qd export --pdf           # export/<deck>.html (one file) + .pdf, checked offline
qd export --strip-notes --out share   # a copy to share, without speaker notes
qd pdf --notes            # export/<deck>-notes.pdf: speaker notes on separate pages
qd browser                # which Chrome is used (qd can download a pinned one)
```

Export renders a copy of the deck with MathJax (and plotly.js) loaded from
local, SHA-256-verified copies, puts its apps (`<iframe data-app="…">`,
background iframes), `<object>` files and SVG `<image>` files inside the one
HTML file, stores a repeated image once, ships only background videos beside
it, and only replaces the previous export if the offline check passes. The
check blocks the network for the deck and its apps, visits every slide and
step, and also lists missing files, elements outside the slide (warnings) and
the largest parts of the file. With ghostscript (`gs`) installed, PDFs are
recompressed to 200 dpi JPEG. A deck's Makefile needs only these commands. `qd bib --zotero KEY` writes
the References slide's list from a Zotero collection, as an ordinary edit.
The deck and `_quarto.yml` are never modified. The first export needs the network
once (MathJax; Chrome for Testing if no Chrome is installed).

The editor and the CLI can be used at the same time; edits carry a version
(`--expect`, or per element `--expect-text`) and are refused rather than lost
if the file changed underneath.

### VS Code (spec [006](specs/006-vscode/spec.md))

A small extension in [`vscode/`](vscode/README.md) shows the editor in a panel
beside the `.qmd`, next to the Quarto extension:

```bash
cd vscode && npx @vscode/vsce package && code --install-extension quartz-deck-*.vsix
```

- **Open in quartz-deck** (the editor title button) on a `.qmd`.
- While the panel is open, the VS Code buffer is the deck: edits on the
  slides, your agent's `qd` edits and undo all land there as ordinary unsaved
  edits (⌘Z undoes them), and the file changes when you save. Typing in VS
  Code updates the slides.
- The cursor and the slide follow each other, both ways; `qd focus` sees the
  VS Code cursor.
- Render and export from the command palette ("quartz-deck: …").
- In the panel the menus are one **☰**, and keys VS Code uses (⌘, ⌘S ⌘B ⌘K
  ⇧⌘X ⇧⌘N ⇧⌘D) stay VS Code's; those actions are in the ☰ menu, and **?**
  lists the keys that work in the panel.
- Remote-SSH is designed in (`qd` runs on the remote host and the panel
  reaches it through VS Code's port forwarding), but not tried yet.

## Develop

```bash
cargo test                                      # ~150 tests; Quarto/Chrome-backed ones skip without them
cargo test --release -- --ignored perf_100 export_30 fifty   # slow checks (as in CI)
cargo clippy --all-targets -- -D warnings
node --test vscode/test/*.test.js               # the VS Code extension
```

Built with [spec-kit](https://github.com/github/spec-kit): `/speckit-plan`,
`/speckit-tasks`, `/speckit-implement` from Claude Code; every PR gets a
concision audit (`/pr-audit`). Verification notes are in each spec's
`checklists/`.

## License

Source-available, all rights reserved: you may read the code here, but it is
not licensed for use, copying or redistribution. See [LICENSE](LICENSE) and
[CLEANROOM.md](CLEANROOM.md).
