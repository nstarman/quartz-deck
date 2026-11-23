# Quickstart: check the properties column

## Prerequisites

- `cargo build --release`; Quarto on `PATH`.
- The showcase deck (`examples/showcase/showcase.qmd`). Serve it with
  `qd serve --deck examples/showcase/showcase.qmd`, or run the `qd-showcase`
  launch entry. Work on a copy for the editing steps.

## 1. The CLI

```bash
cd <copy of examples/showcase>
qd build list builds
qd build set builds/1 --appear fade-in --leave fade-out
qd build order builds builds/2 builds/1
qd build list builds
qd meta set format.revealjs.slide-number false
qd meta size 1050 788 --keep-places
```

Expected:
- the build list shows the new effects and order;
- the deck's `format: revealjs:` gains the keys;
- placed boxes' `left`/`top` scale with the new height.

## 2. Format

1. Click a blank part of a slide. The column shows the slide. Pick a
   background colour: the slide changes. No bar floats over the slide.
2. Click a paragraph: Look, Text and Arrange tabs. Click a picture: Look
   and Arrange. Click a diagram box: its look, link and size.
3. Type in a paragraph. The floating toolbar shows. The Text tab's Bold
   bolds the selection too.

## 3. Animate

1. With nothing selected, choose Zoom, fast, and "Morph into the next
   slide".
2. Select a picture and choose Appear: Grow. Leave is greyed out: an
   element has one effect per click, so only one that fades in (or has no
   appear effect) can also leave.
3. Drag it to the top of the build list.
4. Present the slide and click through: the picture grows first.

## 4. Deck

Change the theme, the slide size to 4:3, and slide numbers. The preview
follows, and placed boxes keep their place.

## 5. Placement

1. Drag the column's header to each zone, and reload after each drop.
2. Narrow the window to 700 px: the column moves below the slide.
3. View ▸ Properties hides it, and again shows it.
