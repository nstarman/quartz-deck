# Research: Arrows on the slide (016)

Each entry: what was measured, the decision, and what else was weighed.

## R1. Coordinates

**Measured** (a spike, 2026-11-17, Quarto 1.11.9, quarto-arrows v0.3.0): on
a 1600×900 reveal.js deck, `{{< arrow from="0,0" to="1600,900"
position="absolute" >}}` draws its line from the slide's top-left corner to
its bottom-right, exactly. With `position="absolute"` the extension wraps its
SVG in `<div style="position: absolute; left: Lpx; top: Tpx">`, and a
reveal.js `<section>` is itself `position: absolute`, so the div's offset
parent is the slide and the numbers are the slide's canvas pixels (before
reveal.js scales the slide to the window).

**Decision:** arrows qd places use `position="absolute"`, with `from`/`to`
(and control points) in slide pixels, whole numbers. The editor's placed
boxes (spec 009) already work in slide pixels, so the editor converts with
the same scale.

**Alternatives:** inline arrows (the default, in the text's flow) have no
fixed relation to the slide; `position="fixed"` is relative to the window.
Neither can be placed by hand.

## R2. A shortcode, not a filter

**Measured:** quarto-arrows' `_extension.yml` contributes `shortcodes:
[arrows.lua]` only. A deck with the extension in `_extensions/` and no
`filters:` entry renders its arrows (the README's `filters: - arrows` is not
needed for the shortcode, and does no harm).

**Decision:** installing the extension is enough; qd does not add a
`filters:` entry. So adding the first arrow is an edit to the slide plus the
extension's files. Undo removes the arrow line; the extension's files stay
in `_extensions/`, where they do nothing without an arrow (as dot-image's
do).

## R3. Getting quarto-arrows: download, don't ship

**Measured:** the repository has tags v0.1.0, v0.2.0 and v0.3.0 (commit
`957a61bc1ef13ec913baefce0f7eee61226c140d`, 2026-10-05, the latest commit),
and three releases. Its README ends "## License / MIT", but the repository
has no licence file and GitHub detects none. The extension is six files,
58 KB. Each file fetched from `raw.githubusercontent.com` at that commit is
byte-identical to what `quarto add emilhvitfeldt/quarto-arrows` installs:

| file | SHA-256 |
|---|---|
| `_extension.yml` | `aa4ebbf40f589bef7817229de8e9cbe9af331f959bf84656ca8a05fcfa8c2f4f` |
| `_schema.yml` | `c21dbf10e0e7e64f8dce62783e7901f54800f4a6cb23b48a3c7e9fdc81a230c3` |
| `_snippets.json` | `a90a57a8a18e7634c583dbe0dacc7efcffacc431b0fc3c36a77b30a6f93dc8f5` |
| `arrows.lua` | `bb67c703f9a984f4b519d8278670181e12d1930efb9458a456b564d8edced398` |
| `utils.lua` | `fa1219cc57f1017a38fe493f19cbf3db70d9fa3d20dcbbcf6fbfab05a99544e5` |
| `resources/arrows-fragment.js` | `a794e3cda99717107cc84d9a93d16a056664827338e2d26325126e91b423148b` |

**Decision (with the person):** qd doesn't redistribute the extension. The
first time a deck gets an arrow, qd downloads the six files from
`https://raw.githubusercontent.com/EmilHvitfeldt/quarto-arrows/<commit>/_extensions/arrows/<file>`,
pinned to the commit (a tag can move), checks each SHA-256, and keeps them in
qd's cache (`<cache>/lib/quarto-arrows-0.3.0/`), as for MathJax
(`assets::ensure`, `download_verified`). From there it installs them into
the deck as `_extensions/emilhvitfeldt/arrows/`, the layout `quarto add`
uses, so `quarto update` still recognises it. `QD_NO_DOWNLOAD` and
`--no-download` are respected; without a cached copy the edit fails with
`E_DOWNLOAD` and changes nothing.

**Alternatives:** bundling the files into qd (rejected: no licence file);
a tarball from GitHub's archive endpoint (rejected: its bytes aren't
guaranteed stable, so a pinned checksum could break); running `quarto add`
(rejected: it prompts, needs network every time, and its version isn't
pinned by checksum); qd's own filter (rejected by the person in favour of the
existing library).

**Licence test:** the downloaded-assets list gains quarto-arrows with "MIT"
(from its README) and a note that qd downloads but doesn't redistribute it.

## R4. An arrow in the deck model

**Measured:** qd's deck reader sees a line holding only an arrow shortcode
as an ordinary paragraph block (`a/1`, kind `para`), and the preview copy
wraps each block in `::: {data-qd="a/1"}`, so an arrow already has a ref,
moves between slides, deletes and undoes like any element.

**Decision:** an **arrow** is a paragraph whose whole text is one
`{{< arrow … >}}` shortcode. A new module `arrows.rs` reads its options
with their spans (`key="value"`, `key='value'` and `key=value`), and each
command patches only the option it changes, inserting new options before
`>}}` in a fixed order and leaving hand-written order and quoting alone.
`qd arrow add` inserts the line after the slide's last content block and
before its speaker notes. An arrow whose `position` isn't `absolute`, or
whose shortcode qd can't read (unbalanced quotes, `>}}` inside a value, two
shortcodes in one paragraph), is reported read-only with the reason.

**Alternatives:** grouping a slide's arrows in one block (rejected: one
element per arrow keeps selection, undo and Delete simple); a new block
kind in `deck.rs` (rejected: a paragraph test is enough and keeps the deck
reader unchanged).

## R5. The editor's overlay

**Decision:** a new module `ui/arrows.js`, like `dot.js`:
- The extension's SVG is `pointer-events: none`. The overlay adds a
  transparent hit path, about 12 px wide, along each editable arrow's line,
  so a click on the line selects the arrow and a click beside it reaches
  the element underneath (spec 015's edge hit path, the same idea).
- A selected arrow shows three handles: start, end and middle. The handles
  sit in slide pixels, converted with the slide's scale.
- Snapping: a dragged end, or the whole arrow's two ends, snap to the
  editor's existing guide lines, the slide's edges and centre and each
  element's edges and centre. The same `nearest` and `SNAP_PX` come from
  `ui/guides.js` (spec 015, T037). The same `placement.snap` setting
  applies, and ⌘/Ctrl places freely.
- Insert ▸ Arrow enters a drawing mode: the next drag on the slide draws.
  A click without a drag adds a 200 px arrow pointing right from there.

## R6. Bending by the middle handle

**Decision:** quarto-arrows draws a cubic Bézier through `control1` and
`control2`. To make the curve pass through the point M where the presenter
drops the middle handle, qd writes both control points as
`C = (8M − P0 − P1) / 6`: a cubic with equal control points passes through
`(P0 + 6C + P1) / 8` at its middle. A drop within 4 px of the straight
line removes both control points. The extension's `curve` and `bend`
options, if written by hand, are kept and drawn; a middle-handle drag
replaces them with control points.

**Alternatives:** `curve` plus `bend` (rejected: a curviness and a side
can't put the curve through the point the presenter chose).

## R7. Labels and appearing on click

**Decision:**
- Double-click opens the editor's existing sheet (`sheet.js` `ask`) with a
  text field for `label`. An empty label removes the option. Properties
  sets `label-position` (start, middle, end).
- "Appears on click" on the element bar toggles `fragment="true"` on an
  arrow, rather than a `.fragment` class as for other elements, because
  quarto-arrows draws the arrow in itself. Its order follows the slide's
  other fragments in source order; `fragment-index` written by hand is
  kept.

## R8. Showing a drag before the re-render

**Decision:** moving the whole arrow translates its rendered SVG, which is
exact. Dragging one end, or drawing, shows a provisional dashed line with a
small head from start to end (curved through the control points when there
are some), as the diagram overlay does for its boxes. The drop runs the
command, and the preview's re-render draws the real arrow (FR-018).
