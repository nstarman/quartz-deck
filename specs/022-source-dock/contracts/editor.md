# Contract: the source pane's place in the editor

## Setting

- `qd settings set source.side far-left|left|right|below`; the editor
  follows within the usual settings broadcast, in every open editor.
- The Settings window lists the four values under "Source pane place".

## DOM (what the browser tests rely on)

- `main[data-source]` is the current value of `source.side`.
- `#splitter[aria-orientation]` is `vertical` in side places and
  `horizontal` below the slide or on a window ≤ 700 px.
- During a drag: `main.docking`, and `#dock-zones > [data-place]`, one per
  place (only `below` on a window ≤ 700 px); the one under the pointer has
  class `over`. Each zone's text is its label.
- After a drop or a cancel, `#dock-zones` is hidden.

## Pointer and keys

| Input | Where | Effect |
|---|---|---|
| press + move ≥ 4 px | empty part of `#srcpane .srcbar` | starts a drag, zones show |
| press | a button or field in `.srcbar` | that control only; no drag |
| release over a zone | during a drag | `source.side` set to that zone's place |
| release elsewhere, Esc, pointer lost | during a drag | nothing changes |
| ← → | `#splitter`, side place | width ± 20 px (as today) |
| ↑ ↓ | `#splitter`, below | height ± 20 px |
