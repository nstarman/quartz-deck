# Editor contract: slide groups (018)

All in the slide list (`ui/list.js`), plus menu entries in `actions.json`
and `actions.js`. Every change to the file is a `qd` op.

## The list

- A sub-slide's row is indented by one step (16 px); heads and top-level
  slides are at the left edge. The title slide stays first, not indented.
- A head with sub-slides has a disclosure button before its thumbnail,
  `aria-expanded` true or false, label "Collapse Part A" / "Expand Part A".
  A collapsed head shows the number of its sub-slides ("+3").
- Sub-slides of a collapsed head are in the list but `hidden`, so ↑/↓,
  the thumbnails' lazy drawing and the drag skip them.
- Numbers stay one running count; a hidden row keeps its number.
- When the shown slide is a hidden sub-slide, its head is expanded and the
  change saved (FR-008).

## Keys (focused row)

| key | does |
|---|---|
| ← | collapses an open head; on a sub-slide, moves focus to its head |
| → | expands a collapsed head |
| Tab / ⌘] | `qd indent` (disabled reason shown if not possible) |
| ⇧Tab / ⌘[ | `qd outdent` |
| Escape | focus back to the canvas |
| ↑ ↓ Enter ⌥↑ ⌥↓ Delete | as today, over visible rows |

## Menus

- Slide menu and the row's context menu: "Indent slide" and "Outdent
  slide", disabled with the reason as their tooltip when not possible.
- Slide menu ▸ "Presentation navigation" ▸ Linear, Vertical, Grid, the
  current one checked; runs `qd navigation`.
- On a collapsed head, "Copy slide", "Cut slide", "Duplicate slide" and
  "Delete slide…" read "Copy group (4 slides)" and so on.

## Drag

- Dragging a collapsed head drags the group: the ghost shows the head with
  "+N", and the gap that opens is one row.
- Horizontal offset beyond 16 px sets the landing depth (research R8); the
  drop marker is drawn at that depth. The drop is one `qd move` with
  `--indent` or `--outdent` when the depth changes.
- A group, an open head dragged alone, or a slide dropped outdented lands
  only where the next slide is a head or at the end of the deck (FR-017): other gaps don't open, and
  the nearest allowed gap does.

## Paste and duplicate

- Pasting slides that start with a head, or duplicating a collapsed group,
  on a slide inside a group goes after the end of that group; on a
  top-level slide, after the last top-level slide (FR-017). The editor
  passes that slide as `after`.

## Messages

After an op whose `changed` includes slides other than the one acted on,
or which turned a `---` slide into an empty head, the editor shows one
line, for example "Methods is now a group head: it shows as a section
slide." After moving or deleting an open head alone: "Its 3 slides now
belong to Results."
