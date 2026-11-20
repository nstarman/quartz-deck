# Quickstart: import a drawing

Uses made-up drawings only (the test fixtures).

## Setup

```bash
cargo build --release
QD=$PWD/target/release/qd
mkdir -p /tmp/dot-import && cd /tmp/dot-import
printf -- '---\ntitle: Import\nformat: revealjs\n---\n\n## Flow {#flow}\n\n' > deck.qmd
cp $OLDPWD/crates/qd/tests/fixtures/dot-import/*.svg $OLDPWD/plugins/dot-image/face.png .
```

## Scenarios

1. **Hand-drawn, dry run**: `$QD dot import hand.svg --on flow --dry-run`.
   The JSON lists 4 boxes and 3 arrows, the stray line and text under
   `left_out` with reasons, and a `diff`. The file is unchanged.
2. **Import**: the same without `--dry-run`. `$QD dot show flow/0` reads it:
   - every box is pinned, with the labels, the link and the picture;
   - the ellipse is `shape=ellipse`;
   - the double-headed arrow has `dir=both`.

   The front matter has `filters: - dot-image`.
3. **Render**: `$QD serve` and look at the slide. The boxes sit where they
   are in `hand.svg` (open it beside the slide); the arrows are routed at
   right angles.
4. **Undo**: `$QD undo` puts the deck back as it was, in one step.
5. **Graphviz**: `$QD dot import graphviz.svg --on flow`. The names in
   `qd dot show` are those of the DOT it was rendered from, and the arrows
   are the same.
6. **Replace**: add `![](hand.svg)` to the slide, then run
   `$QD dot import hand.svg --replace flow/1`. The image becomes the cell.
7. **Errors**: `$QD dot import deck.qmd --on flow` gives `E_INVALID_ARG`
   ("not SVG"), and an SVG with only lines gives `E_INVALID_ARG` ("no
   boxes"). The file is unchanged both times.
