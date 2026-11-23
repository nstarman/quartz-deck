# Quickstart run: the properties column

**Run**: 2026-11-23, on a copy of `examples/showcase`, with the build from
branch `023-placement` (PR 4).

## §1 The CLI

- [x] `qd build list builds`: three builds in presentation order (3, 1, 2),
  with their effects and indexes.
- [x] `qd build set builds/1 --appear fade-in --leave fade-out`: written as
  `.fade-in-then-out`. The quickstart first said `--appear grow --leave
  fade-out`, which the op refuses on purpose: an element takes one effect per
  click, so a leave goes only with fading in or with no appear effect. The
  quickstart now uses a valid pair, and the Animate view greys out Leave when
  the appear effect can't take one.
- [x] `qd build order builds builds/2 builds/1 builds/3`: `build list` shows
  that order, with `fragment-index` 1–3.
- [x] `qd meta set format.revealjs.slide-number false`: edited in the
  existing `format: revealjs:` mapping, the other keys kept.
- [x] `qd meta size 1050 788 --keep-places`: `width` and `height` join the
  mapping. `#box-note` moves from `top=150` to `top=169`, and the arrow from
  `160,420` to `160,473` (× 788/700). Widths and `left` are unchanged, since
  the width stays 1050.

## §2–§5 The editor

Each step is run by a browser test, against the editor and Quarto's
rendering:

- [x] §2 Format: `props_format_slide`, `props_format_element`,
  `props_text_tab`, `props_title_slide`.
- [x] §3 Animate: `props_animate` (transition, speed, morph, appear and
  leave, Leave greyed out after fade-up, reordering by ⌥↓ and by a drag);
  `build_order_presents` (presenting shows the builds in the set order).
- [x] §4 Deck: `props_deck` (theme, 4:3 with a placed box scaled, slide
  numbers, transition, back to the default).
- [x] §5 Placement: `props_dock` (a drag to each of the four places, kept
  after a reload; below the slide at 650 px; View ▸ Properties hides and
  shows it; beside the source pane on the left, right and below, in the set
  order); `props_keyboard` (F6 in and out, ← → along views and tabs, Esc from
  a field).

## SC-001: every old control's new home

| Before | Control | Now |
|---|---|---|
| Slide bar | Background colour | Format ▸ Slide |
| Slide bar | Background image | Format ▸ Slide |
| Slide bar | Transition | Animate (nothing selected) |
| Slide bar | Hidden | Format ▸ Slide |
| Slide bar | ⋯ (slide actions) | Right-click the slide, or the Slide menu |
| Element bar | Run, Edit, Markdown, Delete, ⋯ | The column's footer |
| Element bar | Appears on click | Animate ▸ Appear (an arrow: "Draws in on a click") |
| Properties panel | Slide form (title, classes, markdown) | Format ▸ Slide |
| Properties panel | Title slide (title, subtitle, author, date) | Format ▸ Title |
| Properties panel | Element panels (look, link, picture, geometry) | Format ▸ Look and Arrange |
| Inspector (spec 019) | Slide, Element, Text, Position tabs | Format ▸ Slide, Look, Text, Arrange |
| Text toolbar | Bold, italic, colour, font, size, alignment, lists, indent | Still floats while typing, and in Format ▸ Text |

None is reachable only from a floating bar except the text toolbar's, which
the Text tab also has.

## T024: on a copy of the person's talk

Run 2026-11-23 on a scratch copy, in the editor and from the column. The
talk itself was not changed, and nothing from it is in this repository.

- [x] A background colour on one slide (Format ▸ Slide), and a fade
  transition on it (Animate).
- [x] Two builds on another slide: a picture fading in and a list fading up,
  then the list moved first with ⌥↑ in the build list. Presenting showed the
  list on the first click and the picture on the second.

Found, and fixed in the same PR:
- A slide of the file whose id is `title-slide`, in a deck with no `title:`,
  was taken for Quarto's title slide. Format showed the front matter's title
  form, and its Apply would have added a `title:`. It is now treated as the
  slide it is (test `props_own_title_slide`).
- A build's name kept markdown marks (a bold first word left a stray `**`), and a picture
  with no alt text was named "Image" though it had a `fig-alt` (test
  `build_names`).

Left for later:
- Elements inside `.columns` can't be selected or given builds one by one.
- On a slide filled by a picture, a click lands on the picture; Tab reaches
  the elements under it.
