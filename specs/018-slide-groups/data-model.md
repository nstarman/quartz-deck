# Data model: Slide groups

Nothing new is stored in the `.qmd`. Groups are read from heading levels
(research R1).

## Deck (read)

- `group_level: Option<u8>`: the top section level, the smallest heading
  level among slides whose level is below `slide_level`; when there are
  none, `slide_level - 1`; `None` when `slide_level` is 1 (no groups).

## Slide (read)

Two derived fields, computed in one pass in deck order:

- `is_head: bool`: a heading slide at `group_level` (`Slide.head` is already the heading's byte span).
- `parent: Option<usize>`: the index of the nearest head above it, if any.
  Slides before the first head have none.

A slide with `is_head` has no `parent`. A head with no slide after it before
the next head is a group of one.

## Deck JSON (`qd slides`, the editor)

Each slide gains:

```json
{"head": true}                      // a group head
{"head": false, "in": "part-a"}     // a sub-slide of the head with id part-a
{"head": false}                     // a top-level slide
```

The deck gains `"group_level": 1` (or `null`) and `"navigation_mode":
"linear" | "vertical" | "grid"` (`linear` when the file has none).

## Collapse state (editor only)

`view.json` in the deck's qd cache folder: `{"collapsed": [head ids]}`,
read and written by the editor through `GET`/`POST /api/view` (research
R5). Never in the `.qmd`.

## Level change (patch)

For each slide whose level changes, one replacement inside its `head`
span: the ATX marks, the setext underline character, or the `---` line
(research R2). Everything else in the file is untouched.
