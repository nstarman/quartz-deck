# Quickstart run: 015 diagrams (T028)

**Date**: 2026-11-16
**Build**: branch `015-polish` (debug)
**Setup**: a scratch deck `talk.qmd` with one empty slide `## D {#d}`, in a
scratch folder; qd's config and cache in scratch folders; Quarto 1.11.9.
The picture is the made-up `face.png` from `plugins/dot-image/`.

## §1 Commands (US1, US2) — pass, after two fixes

- [x] `new` inserted a pinned diagram with `data` and `tool`; `show` listed
  it with `editable: true` and `pinned: true`.
- [x] `add --label Quarto --at 4,0 --look tool --from tool` printed `n1` and
  added two lines, the box and `tool -> n1`. `move n1 --by 0.5,0`, `rename
  data "This talk"` and `rm 'tool->n1'` each changed one line.
- [x] `qd undo` put `tool -> n1` back.
- [x] `quarto render`: the boxes sat where pinned (0, 2 and 4.5 in) with
  straight horizontal arrows.
- **Fixed in the quickstart:** it named the starter's boxes `n1`, `n2`. They
  are `data` and `tool`.
- **Fixed in qd:** `add --from n1` on a diagram without `n1` named the new
  box `n1` and wrote a loop `n1 -> n1`. `add` now checks `--from` and
  `--to` against the boxes already there and fails with `E_NOT_FOUND`
  (test in `add_placed_styled_and_connected`).

## §2 Editing on the slide (US1, US2) — pass, by the browser tests

The Claude browser pane was hidden for this run, so the gestures were
checked by the headless browser tests, which drive real mouse and key
input against `qd serve`:
- [x] drag a box: straight lines while dragging, only its `pos` changes,
  Escape cancels (`diagram_drag_moves_a_box`); arrow keys nudge
  (`diagram_arrow_keys_nudge`);
- [x] drag from a handle onto a box, or into empty space for a new box
  (`diagram_connect_and_add`);
- [x] double-click to rename (`diagram_rename_in_place`);
- [x] select an arrow and delete it; reverse it from Properties
  (`diagram_delete_and_edge_properties`);
- [x] a box's look, link and size from Properties
  (`diagram_properties_look_link_size`);
- [x] Insert ▸ Diagram and Insert ▸ Box in diagram (`diagram_insert_menus`).

## §3 Pinning a laid-out diagram (US1) — pass, by the browser tests

- [x] dragging a box of an unpinned diagram offers "Pin layout", and pinning
  keeps every box where it was (`diagram_pin_prompt`).

## §4 Pictures in boxes (US3) — pass

- [x] `dot set d/0 tool image=face.png` wrote the spacer label, installed
  `_extensions/dot-image` and added `filters: [dot-image]`.
- [x] `quarto render`: the picture was drawn at x 147–175 pt inside the box
  (135–218 pt), left of the label.
- [x] `qd export`: the exported deck has the picture as a `data:` URI.

## §5 Live routing (US4) — pass, by the browser tests

- [x] with the setting on, dragging shows routed arrows at right angles that
  follow the pointer, and the drop saves the same text as with it off
  (`diagram_live_routing`); without the engine, straight lines and no error
  (`diagram_live_routing_without_engine`).
- [ ] Not watched by eye: the pane was hidden, and a hidden page runs no
  animation frames, which live routing uses.
