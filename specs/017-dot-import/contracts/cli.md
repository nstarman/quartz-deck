# Contract: `qd dot import`

```text
qd dot import FILE.svg (--on SLIDE | --after REF | --before REF | --replace REF)
              [--deck DECK] [--dry-run] [--expect VERSION] [--human]
```

- **Input:** reads `FILE.svg`, either hand-written or a Graphviz render
  (research R7).
- **Effect:** writes one pinned `{dot}` cell (research R6):
  - `--on`, `--after` and `--before` place it as `qd dot new` does;
  - `--replace` puts it in place of the element `REF` (an image of the
    drawing, or the SVG as raw HTML).
- **One edit:** one op and one undo step. With a picture, the `filters:`
  change is part of the same edit.
- **Output:** JSON, as in data-model.md "Report". With `--dry-run` it also
  has `diff`, and nothing is written.
- **Errors:** `E_IO` (unreadable file); `E_INVALID_ARG` (not SVG, no boxes,
  or two places given); `E_NOT_FOUND` (`REF` or `SLIDE` unknown);
  `E_CONFLICT` (`--expect`). Nothing is written on any error.

## The cell written

````text
```{dot}
//| fig-width: 8.42
//| fig-height: 3.46
digraph {
  layout=neato; splines=ortho;
  data [label="Data", pos="0,1.5!", width=1.5, height=0.6, fixedsize=true, shape=box, style="rounded,filled", fillcolor=white, color="#c8ccd2", fontcolor="#1f2933"];
  tool [label=<…spacer…>, image="face.png", pos="3,1.5!", …, URL="https://example.org/tool"];
  data -> tool;
}
```
````

- **Box statements:** one per box, in drawing order, then the arrows. Each
  is written with qd's own `node_stmt`, so `qd dot show` reads the result
  and every `qd dot` command can edit it (FR-008).
- **Attributes:** only those the drawing gave are written. A box with no
  fill gets no `fillcolor`.

## Schema and skill

- **`qd schema`** lists `dot import` with its output shape.
- **The skill** (`skills/quartz-deck/SKILL.md`) gets a short section: when
  the person has a diagram as an SVG, import it, then refine it with the
  `qd dot` commands. Check `left_out` and `notes`, and tell the person.
