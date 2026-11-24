# Showcase deck

A made-up talk that uses each kind of slide and element qd edits, to try
the editor on and to see changes as they land.

```bash
qd serve --deck examples/showcase/showcase.qmd
```

It has:
- a title slide made from the front matter, with slide numbers and a default transition;
- formatting: bold, italic, underline, strike, highlight, code, a link, colours, a size and a font;
- bullet and numbered lists, with builds that fade up and turn red;
- a group of slides (Results) that you go down through;
- a table and a picture;
- two columns, and a slide whose columns build: the picture on the right, then the list on the left;
- boxes placed by hand, on a slide with its own background and transition;
- a diagram of four boxes;
- arrows;
- display math, a code cell, a code block and a callout;
- a dark slide;
- two slides that morph into each other;
- a hidden slide;
- speaker notes.

The arrows need the quarto-arrows extension. It has no licence file, so qd
downloads it into a deck the first time an arrow is added rather than
shipping it. On a fresh copy, add an arrow and undo it once:

```bash
cd examples/showcase
qd arrow add arrows --from 0,0 --to 10,10 && qd undo
```
