# GUI verification (T039, US3)

Run 2026-11-03 in the Claude desktop browser pane (Chromium, 1024×768 viewport)
against `qd serve` on a copy of `tests/fixtures/basic.qmd`. "== CLI" means the
deck file after the GUI action was compared with `cmp` against a fresh copy
edited by the equivalent `qd` command(s): byte-identical.

- [x] Open `/#/results` → preview opens on Results; panel shows its title/classes.
- [x] Drag "Conclusion" above "Results" in the slide list → order changes; == `qd move conclusion --before results`.
- [x] Click "Hello world." in the preview → outlined, panel shows `Para intro/0` with its source.
- [x] Edit the source, Apply → only that paragraph changes (+ `{#intro}` written); preview re-renders and keeps the selection; == `qd edit intro/0 --md …`.
- [x] Esc → back to slide panel. Set background `#ffe8a3`, transition `fade`, class `center`, Apply → == `qd set intro --attr background-color=#ffe8a3 --attr transition=fade --class center`.
- [x] ⌘Z (focus outside inputs) → file returns to the previous bytes; ⇧⌘Z → restores exactly.
- [x] "+ Add slide" → "New slide" inserted after the current slide, preview jumps to it.
- [x] Duplicate → `new-slide-1` after it, preview jumps to it; Delete (confirm) → removed, preview moves to the neighbour. Add+dup+rm == CLI.
- [x] Console: no errors besides 404 for `plot.png` (fixture references a missing image; `qd check` warns).

Found and fixed during this run:
- Reveal ignored the initial URL hash inside the iframe and started in *scroll
  view* when the pane was < 435 px wide → editor now disables scroll view and
  navigates explicitly once Reveal is ready.
- A stale cached `editor.js` survived a `qd` rebuild → editor assets are now
  served with `Cache-Control: no-cache`.
- Front-matter title slide fell back to the first slide's panel → it now has its
  own panel (deck title via `qd meta set title`).

## US4: positioned boxes (T045)

Same setup; deck from `qd new` plus `qd box add first-slide --text "**Drag me**" --at 100,200 --size 300,120`.

- [x] Click the box → overlay with 8 handles; panel shows Box with Left/Top/Width/Height = 100/200/300/120 (read from the file) and its source.
- [x] Drag the box body → file `left=300 top=464`, size unchanged. Motion = screen distance ÷ `Reveal.getScale()` (≈3.3 slide px per screen px in this pane): correct.
- [x] Drag the bottom-right handle → `width=399 height=219`, top-left unchanged.
- [x] Set 50/60/400/150 in the panel, Apply position → `left=50 top=60 width=400 height=150`.
- [x] Stock `quarto render` (no qd) of that file → `style="top: 60px; left: 50px; width: 400px; height: 150px"`: matches the editor (SC-006, also `render::box_position_fidelity`).
- [x] "Add text box" → `{#b-db5e .absolute left=375 top=300 width=300 height=100}` (centred on 1050×700), selected with handles once rendered.
- [x] Delete box → removed; ⌘Z → restored (journal: undo …, `box add`; redo `box rm`).
- [x] Page errors while selecting/deselecting boxes and other blocks: none.

Found and fixed: the box panel has two forms, and the source form was looked up
with `$('form', view)` → it picked the geometry form (TypeError). Forms are now
selected by class.
