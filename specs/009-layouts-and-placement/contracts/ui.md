# UI contract additions (009)

New actions in `src/ui/actions.json` (FR-020); keys checked against VS Code's
by the node test.

| Id | Label | Menu | Key | Where | qd |
|---|---|---|---|---|---|
| slide-after | Slide after… (opens the gallery) | Insert, Slide | ⇧⌘N (as 007) | slides, slide, element | `qd add --layout <name> --after <slide>` |
| slide-before | Slide before… | Insert, Slide | — | slides, slide | `qd add --layout <name> --before <slide>` |
| place | Place freely | Arrange (new menu), right-click | — | element | `qd block place <ref> --at X,Y` |
| unplace | Put back in the flow | Arrange, right-click | — | element | `qd block unplace <ref>` |
| front / back / forward / backward | Bring to front … | Arrange, right-click | ⌥⇧⌘F / ⌥⇧⌘B, ⌥⌘F / ⌥⌘B | element | `qd box front <ref>` … |
| box-nudge-left/right/up/down | Nudge | — (keys only, listed in the reference) | ←→↑↓, ⇧ for 10 (plain arrows are free on a selected element; ⌥↑/⌥↓ stay Move up/down) | element (a box) | `qd box move <ref> --by` |
| layout-save | Save slide as layout… | Slide | — | slides, slide | `qd layout save` |
| layouts-manage | Layouts… | Slide | — | anywhere | — (the gallery with rename/delete) |
| snap | Snap to guides | View | — | anywhere | setting `placement.snap` |

**Gallery** (a sheet, 008): a grid of previews with names, the last used
preselected; arrows move, Enter adds, Esc cancels; the person's and the deck's
layouts follow the built-in ones under their own headings, with rename and
delete on each.

**Canvas**:
- ⌥-drag an element in the flow: a translucent copy follows the pointer and
  the drop places it.
- A placed element shows the box overlay with handles. Guides are 1 px lines
  in the accent colour.
- An element partly off the slide gets a dashed outline.
- Empty placeholders show "Click to add text" or "Drop an image" (editor CSS
  only).
