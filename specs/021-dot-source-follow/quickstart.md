# Quickstart: check that the source follows the cursor

## Prerequisites

- `cargo build --release`.
- Quarto on `PATH` (for the preview and its Graphviz).
- A deck with a diagram, e.g. a copy of `crates/qd/tests/fixtures/` decks
  with a `{dot}` cell of five boxes and a chain `a -> b -> c`.

## 1. `qd dot at` from the command line

```bash
printf 'digraph {\n  a [label=A];\n  a -> b -> c;\n}\n' | target/release/qd dot at --text - --at 14
```

Expected: `"kind": "node", "node": "a"`, and an `alone` graph with
`label=A`. Try `--at` on `b -> c` (`"edge": "b->c#1"`), on `digraph {`
(`"at": null`), and with an unclosed quote (`"stopped"`).

## 2. In the editor

1. `target/release/qd serve --deck <copy>/deck.qmd`, open the source pane
   and go to the diagram's slide.
2. Put the cursor on a box's line: that box gets a dashed outline on the
   slide, and a small drawing of it shows beside the line.
3. Arrow down through the lines: the outline and the drawing follow. On
   `layout=…` both go away.
4. Change the box's `fillcolor`: the drawing changes at once; the slide
   changes after its render.
5. Type an unclosed `"`: the drawing dims (out of date). Close it: it's
   back.
6. Rest the mouse on another box's line: that box is outlined. Move away:
   back to the cursor's box.
7. Select something else on the slide first: its bars stay as they are
   while the outline moves.
8. `qd settings set diagrams.source-preview false`: no drawing, the
   outline stays.

## 3. SC-002 timing

In the browser pane's console, time from a caret move to the class
appearing on the 20-box diagram:

```js
const t = performance.now(); /* move the caret */; await new Promise((r) => requestAnimationFrame(r));
```

Expected: under 100 ms.
