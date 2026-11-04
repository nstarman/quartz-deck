# Research: 001-core-editor

Sources: Quarto docs — [Reveal.js basics](https://quarto.org/docs/presentations/revealjs/),
[Advanced reveal](https://quarto.org/docs/presentations/revealjs/advanced.html);
Pandoc manual (headings, fenced divs, attributes). No proprietary editor
material was consulted (CLEANROOM.md).

## R1. How Quarto delimits slides

- **Decision**: Model slides as Pandoc does for `revealjs`: a slide starts at
  a heading of level `slide-level` (default 2) or at a horizontal rule
  `---`; level-1 headings make section/title slides. The YAML front matter
  produces the title slide. Respect `slide-level` from front matter.
- **Rationale**: Matches the rendered output, so slide indices agree with the
  preview.
- **Alternatives**: Ask Pandoc for the AST (`quarto pandoc -t json`) — no
  source positions, so it can't drive lossless patches. Used only in
  `qd check` to cross-verify slide count.

## R2. Parser: lossless segmenter vs. full Markdown parser

- **Decision**: Hand-written, line-based segmenter producing a tree of byte
  spans: front matter → slides → top-level blocks. Blocks recognized: ATX
  headings, fenced code (``` and ~~~, incl. `{r}` cells), fenced divs
  (`:::`, nested by colon count), HTML blocks, display math `$$`, lists,
  tables, block quotes, paragraphs, `. . .` pauses. Everything inside a
  block is opaque text. Spans tile the file exactly, so unedited output is
  byte-identical by construction.
- **Rationale**: No Rust crate parses Pandoc Markdown (fenced divs,
  attribute blocks) with source spans; `pulldown-cmark`/`comrak` would
  mis-segment fenced divs. We need only slide/block boundaries, not inline
  structure.
- **Alternatives**: tree-sitter-markdown (no fenced divs); calling Pandoc
  (no spans).
- **Safety rule**: If a construct is ambiguous (e.g. unterminated fence),
  the slide is marked `opaque`; read works, structural edits inside it are
  refused with `E_UNSAFE_EDIT`.

## R3. Stable IDs

- **Decision**: Slide ID = explicit `{#id}` on its heading, else Pandoc's
  auto-identifier from the title (read-only, reported with
  `"explicit": false`), else positional `@N`. Any edit that needs to address
  a slide by a non-explicit ID first writes `{#id}` to that slide's heading
  (`---` untitled slides become `## {#id}`, which Quarto treats as an
  untitled slide). New slides/boxes always get explicit IDs
  (`s-<4 hex>`, `b-<4 hex>`). Elements without IDs are addressed as
  `<slide>/<index>` (index among top-level blocks).
- **Rationale**: Gives agents stable handles while touching only the slide
  being edited (FR-003).

## R4. Positioned boxes

- **Decision**: A box is `::: {#b-xxxx .absolute top=… left=… width=… height=…}`
  fenced div (or an image with those attributes `![](x.png){#b-xxxx .absolute …}`).
  Values written as unitless integers (px in the 1050×700 default, or the
  deck's `width`/`height`). Existing non-px units are preserved; dragging
  such a box converts that box to px and says so in the op result.
- **Rationale**: Quarto docs: `.absolute` with `width`, `height`, `top`,
  `left`, `bottom`, `right`; unitless = px. Renders identically without
  the editor.
- **Drag math**: the editor converts pointer deltas to slide px by dividing
  by `Reveal.getScale()`; writes on pointer-up only.

## R5. Preview rendering and element mapping

- **Decision**: `qd serve` renders a *preview copy* of the deck (written to
  the cache dir next to a symlink farm/`resource-path` so relative assets
  resolve) in which each top-level block of each slide is wrapped in
  `::: {data-qd="<slide>/<i>"}` — except blocks that already carry
  attributes (headings, divs, images with attrs), which get `data-qd` added
  inside their own attribute block. Final `qd render` uses the untouched
  file. The browser maps clicks to `data-qd` → op target.
- **Rationale**: Gives click-to-select for every block with zero heuristics,
  using the segmenter we already have.
- **Risk**: an extra wrapper div could alter layout for CSS that targets
  direct children (e.g. `.columns > .column`). Mitigation: never wrap inside
  divs, only slide-level children; corpus test compares rendered slide
  screenshots of preview vs. final for drift. Fallback: annotate via a Lua
  filter by ordinal.
- **Render**: debounced (150 ms) `quarto render <copy> --to revealjs
  --output-dir <cache>`; one render at a time, latest-wins. Failures keep the
  last good output; the error is pushed to the UI and returned by `check`.
- **Alternatives**: `quarto preview` (its own server/port — cross-origin
  iframe, can't inject the editor script or annotate blocks).

**Verified (spike, Quarto 1.11.9 / Pandoc 3.10)**: `## {#untitled-x}`
renders `<section id="untitled-x" class="slide level2">`; a
`::: {data-qd="intro/0"}` wrapper passes `data-qd` through to HTML;
`.absolute top=100 left=200 …` renders `style="top: 100px; left: 200px; …"`;
`.notes` still becomes `<aside class="notes">`.

## R6. Live update channel

- **Decision**: Server-Sent Events (`GET /events`) for server→browser
  (`rendered`, `render_error`, `changed`); `POST /op` for browser→server.
  Reload keeps position via reveal's URL hash (`#/<slide-id>`).
- **Rationale**: Browser-native `EventSource`, auto-reconnect, one-way is
  all we need. Simpler than WebSockets.

## R7. Concurrency and undo

- **Decision**: Version token = first 16 hex chars of SHA-256 of the file
  bytes. Every op: read → compute version → apply → re-read and compare
  version (abort `E_CONFLICT` if changed) → write temp file in same dir →
  `rename`. Callers may pass `--expect <version>` to bind an edit to what
  they last saw. Undo journal: append-only JSONL of `{version_before,
  version_after, before_text}` in the cache dir; `qd undo` restores only if
  the current version equals the last entry's `version_after`.
- **Rationale**: Constitution V; no lost writes, no hidden deck state.

## R8. Dependency licenses (FR-041)

| Crate | License | Use |
|---|---|---|
| clap | MIT OR Apache-2.0 | CLI |
| serde, serde_json | MIT OR Apache-2.0 | JSON |
| schemars | MIT | `qd schema` |
| axum, tokio, tower-http | MIT | server |
| serde_yaml_ng | MIT | front matter |
| thiserror | MIT OR Apache-2.0 | errors |
| notify | verify at add time (recent releases MIT OR Apache-2.0; older CC0-1.0) | file watch |
| sha2 | MIT OR Apache-2.0 | version token |
| similar | Apache-2.0 | diffs |
| insta, assert_cmd, tempfile (dev) | Apache-2.0 / MIT | tests |
| Quarto (external) | MIT | render |
| reveal.js (via Quarto output) | MIT | runtime |

All are compatible with private, proprietary use. Each is re-checked with
`cargo metadata` when added.

## R9. Skill delivery

- **Decision**: `skills/quartz-deck/SKILL.md` in repo, embedded in the binary;
  `qd skill` prints it, `qd skill --install <dir>` writes it to e.g.
  `~/.claude/skills/quartz-deck/`. A test asserts every command named in the
  Skill exists in `qd schema`.
- **Rationale**: Keeps Skill and CLI in sync (FR-014).
