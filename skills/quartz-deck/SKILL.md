---
name: quartz-deck
description: Edit Quarto reveal.js presentations (.qmd) with the `qd` CLI — list, add, move, retitle and restyle slides, edit content, set front matter, undo. Use when asked to create or change slides in a .qmd deck, or when a repository contains a Quarto revealjs presentation.
---

# quartz-deck (`qd`)

`qd` edits a Quarto reveal.js deck **in place** with minimal text patches: only
the slide you target changes, everything else stays byte-identical. Every
command prints one JSON object on stdout; errors are JSON on stderr with a
stable `code` and a `hint`, and a non-zero exit. The `.qmd` file is the only
state — no hidden database.

Prefer `qd` over hand-editing the `.qmd`: it keeps ids stable, refuses unsafe
edits, detects concurrent changes and supports undo.

## The loop

1. **Inspect** — `qd slides` (ids, titles, one-line summaries, `version`),
   then `qd show <slide>` for the source and blocks of a slide you will edit.
2. **Edit** — one command per change. Pass `--expect <version>` from step 1
   so an edit fails instead of clobbering a change made meanwhile. Add
   `--dry-run` to see the diff first. When the person may be typing on the
   slides in the editor at the same time, prefer `qd edit <slide>/<i> --md …
   --expect-text <hash>` (each block's `hash` is in `qd show`; `qd title`
   takes the hash of the current title): it refuses only if *that* element
   changed, so edits elsewhere in the deck don't block you.
3. **Check** — `qd check` (duplicate ids, unsafe constructs, missing files);
   `qd check --render` also runs Quarto and reports render errors.
4. **Look** — start the live preview once with `qd serve --detach` (prints
   `{"url","port","pid"}`), then open `qd url <slide>` in your browser tool
   (the link carries the token the page needs to run code; share it only with
   the person you work with).
   The page re-renders by itself after every edit (about 1–2 s) and stays on
   the current slide; a red banner shows Quarto errors while the last good
   render stays visible. Stop it with `qd stop`. The person you work with may
   edit in that page at the same time (it runs the same `qd` commands), so
   always pass `--expect` and re-read on `E_CONFLICT`.
5. **"This", "here", "the selected …"** — when the person points at
   something in their editor, run `qd focus` first. It returns the slide,
   the selected `elements` (refs for any edit command) and the selected text
   (`selection.where` = `slide` or `source`). Act on it when `open` is true and
   something is selected — `age` counts from their last click, so a minute
   or two is normal while they type to you; if it is much older, say what
   you are acting on. Ask which one only when `open` is false or nothing is
   selected. If `version` ≠ `deck_version`, re-read (`qd show <slide>`)
   before editing.
6. **Undo** if needed — `qd undo` (and `qd redo`).

Pick the deck with `--deck path/to/talk.qmd` (or `QD_DECK`); it is implicit
when the current directory has exactly one `.qmd`.

## Addressing

- **Slide**: its id (`intro`, `s-3f2a`), `@N` (1-based position), `first`, `last`.
  Slides without an explicit `{#id}` have an auto id from their title; the
  first edit to a slide writes its id into the heading so it stays stable.
  The edit result's `target` is the id to use from then on.
- **Element** (block inside a slide): `<slide>/<index>` (0-based, as listed by
  `qd show`) or `#id` for blocks with an explicit id (e.g. boxes `#b-1a2b`).
  Inside columns the ref goes on with the place at each level: in
  `:::: {.columns}` at `results/1`, `results/1/0` is the first `.column` and
  `results/1/0/2` the third element in it (`qd show` lists them under
  `children`). `build set/list/order`, `block class` and `edit` take these
  refs; `block move/dup/rm/add/place` refuse them (edit the source instead).

## Commands

| Task | Command |
|---|---|
| list slides | `qd slides` |
| one slide | `qd show results` |
| front matter | `qd meta` |
| add slide | `qd add --after intro --title "Results" --md "- one"` |
| add untitled slide at end | `qd add --md "Just text."` |
| duplicate | `qd dup results` |
| paste slides (markdown, e.g. copied from another deck) | `qd paste --after intro --md -` (ids that clash are renewed) |
| new slide from a layout | `qd layouts` lists them (title-content, two-columns, image-text, …, with their placeholders); `qd add --layout two-columns --after intro --title "Before and after"` returns `placeholders` with a `ref` each: fill them with `qd edit <ref> --md "…"` (an image: `--md "![](images/x.png)"`); placeholders left empty render as nothing |
| place an element anywhere | `qd block place results/1 --at 600,400` (px of the slide; `--width W` or `--size W,H`) wraps it in an `.absolute` box, bytes unchanged; then `qd box move/resize` it; `qd block unplace <ref>` puts it back exactly. Divs (columns, callouts) and notes are refused: place what's inside |
| overlapping boxes | `qd box front <ref>` / `back` / `forward` / `backward`: boxes stack in source order (later on top), so this reorders the boxes only; it also changes the order fragments appear in |
| save a slide as a layout | `qd layout save results my-layout` (the deck's `_layouts/`; `--mine` for every deck; `--at REF=L,T,W,H` places a flow element; `--replace` overwrites); `qd layout rename A B`, `qd layout rm A`; then `qd add --layout my-layout`. Only the structure is kept, never the slide's text |
| reorder | `qd move results --before intro` / `--after last`; several at once keep their order: `qd move a b c --after last` |
| delete | `qd rm @4` (several: `qd rm a b`, one undo step) |
| groups (sub-slides) | a group is a `#` section and the `##` slides after it, as Quarto stacks them; `qd slides` shows `head` and `in`. `qd indent b` puts b in the group above (the slide above may become a `#` head), `qd outdent b` makes b head a group of its own; `qd move d --after a --indent` lands it in one step. Only heading marks change. A whole group moves by naming its slides: `qd move a a1 a2 --after b2` |
| navigation | `qd navigation vertical`: → goes from group to group and ↓ through a group's slides when presenting (`grid` keeps the depth; `linear`, Quarto's default, removes the option); `qd slides` shows `navigation_mode` |
| retitle | `qd title results "Findings"` (`""` = untitled) |
| replace content | `qd body results --md "New text"` (keeps positioned boxes) |
| change a few characters anywhere (code, YAML, text) | `qd replace --old 'np.mean(xs' --new 'np.median(xs' --expect <version>` — `--old` must match exactly once (`E_AMBIGUOUS`: add surrounding text); only those bytes change |
| edit one block | `qd edit results/0 --md "- three"` |
| slide attributes | `qd set results --attr background-color=aquamarine --class center` |
| remove attribute/class | `qd set results --unset background-color --unclass center` |
| front matter key | `qd meta set title '"My talk"'` / `qd meta unset subtitle` |
| reveal.js option | `qd meta set format.revealjs.theme dark` / `qd meta unset format.revealjs.slide-number` (under `format: revealjs:`, made when missing) |
| builds (appear on a click) | `qd build set results/1 --appear fade-up --leave fade-out` (`--appear none` makes it always shown), `qd build list results` (in the order they show), `qd build order results results/2 results/1` (every build named once; source order removes the indexes); inside columns, name the element: `qd build set results/1/1/0 --appear fade-up` |
| slide size | `qd meta size 1050 590 --keep-places` (`--keep-places` scales every placed box and arrow with the slide) |
| positioned text box | `qd box add results --text "Note" --at 700,80 --size 300,100` |
| positioned image | `qd box add results --image figures/plot.png --at 100,150 --size 600,400` |
| move / resize / delete box | `qd box move #b-1a2b --by 0,40`, `qd box resize #b-1a2b --size 400,300`, `qd box rm #b-1a2b` |
| speaker notes | `qd block add results --end --md $'::: notes\nSay this.\n:::'` (once per slide; then `qd edit` that block) |
| add / move / delete / duplicate one element | `qd block add results --after results/0 --md "- point"` (or `--before`, `--start`, `--end`), `qd block move results/2 --before results/0`, `qd block rm results/1`, `qd block dup results/1`, `qd block class results/1 --add fragment` (appears on click) — only that element's bytes move; the heading is untouched |
| images beside the deck | `qd images` lists them (newest first); `qd image add chart.png --slide results --end` copies the file into `images/` under a free name and inserts it (`--after`/`--before` an element; without `--slide` it only copies) |
| validate | `qd check` (add `--render` to run Quarto) |
| final output | `qd render` (→ `output` path; `--to pdf` etc. for other formats) |
| Python code cells | each deck has its own locked environment: `qd env init` (uv; `--tool pixi` for pixi with PyPI packages), `qd env add numpy matplotlib`, `qd env` (where, `lock_current`), `qd env sync` (install the lock on a new machine / relock after a hand edit). The manifest and lockfile are part of the deck: commit them. Never `pip install`; `qd render` runs the cells inside the environment |
| run code / check a result | `qd run results/1` (JSON outputs; figures as files: `outputs[].path`), `qd run --all` (stops at the first error), `--fresh` to restart and run from the top, `--timeout 60`. With `qd serve` running it uses the editor's live session and the person sees the outputs; otherwise a fresh one-shot session. Fix a failing cell with `qd replace --old … --new …`, then run it again. Cells run with the deck's engine as Quarto picks it (`engine` in the result: `jupyter` with its `kernel`, `knitr` for R, `julia`); `{ojs}` cells run only in the editor's page |
| stored outputs | `qd outputs` (or `qd outputs results/1`): last outputs per cell, `stale: true` once the cell changed |
| works without network? | `qd check --offline` (headless Chrome, network blocked; problems per slide with hints) |
| live preview | `qd serve --detach`, then `qd url results`; `qd stop` when done |
| what the editor can do, and its keys (to tell the person) | `qd actions` → `[{id, label, menu, key, qd}]`; `qd settings` for their preferences; change one only when they ask (`qd settings set theme dark`, `qd settings reset`) |
| what the person is looking at / has selected | `qd focus` → `slide`, `elements`, `selection.text`, `age` (`{"open": false}` without an editor) |
| show the person something | `qd focus --set results/1` → their editor goes there and flashes it; `deferred` means they were typing at the editor's last report, so it is offered to them instead; `no_editor` without one |
| undo / redo | `qd undo` / `qd redo` |
| the deck's versions (git) | `qd history` (`--slide REF` or `--element REF` for the versions that changed it; `--limit`, `--skip`); `uncommitted` comes first when the file differs from the last commit |
| what changed | `qd diff HEAD~3` (against the file) or `qd diff REV REV` → `slides: [{kind: added|removed|moved|changed, id, title, from, to, diff}]` |
| put something back | `qd restore REV --slide REF` (or `--element REF`, or nothing for the whole deck); REF is as named in that version; an edit like any other (`--expect`, `--dry-run`, `qd undo`) |
| new deck | `qd new talk.qmd --title "My talk"` |
| all commands + output shapes | `qd schema` |

Multi-line markdown: pass real newlines, or use `--md -` with stdin, or
`--file body.md`.

## Offline export (talks without Wi-Fi, archives)

A rendered Quarto deck usually still needs the internet (MathJax from a CDN on
every deck, KaTeX never embedded, Plotly's own web scripts, background videos
left as separate files). On a connected machine it *looks* fine, so check it:

1. `qd check --offline` — renders the deck and opens it in headless Chrome with
   the network blocked; each problem has a `code`, the `slide`, and a `hint`.
2. `qd export` — writes `export/<deck>.html`, one self-contained file: apps
   (`<iframe data-app="apps/x.html">` — use `data-app`, not `src`, for apps —
   and `background-iframe`) go inside it gzipped, `<object>` and SVG `<image>`
   files as `data:` URIs, a repeated image once; only background videos ship
   beside it. The offline setup is applied to a copy (your `.qmd` and
   `_quarto.yml` are never changed) and the check runs on the result.
   `--pdf` adds `export/<deck>.pdf` (one page per fragment step;
   `--per-slide`; `--notes` writes `<deck>-notes.pdf`). `--strip-notes` makes
   a copy to share; `--out DIR` picks the folder (e.g. `_output`).
3. `qd pdf` — re-print the latest export (`--notes`, `--per-slide`); with
   ghostscript installed the PDF's images are recompressed (`compressed`).

The check report also has `size.largest` (where the file's megabytes go,
apps named by path). A deck needs no build scripts of its own: its Makefile
calls `qd export` and `qd pdf`.

If the check fails, the export is **not** replaced (exit 1, `E_OFFLINE`, the
report is still on stdout with `"ok": false`). Fix the reported problem and
export again:

| problem | fix |
|---|---|
| `P_NETWORK` | something loads from the web; `qd export` bundles MathJax/plotly.js, otherwise remove or download the resource |
| `P_MISSING_FILE` | a file the deck or an app loads (background video/iframe, parallax, `data-app`, `<object>`, SVG `<image>`) isn't at that path next to the deck: add it or fix the path (`qd set <slide> --attr background-video=...`) |
| `P_RAW_TEX` | math not typeset offline (KaTeX, or the web MathJax); `qd export` uses local MathJax |
| `P_PAGE_ERROR` | a script on the page failed (`{=html}` blocks, embedded apps) |
| `P_OFFLINE_INCOMPATIBLE` | multiplex (needs a server); chalkboard cannot be one file (`E_OFFLINE_INCOMPATIBLE` from `qd export`) |
| `P_OVERFLOW` (warning) | an element sticks out of the slide (all fragments shown); move or shrink it, or give it class `overflow-ok` if that is on purpose |
| `P_NOTES_VISIBLE` (warning) | `show-notes` in the YAML shows notes to the audience: `qd meta unset show-notes` if it's top-level |

The first export downloads MathJax 3.2.2 (and Chrome for Testing if no
Chrome/Chromium is installed; `qd browser` shows which is used), verified by
SHA-256; later exports work offline. `--no-download` forbids downloads.

## Other formats (PowerPoint, Beamer, Word, Markdown, …)

`qd export --to <format>` writes `export/<deck>.<ext>` (`--out FILE` for
another path); `qd export --list` lists every format with `ready` and, if not,
a `hint`. Menu formats: `pptx`, `beamer`, `docx`, `html`, `pdf` (LaTeX),
`typst`, `gfm`, `commonmark`, `ipynb`, `odt`, `epub`, and any other writer
Quarto's Pandoc has (`rst`, `org`, `latex`, `asciidoc`, …). Each is one
Quarto render of a copy, so code runs and Quarto's features are kept; the
deck is never changed.

Read `losses` in the result before telling the person an export is done:
each is `{feature, keep, what, slides}`, e.g. `placed` boxes in the `flow`,
`pause`/`fragment` `all-shown`, `notes` `hidden` in Beamer,
`background-image` or `interactive` `lost`. Only features the deck uses are
listed. `qd --dry-run export --to X` reports them without rendering, which
is the way to pick a format. Documents get one section per slide with notes
under "Speaker notes"; text formats (Markdown, the HTML page, rst, LaTeX, …)
put figures and libraries in `export/<deck>_files/` (listed in `files`).

`E_NO_TEX` (Beamer, `pdf`): install TeX with `quarto install tinytex`, or use
`typst`, which needs nothing extra.

## History (git)

`qd history`, `qd diff` and `qd restore` read the person's own git
repository, so they see the person's commits as well as qd's. REV is
anything git accepts (`a1b2c3d`, `HEAD~2`, a tag). To find "the slide as it
was before the agent changed it", run `qd history --slide REF`, then `qd diff`
against a version, then `qd restore`. Restore never rewrites history.

**Save as git commit** is the person's setting (`history.commit`, off by
default; leave it alone unless they ask). With it on, an edit made while no
`qd serve` is running commits the deck at once, and the edit's result has
`"commit": {"rev"}` or `{"error"}`. A commit error doesn't undo the edit:
tell the person what git said (a hook, signing). With a server running, it
commits after `history.pause` seconds. Outside a repository, `qd history
--init` starts one, but only when the person asks for it.

## References

A deck cites with Pandoc's own syntax (`@key`, `[@key]`), and its
references live in a companion file named by `bibliography:` (BibTeX or
CSL-JSON), or embedded in the front matter as `references:`.
- `qd bib`: where they live, how many, and `missing`: keys cited on slides
  with no entry.
- `qd bib pull FILE`: merge a `.bib` or CSL-JSON file (any manager's export).
  The first pull creates `_talk/references.bib` (or `references.bib` beside
  the deck). A paper already in the deck keeps its key; the file's fields
  win; nothing is deleted (`gone` lists entries the file lacks).
- `qd bib pull --from zotero KEY`: the same, from a Zotero collection (KEY:
  its 8-character key), through the source plugin `qd-zotero` (Zotero 7
  running, with the local API allowed). `E_SOURCE`: the plugin is missing,
  or it failed; its message says why.
- `qd bib embed` / `qd bib extract [--to FILE] [--format bib|json]`: move
  them into the front matter and back. `qd bib convert --format bib|json`:
  the companion file in the other format.
- `qd bib install`: the citation extensions into `_extensions/`, named in
  `filters:`. Every `qd bib` write does this too. With `cite-links`, each
  citation links to its paper with an icon ("Name et al. (2021)", from its
  `talk.csl`), and `[[@key]]{.corner}` is the icon alone, top right.
  With `refs-slide`, the References slide (`::: {#refs}` then `:::`) takes
  `refs-style="compact|short|full"`, `refs-columns="1|2|3"` and
  `refs-size="normal|smaller|smallest"` (defaults: compact, 3, smaller).

Each write is one edit of the deck and its companion file (`--dry-run`,
`--expect`, one `qd undo` reverts both). A bibliography file outside the
deck's folder is only read: writing it gives `E_READ_ONLY`.

A deck still citing with the old `{{< paper >}}` shortcode: `qd bib migrate`
converts it to `@key` citations, with references from `--from zotero KEY`
(or the Makefile's old `qd bib --zotero KEY` line), else built from the
shortcodes. It renders the deck before and after and compares each slide's
links and their text. Any difference puts every file back and fails with
`E_MIGRATE_DIFF`, naming a report: show it to the person, and run again with
`--accept` only when they say so. Outside git it needs `--backup DIR`.

## Diagrams

A diagram is a `{dot}` (Graphviz) cell. `qd dot show CELL` lists its boxes
(name, label, look, link, image, position in inches, size), its arrows
(`A->B`, with `#2` for a repeat), its graph settings, and whether it can
be edited. `pinned` is true when the cell uses `layout=neato` and every box
has `pos="x,y!"`, so boxes stay where they're placed. DOT that can't be
read safely is reported in `editable` with the line, and stays editable
as text. So is a `//| file: x.dot` diagram, which lives in another file.

`qd dot at --text BODY --at OFFSET` says what is at a byte offset of a
cell's body (the text between its fences, `-` for stdin), saved or not: a
box (`node`), an arrow (`edge`, the link under the cursor in a chain, and
every link in `edges`), a cluster, a setting or defaults, or `null`. Text
that can't be read yet gives `stopped` with a line and column. For a box or
an arrow, `alone` is a small DOT graph drawing just it, with the cell's
defaults. It changes nothing and needs no deck: use it to answer "which box
is on line 4".

`qd dot pin CELL` pins a diagram where Graphviz put it: it adds
`layout=neato` and a `pos` for every box, read from the preview (or from
`--positions '{"a":[0,1],...}'` in inches). It adds `splines=ortho` (right
angles) only when the diagram has no `splines` and no arrow uses a port. A
diagram with a `subgraph` is refused (`E_UNSAFE_EDIT`): `neato` doesn't draw
clusters. Then
`qd dot move CELL NODE --at X,Y` or `--by DX,DY` (inches, y up) places one
box; positions snap to 0.25 in unless `--no-snap`. Arrows re-route on the
next render. A diagram that isn't pinned refuses `move` with the hint to pin.

Building a diagram:
- `qd dot import FILE.svg --on SLIDE` (or `--after`/`--before REF`, or
  `--replace REF`) turns a drawing into a pinned diagram: a hand-drawn SVG's
  rectangles, circles and ellipses with text become boxes, its lines between
  two boxes become arrows; an SVG Graphviz drew keeps its names and arrows.
  Check `left_out` (with reasons) and `notes` in its output and tell the
  person what didn't come across; then refine with the commands below.
- `qd dot new --on SLIDE` (or `--after REF`, `--before REF`) inserts a
  pinned starter diagram: two boxes, `data` and `tool`, one in each starter
  look, joined.
- `qd dot add CELL [--label L] [--at X,Y] [--look tool|artifact | --look-of NODE] [--from A] [--to B]`
  adds a box named `n1`, `n2`, … and prints its name. In a pinned diagram
  with no `--at`, it goes right of the rightmost box.
- `qd dot connect CELL A B` adds an arrow and prints it (`A->B#k`).
- `qd dot rename CELL NODE LABEL` changes only the label.
- `qd dot set CELL TARGET KEY=VALUE…` sets attributes of a box or an arrow
  (`A->B`, `A->B#2`). `KEY=` removes one. For a box, `--look` or
  `--look-of` sets its look and removes look keys the new one doesn't have.
- `qd dot rm CELL TARGET` removes a box with every arrow touching it, or
  one arrow. A chain `a -> b -> c` is split, keeping its attributes.
- `qd dot flip CELL A->B` reverses an arrow (not in an undirected `graph`).

- `qd dot set CELL NODE image=face.png` gives a box a picture at its left.
  The label becomes a spacer label (an empty square cell, then the text),
  and the first picture installs the `dot-image` filter into the deck's
  `_extensions/` and names it in `filters:`, because Quarto's Graphviz
  doesn't draw images. `rename` keeps the spacer, and `image=` puts a plain
  label back. `qd dot show` reports the label's text, not the spacer.

In the editor the person does the same on the slide: drag boxes, drag from
a box's handles to connect, double-click to rename, and View ▸ Properties
panel for the details. Each gesture runs one of these commands. The setting
`diagrams.live-layout` (on by default; `qd settings set diagrams.live-layout false`
when they ask) routes arrows while they drag; it changes nothing that's saved.
With the cursor on a box's or an arrow's line of a diagram's source, the
editor outlines it on the slide and draws it alone beside the line;
`diagrams.source-preview` (on by default) turns the drawing off.
The details are in the editor's properties column on the right;
`ui.inspector` (on by default) hides it when they ask.

Looks are written on each box, so a box renders the same everywhere.
Boxes and arrows inside a `subgraph` are refused with `E_UNSAFE_EDIT`:
edit those as text.

## Arrows

An arrow on a slide is one line of Emil Hvitfeldt's quarto-arrows shortcode,
`{{< arrow from="x,y" to="x,y" position="absolute" … >}}`, placed in the
slide's own pixels (on a 1600×900 deck, `0,0` is the top-left corner and
`1600,900` the bottom-right). `qd arrow list SLIDE` and `qd arrow show REF`
read them; an arrow without `position="absolute"` (in the text's flow), or
one that can't be read safely, is reported in `editable` with the reason
and stays editable as text.

- `qd arrow add SLIDE --from X,Y --to X,Y [--color C] [--width N] [--dash true|A,B] [--head H] [--heads end|start|both|none] [--label T] [--label-position start|middle|end] [--fragment]`
  adds one after the slide's content, before its notes, and prints its ref.
  The first arrow in a deck downloads quarto-arrows (pinned by checksum)
  into qd's cache and installs it into the deck's `_extensions/`; offline
  with nothing cached it fails with `E_DOWNLOAD` and changes nothing.
- `qd arrow move REF --from X,Y` / `--to X,Y` / `--by DX,DY` moves an end,
  or the whole arrow; only what moves is rewritten.
- `qd arrow bend REF --through X,Y` curves it through the point (writing
  `control1`/`control2`); `--straight` removes the curve.
- `qd arrow set REF KEY=VALUE…` sets colour, width, dash, line, head,
  head-start, head-end, label, label-position, fragment and the like;
  `KEY=` or the default value removes the option.
- `qd arrow rm REF` removes it.

Arrows are free: moving an element later doesn't move them. In the editor
the person draws them with Insert ▸ Arrow, drags their ends (snapping to
elements), bends them by the middle handle, and labels them by
double-click.

## Errors and what to do

| code | meaning | recover |
|---|---|---|
| `E_NOT_FOUND` | ref matches nothing | `qd slides` / `qd show`, use a listed id |
| `E_AMBIGUOUS` | duplicate ids | address by `@N`; fix ids (see `qd check`) |
| `E_CONFLICT` | file changed since you read it | re-run `qd slides`, retry with new `--expect` |
| `E_UNSAFE_EDIT` | slide has an unterminated fence/div or a heading inside a div | fix the source (see `qd check`), then retry |
| `E_INVALID_ARG` | bad value (attr name, YAML, multi-line title) | see `qd schema` |
| `E_USAGE` | wrong flags | `qd schema` or `qd <cmd> --help` |
| `E_NO_UNDO` | nothing to undo, or file changed since | edit forward instead |
| `E_NO_DECK` | no/several decks | pass `--deck PATH` |
| `E_NO_SERVER` | no preview running | `qd serve --detach` |
| `E_NO_QUARTO` | Quarto missing (preview/render only) | install Quarto; edits still work |
| `E_OFFLINE` | the deck would not work offline | read `problems` in the stdout report; fix; re-run |
| `E_OFFLINE_INCOMPATIBLE` | chalkboard/multiplex can't be one offline file | remove the feature for the export |
| `E_NO_TEX` | `qd export --to beamer`/`pdf` without TeX | `quarto install tinytex`, or `--to typst` |
| `E_MIGRATE_DIFF` | `qd bib migrate` found differences for the audience; everything was put back | show the person the report; `--accept` only on their word |
| `E_SOURCE` | `qd bib pull --from NAME`: `qd-NAME` is not on `PATH`, failed, or printed no reference list | install it (`cargo install --git https://github.com/nstarman/quartz-deck qd-NAME`), or fix what its message says (for Zotero: open it) |
| `E_READ_ONLY` | a `qd bib` write would change a bibliography file outside the deck's folder | copy it into the folder, or `qd bib pull PATH` from it |
| `E_NO_ENV` | deck has code cells but no environment | `qd env init`, then `qd env add <pkgs>` |
| `E_LOCK_STALE` | manifest edited, lock out of date | `qd env sync` |
| `E_ENV_TOOL` / `E_ENV` | uv/pixi missing, or it failed | install per the hint / read `details` |
| `E_RUN` | a cell raised or was stopped (`qd run`) | read the error output in the stdout result; fix the cell; run again |
| `E_NOT_CODE` / `E_ENGINE` | not an executable cell / the deck's engine can't run it here: the message names what is missing (a Jupyter kernel, with the ones installed; R, or its knitr/rmarkdown packages; Julia), or the cell's language isn't the deck's (an `{r}` cell in a Jupyter deck), or it's an `{ojs}` cell | tell the person what to install; `{ojs}` cells run in the editor; `qd render` still runs the whole deck |
| `E_NO_REPO` | the deck is not in a git repository | `qd history --init`, if the person wants a history |
| `E_NO_GIT` / `E_GIT` | git missing, or git refused (its message) | install git / read the message |
| `E_REPO_BUSY` | a merge, rebase, cherry-pick or bisect, or a detached HEAD, so no commit is made | only in `commit.error`; the edit is kept and committed after the person finishes |
| `E_NO_BROWSER` / `E_DOWNLOAD` | no Chrome, or a download failed its checksum | install Chrome, or allow the download; retry |

## Worked examples

**Add a results slide after the intro, then move the conclusion to the end**

```bash
qd slides                                   # note "version"
qd add --after intro --title "Results" --md $'- Accuracy 0.91\n- Recall 0.87' --expect <version>
qd move conclusion --after last
qd check
```

**Retitle slide 3 and give it a dark background**

```bash
qd show @3                                  # check it is the right one
qd title @3 "Key findings"                  # result.target is now its stable id
qd set <target> --attr background-color="#1a1a2e" --class center
```

**Fix a failing code cell** — the run result names the cell, shows the
traceback (with the failing line) and gives the deck `version`:

```bash
qd run --all                                # E_RUN; stdout: runs[-1] is the failing cell, its outputs the traceback
qd replace --old 'df.colum' --new 'df.columns' --expect <version from that result>
qd run --all                                # "ok": true; figures in outputs[].path
```

**Positioned image** — boxes are Quarto `.absolute` blocks, in px on the
slide (1050×700 unless the deck sets `width`/`height`; `qd slides` reports it):

```bash
qd add --title "Architecture"               # target: "architecture"
qd box add architecture --image figures/arch.png --at 100,120 --size 850,500
qd show architecture                        # block kind "box", geom {left, top, width, height}
qd box move #b-xxxx --by 0,-20              # use the id from the add result's target
```

## Notes

- **What to write** (reveal.js features, Quarto syntax, offline single-file
  decks, MathJax/Plotly, PDF) is the `quarto-revealjs` skill's job; this skill
  is **how to edit** a deck. Deck folders set up that way (a `_quarto.yml`
  project) work with `qd serve` and `qd check --render` as they are.
- Link to a slide in deck content with `[text](#id)`, not `#/id`.
- `{visibility="hidden"}` slides are left out of the rendered deck; `qd slides`
  marks them `"hidden": true` and the preview still shows them, badged.
- Section slides are `#` headings (with the default `slide-level: 2`); a `---`
  rule starts an untitled slide. Content before the first heading is an
  untitled slide too.
- `qd meta set` edits top-level keys, and reveal.js options as
  `format.revealjs.KEY`. For other nested options, edit the front matter by
  hand (then `qd check`).
- Speaker notes are `::: {.notes}` blocks; incremental reveal is `. . .`.
- With the deck open in VS Code's quartz-deck panel, `qd` reads and edits the
  person's unsaved VS Code buffer, not the file: your edits show up in their
  editor as unsaved changes, and the `.qmd` on disk changes only when they
  save. Don't read the file directly then; use `qd show` / `qd slides`.
  (`qd render` alone renders the saved file; `qd export` and `qd check` use
  the buffer.)
