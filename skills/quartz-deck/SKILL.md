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
   `age` is a few seconds; ask which one when `open` is false, nothing is
   selected, or the focus is old. If `version` ≠ `deck_version`, re-read
   (`qd show <slide>`) before editing.
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

## Commands

| Task | Command |
|---|---|
| list slides | `qd slides` |
| one slide | `qd show results` |
| front matter | `qd meta` |
| add slide | `qd add --after intro --title "Results" --md "- one"` |
| add untitled slide at end | `qd add --md "Just text."` |
| duplicate | `qd dup results` |
| reorder | `qd move results --before intro` / `--after last` |
| delete | `qd rm @4` |
| retitle | `qd title results "Findings"` (`""` = untitled) |
| replace content | `qd body results --md "New text"` (keeps positioned boxes) |
| change a few characters anywhere (code, YAML, text) | `qd replace --old 'np.mean(xs' --new 'np.median(xs' --expect <version>` — `--old` must match exactly once (`E_AMBIGUOUS`: add surrounding text); only those bytes change |
| edit one block | `qd edit results/0 --md "- three"` |
| slide attributes | `qd set results --attr background-color=aquamarine --class center` |
| remove attribute/class | `qd set results --unset background-color --unclass center` |
| front matter key | `qd meta set theme dark` / `qd meta unset theme` |
| positioned text box | `qd box add results --text "Note" --at 700,80 --size 300,100` |
| positioned image | `qd box add results --image figures/plot.png --at 100,150 --size 600,400` |
| move / resize / delete box | `qd box move #b-1a2b --by 0,40`, `qd box resize #b-1a2b --size 400,300`, `qd box rm #b-1a2b` |
| speaker notes | `qd block add results --end --md $'::: notes\nSay this.\n:::'` (once per slide; then `qd edit` that block) |
| add / move / delete / duplicate one element | `qd block add results --after results/0 --md "- point"` (or `--before`, `--start`, `--end`), `qd block move results/2 --before results/0`, `qd block rm results/1`, `qd block dup results/1`, `qd block class results/1 --add fragment` (appears on click) — only that element's bytes move; the heading is untouched |
| validate | `qd check` (add `--render` to run Quarto) |
| final output | `qd render` (→ `output` path; `--to pdf` etc. for other formats) |
| Python code cells | each deck has its own locked environment: `qd env init` (uv; `--tool pixi` for pixi with PyPI packages), `qd env add numpy matplotlib`, `qd env` (where, `lock_current`), `qd env sync` (install the lock on a new machine / relock after a hand edit). The manifest and lockfile are part of the deck: commit them. Never `pip install`; `qd render` runs the cells inside the environment |
| run code / check a result | `qd run results/1` (JSON outputs; figures as files: `outputs[].path`), `qd run --all` (stops at the first error), `--fresh` to restart and run from the top, `--timeout 60`. With `qd serve` running it uses the editor's live session and the person sees the outputs; otherwise a fresh one-shot session. Fix a failing cell with `qd replace --old … --new …`, then run it again |
| stored outputs | `qd outputs` (or `qd outputs results/1`): last outputs per cell, `stale: true` once the cell changed |
| works without network? | `qd check --offline` (headless Chrome, network blocked; problems per slide with hints) |
| live preview | `qd serve --detach`, then `qd url results`; `qd stop` when done |
| what the person is looking at / has selected | `qd focus` → `slide`, `elements`, `selection.text`, `age` (`{"open": false}` without an editor) |
| show the person something | `qd focus --set results/1` → their editor goes there and flashes it; `deferred` means they were typing at the editor's last report, so it is offered to them instead; `no_editor` without one |
| undo / redo | `qd undo` / `qd redo` |
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
2. `qd export` — writes `export/<deck>.html` (one self-contained file) plus any
   files that cannot be inlined (background video/iframe), with the offline
   setup applied to a copy (your `.qmd` and `_quarto.yml` are never changed),
   and runs the check on the result. `--pdf` adds `export/<deck>.pdf`
   (one page per fragment step; `--per-slide`, `--notes` for speaker notes).
3. `qd pdf` — re-print the latest export (`--notes`, `--per-slide`).

If the check fails, the export is **not** replaced (exit 1, `E_OFFLINE`, the
report is still on stdout with `"ok": false`). Fix the reported problem and
export again:

| problem | fix |
|---|---|
| `P_NETWORK` | something loads from the web; `qd export` bundles MathJax/plotly.js, otherwise remove or download the resource |
| `P_MISSING_FILE` | the background video/iframe/parallax file isn't at that path next to the deck: add it or fix the path (`qd set <slide> --attr background-video=...`) |
| `P_RAW_TEX` | math not typeset offline (KaTeX, or the web MathJax); `qd export` uses local MathJax |
| `P_PAGE_ERROR` | a script on the page failed (`{=html}` blocks, embedded apps) |
| `P_OFFLINE_INCOMPATIBLE` | multiplex (needs a server); chalkboard cannot be one file (`E_OFFLINE_INCOMPATIBLE` from `qd export`) |
| `P_NOTES_VISIBLE` (warning) | `show-notes` in the YAML shows notes to the audience: `qd meta unset show-notes` if it's top-level |

The first export downloads MathJax 3.2.2 (and Chrome for Testing if no
Chrome/Chromium is installed; `qd browser` shows which is used), verified by
SHA-256; later exports work offline. `--no-download` forbids downloads.

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
| `E_NO_ENV` | deck has code cells but no environment | `qd env init`, then `qd env add <pkgs>` |
| `E_LOCK_STALE` | manifest edited, lock out of date | `qd env sync` |
| `E_ENV_TOOL` / `E_ENV` | uv/pixi missing, or it failed | install per the hint / read `details` |
| `E_RUN` | a cell raised or was stopped (`qd run`) | read the error output in the stdout result; fix the cell; run again |
| `E_NOT_CODE` / `E_ENGINE` | not an executable cell / not Python | run only ```` ```{python} ```` cells; `qd render` still runs others |
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
- `qd meta set` edits top-level keys only. For nested options under
  `format: revealjs:`, edit the front matter by hand (then `qd check`).
- Speaker notes are `::: {.notes}` blocks; incremental reveal is `. . .`.
