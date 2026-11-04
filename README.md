# quartz-deck

A local GUI + CLI editor for [Quarto](https://quarto.org) reveal.js
presentations, written in Rust.

- The `.qmd` file is the source of truth. The GUI and the CLI both edit it
  with minimal text patches, so it stays readable and diff-friendly.
- `qd serve` opens an editor in your browser: slide list, live reveal.js
  preview, click-to-edit, slide properties, drag/resize boxes, undo/redo.
- Every editing action is also a CLI command with JSON output, so an LLM
  (e.g. Claude Code) can edit the deck in-session and watch the same
  browser preview.

Status: specs [001](specs/001-core-editor/spec.md) (editor) and
[002](specs/002-offline-export/spec.md) (offline export + PDF) implemented. See [`specs/`](specs/) and the
[constitution](.specify/memory/constitution.md).

## Install

Needs Rust ≥ 1.89; for preview/render [Quarto](https://quarto.org/docs/get-started/) ≥ 1.4;
for offline check/export/PDF a Chrome or Chromium (or qd downloads Chrome for Testing).

```bash
cargo install --path .
qd skill --install ~/.claude/skills/quartz-deck   # teach Claude Code to use qd
```

## Use

```bash
qd new talk.qmd --title "My talk"                # or work on an existing deck
qd add --title "Results" --md $'- one\n- two'    # JSON out; target = "results"
qd set results --attr background-color=aquamarine
qd box add results --image figures/plot.png --at 100,150 --size 600,400
qd --human slides
qd check                                         # --render also runs Quarto
qd serve --open                                  # browser editor + live preview
qd render                                        # final output; reports its path
qd undo
```

The deck is the only `*.qmd` in the current folder, or `--deck PATH` /
`$QD_DECK`. `qd schema` describes every command and output; errors are JSON on
stderr with a stable `code` and a `hint`. Decks inside a Quarto project
(`_quarto.yml`, e.g. the `quarto-revealjs` skill's offline setup) work as is.

### Offline export (spec [002](specs/002-offline-export/spec.md))

```bash
qd check --offline        # will it work with no network? problems per slide, with hints
qd export --pdf           # export/<deck>.html (self-contained) + .pdf, checked offline
qd pdf --notes            # re-print with speaker notes on separate pages
qd browser                # which Chrome is used (qd can download a pinned one)
```

Export renders a copy of the deck with MathJax (and plotly.js) loaded from
local, SHA-256-verified copies, ships background videos/iframes beside the
HTML, and only replaces the previous export if the offline check passes. The
deck and `_quarto.yml` are never modified. The first export needs the network
once (MathJax; Chrome for Testing if no Chrome is installed).

The editor and the CLI can be used at the same time; edits carry a version
(`--expect`) and are refused rather than lost if the file changed underneath.

## Develop

```bash
cargo test                                      # ~80 tests; Quarto-backed ones skip without Quarto
cargo test --release --test cli -- --ignored    # 100-slide performance check
cargo clippy --all-targets -- -D warnings
```

Built with [spec-kit](https://github.com/github/spec-kit): `/speckit-plan`,
`/speckit-tasks`, `/speckit-implement` from Claude Code. Verification notes are
in [`specs/001-core-editor/checklists/`](specs/001-core-editor/checklists/).

## License

Proprietary, personal use only. See [LICENSE](LICENSE) and
[CLEANROOM.md](CLEANROOM.md).
