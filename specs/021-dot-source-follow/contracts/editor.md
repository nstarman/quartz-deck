# Contract: the editor follows the cursor in a diagram's source

## Where it listens

- **Source pane** (`ui/pane.js`): caret moves and typing (60 ms after
  they stop), and the pointer resting 300 ms over a line. Esc, blur, or a
  caret outside every ```` ```{dot} ```` fence clears.
- **Cell editor on the slide** (`ui/source.js`, `#srcedit`): the same,
  when the element being edited is a `{dot}` cell.

## What it does

1. Takes the cell's body and the offset as the textarea has them, and asks
   `dot at`.
2. **Outline** (FR-002, FR-003): adds `qd-dot-follow` to the matching
   `g.node`, `g.edge` (one or several) or `g.cluster` in the cell's SVG on
   the slide, and removes it everywhere else. It doesn't touch the
   selection, the bars or the inspector.
3. **Preview** (FR-004, FR-005), for `node` and `edge` when
   `diagrams.source-preview` is on:
   - lays out `alone` with the page's Graphviz and adds pictures;
   - shows it in `#dot-peek`, a small pop-over beside the cursor's line
     (or the hovered line) that doesn't cover it and stays inside the window;
   - with `aria-hidden` and no focus, since it is a picture.
4. **Stop** (FR-007): keeps the last outline and preview, and adds
   `.stale` to `#dot-peek` (dimmed, "out of date").
5. Nothing while presenting or in the speaker view (FR-011).

## Look

- `qd-dot-follow`: a dashed 2 px outline in the accent colour, distinct
  from the selection's solid outline. Arrows get a wider dashed stroke.
- `#dot-peek`: the panel's background and border, a soft shadow, at most
  240 × 160 px, the drawing scaled down to fit.

## Tests (`tests/onslide.rs`)

- `source_follow_outlines`: caret on a node line, an edge line, each link
  of a chain, a setting line and outside the cell. The outline is on
  exactly the right element, or none, and the selection is unchanged.
- `source_follow_hover`: the pointer resting on another node line moves
  the outline; leaving gives it back.
- `source_follow_preview`: the preview shows the node's label and shape.
  Typing a new `fillcolor` redraws it before the slide renders (Quarto
  slowed through `QD_QUARTO`). An unclosed quote leaves it marked `.stale`.
  Esc closes it.
- `source_follow_setting_off`: with the setting off, no preview, and the
  outline still there.
- `source_follow_cell_editor`: the same outline from `#srcedit`.
