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
   `--dry-run` to see the diff first.
3. **Check** — `qd check` (duplicate ids, unsafe constructs, missing files);
   `qd check --render` also runs Quarto and reports render errors.
4. **Look** — start the live preview once with `qd serve --detach` (prints
   `{"url","port","pid"}`), then open `qd url <slide>` in your browser tool.
   The page re-renders by itself after every edit (about 1–2 s) and stays on
   the current slide; a red banner shows Quarto errors while the last good
   render stays visible. Stop it with `qd stop`. The person you work with may
   edit in that page at the same time (it runs the same `qd` commands), so
   always pass `--expect` and re-read on `E_CONFLICT`.
5. **Undo** if needed — `qd undo` (and `qd redo`).

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
| edit one block | `qd edit results/0 --md "- three"` |
| slide attributes | `qd set results --attr background-color=aquamarine --class center` |
| remove attribute/class | `qd set results --unset background-color --unclass center` |
| front matter key | `qd meta set theme dark` / `qd meta unset theme` |
| validate | `qd check` (add `--render` to run Quarto) |
| live preview | `qd serve --detach`, then `qd url results`; `qd stop` when done |
| undo / redo | `qd undo` / `qd redo` |
| new deck | `qd new talk.qmd --title "My talk"` |
| all commands + output shapes | `qd schema` |

Multi-line markdown: pass real newlines, or use `--md -` with stdin, or
`--file body.md`.

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

**Positioned image** — boxes are Quarto `.absolute` blocks (px on a
1050×700 slide):

```bash
qd add --title "Architecture" --md '![](figures/arch.png){.absolute top=120 left=100 width=850 height=500}'
qd show last                                # the image block is kind "box"
```

## Notes

- Section slides are `#` headings (with the default `slide-level: 2`); a `---`
  rule starts an untitled slide. Content before the first heading is an
  untitled slide too.
- `qd meta set` edits top-level keys only. For nested options under
  `format: revealjs:`, edit the front matter by hand (then `qd check`).
- Speaker notes are `::: {.notes}` blocks; incremental reveal is `. . .`.
