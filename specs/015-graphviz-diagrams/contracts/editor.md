# Editor contract: diagrams (015)

The overlay is `src/ui/dot.js`. It acts on `div[data-qd][data-qd-lang=dot]`
in the preview whose `qd dot show` says `editable`. It sends every change
through the op route as the `qd dot` command of the same name (Principle
III).

| Action | Gesture | Command |
|---|---|---|
| select a box | click a node | none (`qd dot show` feeds the panel) |
| select an arrow | click an edge | none |
| move | drag a selected box and drop; Escape cancels | `move --by DX,DY` |
| connect | drag from one of four handles onto a box | `connect A B` |
| add a connected box | drag from a handle onto empty space | `add --from A --at X,Y --look-of A` |
| rename | double-click a box, type, Enter (Escape cancels) | `rename` |
| delete | Delete or Backspace on a selected box or arrow | `rm` |
| look, link, image, size | Properties panel fields | `set` |
| direction, arrowhead | Properties panel for an arrow | `flip`, `set arrowhead=none\|normal` |
| pin | Properties ▸ "Pin layout", or the prompt on a first drag of an unpinned diagram | `pin --positions` (read from the SVG) |
| Insert ▸ Diagram | menu, on a slide | `new --on SLIDE` |
| Insert ▸ Box | menu, with a diagram selected | `add --at (centre) --look-of (last selected)` |

**While dragging:**
- the node's `<g>` moves with a CSS transform;
- its edges are replaced by provisional straight `<line>`s centre to
  centre;
- with `diagrams.live-layout` on, a routed layout replaces them each
  frame instead (falling back to straight lines on any failure);
- on drop, the command runs and the next preview render replaces
  everything.

**Accessibility:**
- every action is also in the Properties panel or a menu;
- handles have accessible names ("Connect from figpress");
- arrow keys nudge a selected box by 0.25 in (`move --by`).

**Actions:** in `actions.json`:
- `dot-new`, `dot-add` (Insert);
- `dot-rm`, `dot-flip`, `dot-pin` (element menu, where `diagram`).

**Setting:** Settings ▸ Diagrams ▸ "Route arrows while dragging"
(`diagrams.live-layout`, off), with a note that the first use may download
the layout engine when the person's Quarto has none the browser can load.
