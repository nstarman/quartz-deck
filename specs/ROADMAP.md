# Spec roadmap

The order in which features are specified and built. Each line becomes a
`specs/NNN-name/` folder when its spec is started; until then it is a plan
and may be reordered. Notes under "Planned specs" record the intent and the
open questions each spec must settle.

| # | Spec | Status | Summary |
|---|------|--------|---------|
| 001 | core-editor | done (merged) | Lossless `.qmd` model, `qd` CLI and skill, live preview, browser editor with form panel, drag/resize boxes, safe concurrent edits |
| 002 | offline-export | done (merged, PR #2) | `qd check --offline`, `qd export` (one self-contained HTML), `qd pdf`, editor Export |
| 003 | on-slide-editing | done (P1 merged #4; P2 #5; polish) | Edit on the slide itself: type into titles/text with a full formatting toolbar, in-place source editor for math/code, property toolbars, add/delete/reorder elements, right-click context menus; the form panel becomes a hidden Source view |
| 004 | source-and-run | done (PRs #6–#12; polish) | The `.qmd` as an editable source pane beside the slides, kept in sync both ways, with **Run** buttons for code cells and Run all — one place to type markdown, edit slides and run code |
| 005 | shared-focus | done (PRs #15, #16; polish) | The editor reports what the person is looking at and has selected (`qd focus`), so "make this slide…" in chat refers to the right slide/element; agents can point the editor at a slide or element; also the cursor↔slide link that 004 and 006 need |
| 006 | vscode | done (PRs #18 and the US4 + polish PR) | Run the interactive editor inside VS Code next to the `.qmd`, alongside the Quarto extension, so text editing, on-slide editing, running code and reveal.js preview all work together there |
| 007 | app-shell | planned | Desktop-app conventions: menu bar (File, Edit, Insert, Slide, View, Run, Help) with every action and shortcut; Settings window; shortcut reference; consistent selection, clipboard, undo, Escape/Enter |
| 008 | layouts-and-placement | planned | **Slide layouts** (title, title + content, two columns, image + text, section, big number, blank, and the person's own saved ones) to start a slide from, then drag **any** element anywhere on it, resize it, or put it back in the flow; all stored as plain Quarto (columns, absolute positioning) |
| 009 | export-formats | planned | An **Export ▾** drop-down (and `qd export --to <format>`) for every format Quarto/Pandoc can write: PowerPoint, PDF (Beamer/LaTeX or Typst), Word, HTML, Markdown, Jupyter notebook, and more |
| 010 | history | planned | Optional **save = git commit** (a setting), a **History** side panel to scroll back through versions with a side-by-side preview of the deck as it was, diff any version against now (source and rendered slides), and restore a whole version or just one slide/element |

Every PR — spec or small — passes a concision audit (`/pr-audit`,
constitution 1.1.0) before it is marked ready.

Small editor improvements that don't need a spec (e.g. the side-scrolling
slide strip and hover preview, PR #3) go straight to a PR.

## Facts these plans rely on (checked 2026-11-04)

- **Quarto → reveal.js is a Pandoc pipeline.** `quarto render` first runs the
  code cells (Jupyter for Python/Julia, knitr for R, or none), producing
  plain markdown, then calls its bundled Pandoc (3.10 in Quarto 1.11.9, run
  standalone as `quarto pandoc`) with Quarto's own Lua filters and templates
  to write reveal.js HTML.
- That Pandoc writes ~70 output formats (`quarto pandoc
  --list-output-formats`), including `pptx`, `beamer`, `latex`/`pdf`,
  `typst`, `docx`, `odt`, `html`, `gfm`/`commonmark`, `ipynb`, `epub`,
  `jats`, `rst`, `org`.
- This machine has VS Code 1.144 with the Quarto extension (`quarto.quarto`).

## Planned specs

### 004 source-and-run

Goal: one window where the person can type the Quarto source, edit the
slides directly, and run the code — all on the same file, all live.

- A source pane (toggle or split with the slides) showing the whole `.qmd`
  with markdown/YAML/code highlighting; typing there saves through qd (same
  conflict rules as everything else) and the slides re-render; clicking or
  editing on a slide scrolls the source to that block, and moving the cursor
  in the source shows that slide.
- A **Run** button on each code cell and **Run all**; outputs (text, tables,
  figures, errors) appear under the cell in the source pane and in the
  slides. Agents get the same through the CLI (`qd run <cell>`, `qd run
  --all`), with JSON results.
- Open questions for the spec: run cells one at a time in a live kernel
  (fast, interactive, state carries between cells — like notebooks) versus
  re-rendering the deck with execution on (simple, always matches the final
  render, slower); how outputs persist between sessions (Quarto `freeze` /
  cache vs. qd's own cache) without ever writing outputs into the `.qmd`;
  which engines (Jupyter/Python and Julia, knitr/R).
- Safety: running code executes arbitrary programs on the machine, so Run is
  only ever an explicit action by the person or a CLI call; the browser API
  keeps its loopback/origin checks and never runs code because a page asked.
- Build choice to settle in the plan: a proper code editor in the browser
  (e.g. a vendored CodeMirror build) versus a plain textarea, given the
  no-build-step editor.

### 005 shared-focus

- `qd focus` returns the slide, element and text selection the person has in
  the editor (and in VS Code, via 006); `qd show-slide <ref>` / `qd focus
  --set` moves the editor there.
- The same two-way link drives 004's source↔slide sync and 006's
  editor↔VS Code cursor sync.

### 006 vscode

Goal: everything above, inside VS Code, working with the Quarto extension
rather than replacing it.

- A small VS Code extension (personal-use `.vsix`, same licence) with a
  command "Open in quartz-deck" that starts `qd serve` for the active `.qmd`
  and shows the interactive editor in a panel beside the text editor (a
  webview hosting the local editor page).
- The text editor and the panel edit the same file: on-slide edits appear in
  the VS Code buffer, typing in VS Code re-renders the panel; cursor ↔ slide
  follow each other (005).
- Running code uses the Quarto extension's own Run Cell / Jupyter where it is
  installed, or 004's Run when not; render/preview/export commands map to
  `qd` commands in the VS Code command palette.
- Key design question for the spec: VS Code holds unsaved changes in its
  buffer while qd writes the file on disk. The extension must either route
  panel edits into the buffer as VS Code edits, or save/lock around them, so
  neither side overwrites the other (constitution V).
- Also: keyboard focus between panel and editor, theme matching, and
  working over Remote-SSH (the panel reaches `qd serve` through VS Code's
  port forwarding).

### 007 app-shell

As before: menu bar, Settings window, shortcut reference, consistent
paradigms. The Run menu (004) and the Export ▾ menu (009) live here.

### 008 layouts-and-placement

Goal: work like a slide app. Start a slide from a layout, then place
anything anywhere on it, while the `.qmd` stays plain Quarto.

**Layouts (templates for a new slide):**

- Insert ▸ Slide (and the slide list's +) offers a gallery of layouts with a
  small preview of each: title only, title + content, two columns, three
  columns, image + text (left or right), image full-bleed, section header,
  big number / quote, title + notes, blank.
- A layout is plain Quarto markdown: a heading, `:::: {.columns}` with
  `::: {.column}`s, or `.absolute` boxes, filled with placeholder text and
  images ("Click to add text", "Drop an image") that are ordinary elements.
  Nothing about the layout is stored separately: once added, the slide is just
  markdown, and every piece can be edited, moved, resized or placed freely.
- **The person's own layouts:** "Save slide as layout…" keeps a slide's
  structure (placeholders in place of its content) for reuse. Where they
  live is an open question: per user (qd's config folder, like 007's
  settings, so every deck has them) or with the project (a folder beside the
  deck, so collaborators and agents share them), or both.
- CLI for agents: `qd layouts` lists them (built-in and saved, with a
  description of each placeholder); `qd add --layout two-columns [--after
  <slide>] [--title …]` adds a slide from one and returns the placeholder
  refs, so an agent can fill them with `qd edit`.
- Open questions: whether layouts follow the deck's theme and slide size;
  applying a layout to an existing slide (re-flowing its content into the new
  structure) vs. only for new slides; layouts that need CSS beyond Quarto's
  built-ins (kept out, or shipped as a small `.scss` the deck opts into).

**Free placement:**

- Today (001 US4) only positioned boxes (Insert ▸ Text box, image boxes) can
  be dragged anywhere and resized; other elements (003) only move up or down
  within the slide's flow.
- **Take an element out of the flow:** ⌥-drag a paragraph, list, image,
  equation or cell output (a plain drag still reorders, as in 003), or choose
  "Place freely" (menu, right-click, `qd block place <ref> --at X,Y
  [--size W,H]`). The element is wrapped in Quarto's `::: {.absolute top=…
  left=… width=…}` div, the same syntax as today's boxes, so stock `quarto
  render` puts it in the same spot. Its own bytes are unchanged.
- **Move and resize** it like any box: mouse drag, resize handles, arrow keys
  to nudge (⇧ for bigger steps, matching 007's keyboard rules), and the
  existing `qd box move`/`resize`.
- **Back into the flow:** "Put back in the flow" (`qd block unplace <ref>`)
  removes the wrapper and leaves the element where its source is.
- Optional help while dragging: alignment guides to the slide's edges,
  centre and other elements; snapping that can be turned off (a 007
  setting).
- Open questions: units (pixels of the slide's width × height vs. percent,
  and what happens when the deck's size changes); elements that can't be
  wrapped safely (headings, columns, a code cell's input vs. its output, an
  element already inside a div); stacking order when boxes overlap (a "Bring
  to front / Send to back" that reorders the source); how a placed element
  shows in the source pane and in VS Code's text (005/006 follow); and
  whether an element placed freely still appears on click (fragments).

### 009 export-formats

- **Export ▾** drop-down in the editor (and File → Export in 007) listing the
  formats below; `qd export --to <format> [--out]` for agents, JSON result.
- Default route: `quarto render --to <format>` on an export copy (as in 002),
  so code still runs and Quarto features (callouts, cross-references,
  layouts, themes) are kept. Formats Quarto has no profile for go through
  `quarto pandoc` on the rendered markdown instead, with a note that
  Quarto-specific features may be simplified.
- Slide formats first: offline reveal.js (002), PDF of the slides (002),
  PowerPoint (`pptx`), Beamer PDF. Then documents: Word, HTML page, PDF via
  LaTeX or Typst, Markdown (GFM/CommonMark), Jupyter notebook, ODT, EPUB.
- The spec must say per format what carries over (positioned boxes,
  fragments, speaker notes, backgrounds, interactive figures) and report
  losses in the result rather than silently dropping content; PDF/LaTeX
  formats need a TeX or Typst install, checked and reported like the browser
  in 002.

### 010 history

Goal: never lose work, and be able to look back — built on git, visible in
the editor.

- **Setting (007): "Save as git commit".** When on and the deck is in a git
  repository, every save (on-slide edit, source edit, CLI edit) also makes a
  commit of the `.qmd` only, with a message describing the change (`qd: edit
  results/1`, `qd: title methods`, `person`/`agent` noted). Off by default;
  never commits other files, never pushes. Edits made in a burst (typing
  sessions, a run of agent commands) may be squashed into one commit after a
  pause, so history stays readable.
- **History panel** beside the slides: a timeline of commits that touched the
  deck (qd's and the person's own), each with time, author and message.
  Selecting one shows a **side preview** of the deck as it was (rendered,
  navigable, next to the current deck), with changed slides marked.
- **Diff** any version against the current file or another version: source
  diff per slide, and the rendered slides side by side.
- **Restore**: the whole deck, one slide, or one element from a chosen
  version — as a normal qd edit (undoable, conflict-checked, and itself
  committed when the setting is on). Never rewrites git history.
- CLI for agents: `qd history [--slide <ref>]`, `qd diff <rev> [<rev>]`,
  `qd restore <rev> [--slide <ref> | --element <ref>]`, JSON output.
- Open questions: how commits from qd interact with the person's own
  workflow (staged changes, other files, branches, hooks, signing); what to
  do outside a git repository (offer `git init`, or keep a private history
  in qd's cache); rendering old versions (render on demand into the cache,
  with the code-execution rules of 004).
