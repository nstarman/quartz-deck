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
