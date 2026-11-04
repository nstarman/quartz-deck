# Implementation Plan: Core Editor — Quarto reveal.js GUI + LLM-Friendly CLI

**Branch**: `001-core-editor` | **Date**: 2026-11-03 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/001-core-editor/spec.md`

## Summary

One Rust binary, `qd`, that edits a Quarto reveal.js `.qmd` file in place
using lossless, span-located text patches. Every edit is a CLI command with
JSON output (the LLM interface). `qd serve` runs a localhost server: it
renders the deck with `quarto render` into a cache directory, serves the
output plus a plain-HTML/JS editor shell, watches the file, and pushes
reloads over Server-Sent Events. GUI actions are `POST`s to the same edit
operations the CLI calls. Drag/resize moves boxes client-side for smooth
feedback and writes `.absolute` attributes on release. A bundled Skill
(`qd skill`) teaches agents the loop.

## Technical Context

**Language/Version**: Rust stable (edition 2024, MSRV 1.85; dev machine
1.99.1). Quarto 1.11.9 / Pandoc 3.10 on dev machine.

**Primary Dependencies**: `clap` (CLI), `serde`/`serde_json` (I/O),
`schemars` (`qd schema`), `axum` + `tokio` + `tower-http` (server, SSE,
static files), `notify` (file watch), `sha2` (version tokens), `similar`
(diffs for dry-run), `serde_yaml_ng` (front matter), `thiserror` (errors). UI is plain
HTML/CSS/JS embedded with `include_str!`; reveal.js comes from Quarto's own
render output. Dev: `insta` (snapshots), `assert_cmd` (CLI tests),
`tempfile`. See research.md R8 for license review.

**Storage**: The `.qmd` file (source of truth). Undo journal and render
cache in the OS cache dir (`~/Library/Caches/quartz-deck/<path-hash>/`) —
derived/ephemeral data, never deck state.

**Testing**: `cargo test` — unit tests for the segmenter/patcher,
round-trip corpus tests (`tests/corpus/*.qmd`), `assert_cmd` integration
tests per CLI command, browser checks of the GUI with Playwright via the
agent's browser tooling (manual-run, not CI, in v1).

**Target Platform**: macOS (primary), Linux; any modern browser.

**Project Type**: CLI + local web server (single crate, lib + bin).

**Performance Goals**: edit commands < 200 ms on a 100-slide deck (SC-004);
preview update < 3 s for decks without code cells (SC-005); drag at display
frame rate (SC-006).

**Constraints**: offline; binds `127.0.0.1` only; requires Quarto ≥ 1.4 on
PATH for render/preview/check (edit commands work without it).

**Scale/Scope**: one user, one deck per server, decks up to ~200 slides.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design.*

| Principle | How the plan complies | Status |
|---|---|---|
| I. Clean-room | Only Quarto/reveal.js/Pandoc public docs consulted (research.md cites them). UI designed from scratch. Dependencies are permissive OSS. No slides.com material. | PASS |
| II. `.qmd` is truth | Lossless segmenter keeps byte spans; edits are span replacements; round-trip corpus test is byte-identical. Boxes use stock `.absolute` divs. Cache/undo are derived, not state. | PASS |
| III. CLI-first | GUI calls the same `ops` functions over HTTP that the CLI calls; contract in contracts/cli.md; JSON default; stable error codes; `qd schema`; `qd skill`. | PASS |
| IV. Live browser preview | `qd serve` + SSE reload, preview is real `quarto render` output, keeps current slide via reveal hash. | PASS |
| V. Safe concurrent edits | Every edit: read → hash → patch → re-read & compare → atomic rename; optional `--expect <version>`. | PASS |
| VI. Test-first core | Segmenter/patcher/ops written test-first; corpus round-trip; CLI integration tests. | PASS |
| VII. Simplicity | One crate, one binary, no frontend framework, SSE over WebSockets, `include_str!` over an embed crate. | PASS |

Post-design re-check (after data-model/contracts): PASS — no new
violations; Complexity Tracking empty.

## Project Structure

### Documentation (this feature)

```text
specs/001-core-editor/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── cli.md          # command/JSON/error contract (LLM-facing)
│   └── http.md         # editor server endpoints
└── tasks.md            # /speckit-tasks
```

### Source Code (repository root)

```text
Cargo.toml
src/
├── main.rs             # clap entry → cli::run
├── lib.rs
├── deck.rs             # lossless segmenter: Deck/Slide/Block with spans, IDs
├── attrs.rs            # Pandoc attribute block {#id .class k=v} parse/print
├── ops.rs              # edit operations: &str → Patch (pure, test-first)
├── store.rs            # read/version/atomic write/undo journal
├── cli.rs              # commands, JSON output, error codes, schema
├── render.rs           # quarto render/check wrapper, preview copy
├── server.rs           # axum: static output, editor shell, SSE, POST /op
└── ui/
    ├── index.html      # editor shell (slide list | preview iframe | panel)
    ├── editor.js       # selection, text edit, drag/resize, undo, SSE
    └── editor.css
skills/quartz-deck/SKILL.md   # agent skill (also printed by `qd skill`)
tests/
├── corpus/             # real-world .qmd decks (round-trip)
├── roundtrip.rs
├── ops.rs              # op → expected file snapshots (insta)
└── cli.rs              # assert_cmd per command
```

**Structure Decision**: single crate with lib + bin so tests can call
`ops` directly and the CLI/server stay thin. Modules map 1:1 to the units
in data-model.md.

## Complexity Tracking

No violations.
