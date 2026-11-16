# dot-image

A Quarto filter extension: pictures inside the boxes of a `{dot}`
(Graphviz) diagram. Quarto's own Graphviz ignores a node's `image="…"`;
this filter draws the picture into the rendered diagram, at the box's
left, clear of its label.

````markdown
```{dot}
digraph {
  layout=neato; splines=ortho;
  tool [label=<<TABLE BORDER="0" CELLPADDING="0" CELLSPACING="4"><TR><TD WIDTH="28" HEIGHT="28" FIXEDSIZE="TRUE"></TD><TD>Tool</TD></TR></TABLE>>, image="face.png", pos="0,0!"];
}
```
````

The label's first cell is an empty square that Graphviz lays out, so the
box is wide enough. The picture goes over that square, with the square's
size. A box with `image=` and a plain label also works: the picture then
goes at the box's left edge, and may cover a long label.

You don't need to write the spacer by hand:
- `qd dot set CELL NODE image=face.png` writes it, sized from the box's
  height (28 pt at the default 0.5 in);
- `qd dot rename` changes only its text;
- `qd dot set CELL NODE image=` puts a plain label back.

The first `image=` also installs this extension into the deck's
`_extensions/` and names it in `filters:`. Without qd: copy
`_extensions/dot-image/` into the deck's folder, and add it to the deck's
front matter:

```yaml
filters: [dot-image]
```

The picture is an `<image class="dot-image">`, so the deck's CSS can style
it. For example, a round picture:

```css
.dot-image { clip-path: circle(50%); }
```

A picture whose file doesn't exist is left out, and the render warns
naming it. The path is relative to the deck's folder. A one-file export
(`qd export`, or Quarto's `embed-resources`) carries the picture inside
the file.

**How it works** (spec 015, research R5):
- It runs after Quarto (`post-quarto`).
- It reads the deck's own source and finds each `{dot}` cell's boxes with
  `image=`, skipping arrows' attributes.
- It gives each rendered diagram the first cell, in order, whose picture
  boxes it has, so a diagram it can't see (from an include, or pasted SVG)
  doesn't shift the rest.
- It adds the `<image>` inside the box's `<g>`, before its label, so the
  text stays on top.

**Limit:** cells pulled in with `{{< include >}}` aren't scanned, so their
boxes get no picture. The render warns once when the deck uses `include`.
Quarto turns a diagram into SVG before any filter runs, so the cell's text
is only in the source.

**Licence:** our own work, like the rest of quartz-deck.

`example.qmd` uses a made-up picture, `face.png`: an orange disc generated
for this example.
