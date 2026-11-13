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
[008](specs/008-usability/spec.md) (usability),
[009](specs/009-layouts-and-placement/spec.md) (layouts and placement),
[010](specs/010-one-file-deck/spec.md) (one-file deck) and
[011](specs/011-export-formats/spec.md) (export formats) implemented; what
comes next is in [`specs/ROADMAP.md`](specs/ROADMAP.md). See also the
[constitution](.specify/memory/constitution.md).

## Install

Needs Rust ≥ 1.89; for preview/render [Quarto](https://quarto.org/docs/get-started/) ≥ 1.4;
for offline check/export/PDF a Chrome or Chromium (or qd downloads Chrome for Testing).

```bash
cargo install --path .
qd skill --install ~/.claude/skills/quartz-deck   # teach Claude Code to use qd
```

`skills/quarto-revealjs/` is the agent Skill for offline, archivable Quarto reveal.js decks built
with qd, listed in [nstarman/skills](https://github.com/nstarman/skills). Unlike the rest of this
repository it is MIT-licensed (`skills/quarto-revealjs/LICENSE`); its end-to-end test is
`tests/skills/quarto-revealjs/run.sh`.

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
- **Run:** a ▶ beside each code cell, ⇧Enter on the cell under the caret,
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

### Run on the slide (spec [013](specs/013-run-on-slide/spec.md))

- **On the slide:** pointing at a code cell, or selecting it, shows a ▶ at
  its top right. ⇧Enter runs the selected cell. While a cell runs, the ▶
  turns into ■ to stop it, and a bar at the cell's left edge shows running,
  error or stopped. The new output replaces the old one in place, on the same
  slide. A cell with hidden code and nothing to show yet appears as "Code
  cell, not run yet", in the editor only.
- **In the pop-up:** the box that opens over a code cell has **Run ⇧Enter**.
  It saves your change as one undo step, runs the cell and stays open, so a
  plot can be tuned without leaving the slide. If the file changed underneath,
  it says so and runs nothing.
- **The deck's own engine:** cells run the way `quarto render` runs them.
  - A Python deck uses its locked environment, as above.
  - A deck with `jupyter: NAME` runs that kernel. Any other Jupyter deck
    runs the first installed kernel for its language, without needing a qd
    environment.
  - An `{r}` deck runs in one R session (knitr), with plots at the deck's
    figure size.
  - An `engine: julia` deck runs in one Julia session, using the deck's own
    project.
  - Observable JS cells run in the page: their ▶ reloads the preview on the
    same slide.

  R and Julia are your own installs, the same ones Quarto uses. The source
  bar's badge names the engine (`R · idle`).
- **Never when presenting:** the presented deck, and a page opened without
  the session token, show no ▶.

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

### Other formats (spec [011](specs/011-export-formats/spec.md))

```bash
qd export --list            # every format, and whether it is ready (Beamer and PDF need TeX)
qd export --to pptx         # export/<deck>.pptx: slides, notes in the notes, backgrounds
qd export --to beamer       # export/<deck>.pdf through LaTeX: pauses become overlays
qd export --to docx         # a handout: one section per slide, notes under "Speaker notes"
qd export --to gfm          # Markdown, figures in export/<deck>_files/
qd export --to rst          # any other writer Quarto's Pandoc has, by name
qd --dry-run export --to pptx   # what won't carry over, without rendering
```

Every format is one Quarto render of a copy of the deck (`quarto render --to
<format>`), so code runs and callouts, cross-references and figures are kept,
and any writer Quarto's Pandoc has works by name; the deck is never changed. The result
lists what a format can't keep, slide by slide: placed boxes go back in the
flow, pauses and fragments are shown all at once, background images are kept
only in PowerPoint, interactive figures only in the HTML page and the
notebook. Code stays hidden, as on the slides, unless the deck sets `echo`.
In the editor, **Export ▾** (or File ▸ Export) has every format; a format
whose tool is missing is greyed out and says what to install. TeX comes from
`quarto install tinytex`; Typst comes with Quarto.

The editor and the CLI can be used at the same time; edits carry a version
(`--expect`, or per element `--expect-text`) and are refused rather than lost
if the file changed underneath.

### History (spec [012](specs/012-history/spec.md))

```bash
qd history                    # the versions that changed the deck, newest first
qd history --slide methods    # only the versions in which that slide changed (or --element REF)
qd diff HEAD~3                # the slides changed since then: added, removed, moved, changed
qd diff a1b2c3d HEAD          # between two versions
qd restore HEAD~3 --slide methods   # put one slide back (--element REF, or the whole deck)
qd history --init             # start a history: a git repository beside the deck
```

History reads the deck's versions from git, so the person's own commits show
up next to qd's. In the editor, **View ▸ History** (⌥⌘H) lists them. Choosing
one renders the deck as it was beside the current one, without running code,
and marks the slides that changed since. **Compare this version with now**
lists those slides with their source diffs. **Restore slide from this
version** (slide menus), **Restore from this version** (element menus) and
**Restore this version…** (View) put things back as an ordinary edit, so
undo takes it back again. qd never rewrites history.

**Save as git commit** (Settings, off by default) commits the deck file and
nothing else after a pause in editing (30 s by default; edits during the
pause become one commit), with a message such as `qd: title methods ×3
(agent)`. The History panel counts down to it. A CLI edit with no `qd serve`
running commits at once. Commits follow your git config, so hooks run and
commits are signed if you sign. When git refuses (a hook, a locked key) or
the repository is in a merge or rebase, the edit is kept and qd tries again
at the next pause. qd never pushes.

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
