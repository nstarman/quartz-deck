# Quickstart run: 016 arrows on the slide (T018)

**Date**: 2026-11-17
**Build**: branch `016-polish` (debug)
**Setup**: a scratch 1600×900 deck `talk.qmd` with a slide `## A {#a}`
holding a paragraph, a made-up picture and speaker notes; qd's config and
cache in scratch folders; Quarto 1.11.9; network on (except §5).

## §1 Commands (US1–US4) — pass

- [x] `arrow add a --from 200,600 --to 700,300` printed `a/2`, added one
  line before the notes, and installed `_extensions/emilhvitfeldt/arrows/`
  (downloaded into the scratch cache, checked by SHA-256).
- [x] `arrow move a/2 --to 760,280` changed only `to`.
- [x] `arrow bend a/2 --through 450,350` added `control1="440,320"
  control2="440,320"`: the curve's middle is (200 + 6·440 + 760)/8 = 450,
  (600 + 6·320 + 280)/8 = 350, the point given.
- [x] `arrow set a/2 color=crimson width=4 dash=true label="the peak"
  fragment=true` added only those options, after the last one.
- [x] `arrow show a/2` gave the ends, both control points and
  `editable: true`.
- [x] Four `undo`s removed the line; the extension's files stayed, unused,
  as the spec says.
- [x] `quarto render`: the arrow's ends landed at exactly (200, 600) and
  (700, 300) in slide pixels.

## §2 On the slide (US1) — pass

- [x] In the browser pane, on a scratch deck: the arrow rendered; a click on
  its line selected it with its handles and the element bar beside it;
  dragging its end saved only `to` and the re-render drew it there.
- [x] The headless browser tests, with real mouse and key input:
  `arrow_draw` (Insert ▸ Arrow, a drag, snapping to a box's edge exactly,
  ⌘ to place freely), `arrow_select_and_drag` (ends, the line, Escape),
  `arrow_keys_and_delete`, `arrow_click_beside_line`.

## §3 Curve and style (US2) — pass, by the browser tests

- [x] `arrow_bend_handle`: the curve passes within 2 px of the drop point;
  the middle handle then sits on the curve; dropping it on the line
  straightens it.
- [x] `arrow_properties_style`: colour, width, dash, head and heads from the
  Properties panel give exactly the expected line, and the render is
  crimson.

## §4 Label and appearing on click (US3, US4) — pass, by the browser tests

- [x] `arrow_label_by_double_click`: the sheet sets the label, the render
  shows it, Escape changes nothing, the panel sets its position, and an
  empty label removes it.
- [x] `arrow_appears_on_click`: the element bar's toggle writes
  `fragment="true"` (no class), the preview wraps the arrow in a fragment,
  and the button shows as pressed.
- [ ] Not watched by eye: the draw-in when presenting (quarto-arrows' own
  animation).

## §5 Offline the first time — pass

- [x] With `QD_NO_DOWNLOAD=1` and an empty cache, `arrow add` failed with
  `E_DOWNLOAD` and its hint; the deck was byte-for-byte unchanged and no
  `_extensions/` folder was made.
