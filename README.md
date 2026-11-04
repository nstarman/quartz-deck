# quartz-deck

A local GUI + CLI editor for [Quarto](https://quarto.org) reveal.js
presentations, written in Rust.

- The `.qmd` file is the source of truth. The GUI and the CLI both edit it
  with minimal text patches, so it stays readable and diff-friendly.
- `qd serve deck.qmd` opens an editor in your browser: slide list, live
  reveal.js preview, drag/resize boxes, properties panel.
- Every editing action is also a CLI command with JSON output, so an LLM
  (e.g. Claude Code) can edit the deck in-session and watch the same
  browser preview.

Status: **CLI editing (US1) and live browser preview (US2) work.** GUI
editing (click-to-edit, drag/resize boxes) is next. See [`specs/`](specs/) and the
[constitution](.specify/memory/constitution.md).

## Install & try

```bash
cargo install --path .
qd new demo/deck.qmd --title "Demo" && cd demo
qd add --title "Results" --md $'- one\n- two'
qd --human slides
qd serve --open                                   # live preview in the browser
qd skill --install ~/.claude/skills/quartz-deck   # teach Claude Code to use qd
```

`qd schema` lists every command; `cargo test` runs the suite.

Built with [spec-kit](https://github.com/github/spec-kit): run
`/speckit-plan`, `/speckit-tasks`, `/speckit-implement` from Claude Code.

## License

Proprietary, personal use only. See [LICENSE](LICENSE) and
[CLEANROOM.md](CLEANROOM.md).
