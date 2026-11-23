# Quickstart run: diagram source follows the cursor

Run on 2026-11-22 with the release build of spec 021 PR 3, on made-up decks
only.

## §1. `qd dot at` from the command line

With the quickstart's body (`a [label=A];` then `a -> b -> c;`):

| `--at` | Answer |
|---|---|
| 14 (on `a [label=A]`) | `node`, `a` |
| 30 (on `a -> b`) | `edge`, `a->b#1`, `edges` both links |
| 36 (on `b -> c`) | `edge`, `b->c#1`, `edges` both links |
| 0 (`digraph {`) | `null` |

An unclosed quote gives `stopped`: "a quoted string is never closed",
line 2, column 12. `--at 99` gives `E_INVALID_ARG`. All as expected.

## §2. In the editor

Covered by the browser tests, all passing:
- the outline on each box, link and chain, and nothing on a setting or
  outside the cell, with the selection unchanged (`source_follow_outlines`);
- hover, and giving the outline back (`source_follow_hover`);
- the drawing beside the line, redrawn as typed before the slide changes,
  dimmed while the text can't be read, and gone on Esc
  (`source_follow_preview`);
- the setting off (`source_follow_setting_off`);
- the cell editor over the slide (`source_follow_cell_editor`).

Seen by eye in the browser pane on a demo deck: with the caret on `tool`'s
line, the Tool box has the dashed outline. The pane was hidden for the
drawing check, which pauses its frame callbacks, so the drawing was
checked in the headless tests only.

## §3. SC-002 timing (20 boxes, 19 arrows)

From a caret stop to the outline, for eight boxes in turn: 74, 74, 72, 70,
69, 74, 73, 74 ms. The drawing appeared at the same moment, except the
first one (96 ms, while Graphviz loaded). This includes the 60 ms wait for
the caret to settle. Under 100 ms: **pass**.

## SC-003: typing is no slower

200 characters typed in the source pane, each followed by a frame:
- inside the diagram cell: median 16.7 ms, 95th percentile 17.5 ms, max 17.9 ms, 1 `dot at` request (after typing stopped);
- in a paragraph outside it: median 16.7 ms, 95th percentile 17.6 ms, max 17.8 ms, no requests.

Each keystroke shows within one frame either way: **pass**. This was
measured with a throwaway test, not kept, because frame timing is too
noisy for CI under load.

## Left for the person

SC-005 (T020): trying it on a copy of their talk's diagram.
