# Implementation Plan: Source and Run

**Branch**: `004-source-and-run` | **Date**: 2026-11-04 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/004-source-and-run/spec.md`

## Summary

The editor gains a **source pane**: the whole `.qmd` in a resizable split
beside the slides (or full width), highlighted, saving each change with a new
`qd replace` op (unique-match search-and-replace, so edits elsewhere never
block it) and following the slides both ways via line ranges. Code cells get
**Run / Run all / Stop**: each deck has its own **reproducible environment**
— **uv** (`pyproject.toml` + `uv.lock`) by default, **pixi** with PyPI
dependencies as the alternative — and qd runs a Jupyter kernel *inside* it
through a small embedded Python bridge over JSON lines and IPC sockets.
Outputs go to qd's cache (never the `.qmd`), keyed by cell fingerprint so
stale ones are marked; the editor preview stops executing code and shows the
stored outputs instead, while `qd render`/`export` execute for real through
`uv run --locked quarto render`. Agents get `qd env …` and `qd run …` with
JSON; running code requires the editor's session token, so no other page can
trigger it.

## Technical Context

**Language/Version**: Rust stable, MSRV 1.89; editor plain ES modules (no
build); a ~150-line Python kernel bridge embedded in the binary.

**Primary Dependencies**: none new in Rust. External tools: **uv** (default)
or **pixi**; Quarto (unchanged). In each deck's environment: `ipykernel`,
`nbclient`, `nbformat`, `pyyaml` (research R2, verified).

**Storage**: per-deck `pyproject.toml` + `uv.lock` (or pixi manifest + lock)
next to the deck / at the Quarto project root — versioned with the deck;
outputs and run settings in qd's cache (`<cache>/outputs/…`).

**Testing**: unit (env discovery, `replace`, output store, line ranges);
integration gated on `uv` (+ network for the first lock) — `qd env`, `qd run`,
render through the environment; browser tests for the source pane, sync, Run,
and token rejection. CI installs uv and pixi.

**Target Platform**: macOS (primary), Linux (CI).

**Project Type**: additions to the existing crate + editor modules.

**Performance Goals**: source save ≤ 2 s (SC-001); cursor→slide ≤ 0.5 s
(SC-002); warm Run of a short cell ≤ 3 s, figure on slide ≤ 5 s (SC-003);
kernel cold start ~8 s (measured), shown as "starting…".

**Constraints**: running code never changes the `.qmd` (FR-009); code runs
only on explicit action (FR-015); every run/render uses the locked
environment and refuses a stale lock (FR-019); qd never calls `pip`.

**Scale/Scope**: decks up to a few hundred slides / thousands of source
lines; one live kernel per deck.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design.*

| Principle | How the plan complies | Status |
|---|---|---|
| I. Clean-room | Built from public docs (Quarto, Jupyter messaging via `jupyter_client`, uv/pixi docs); the overlay editor and bridge are our own. | PASS |
| II. `.qmd` is truth | Source edits via `qd replace` write only the changed region; runs never touch the `.qmd`; outputs live in the cache; environments are separate files the person versions. | PASS |
| III. CLI-first | `qd replace`, `qd env [init|add|sync]`, `qd run [<cell>|--all] [--restart] [--fresh]`, `qd outputs` with JSON; skill updated; the editor uses the same ops (Run via an authenticated endpoint backed by the same code). | PASS |
| IV. Live preview | Preview shows stored outputs (fast, no kernel); source and slides follow each other. | PASS |
| V. Safe writes | `replace` refuses on missing/ambiguous match; conflict bar as in 003; outputs written atomically to the cache. | PASS |
| VI. Test-first | Tests per module before code; gated integration tests run in CI. | PASS |
| VII. Simplicity | No Rust ZeroMQ/Jupyter crate, no editor framework; a small Python bridge and an overlay textarea. See Complexity Tracking. | PASS (justified) |

Post-design re-check: PASS — the deck file gains nothing; environments are
standard uv/pixi files.

## Project Structure

### Documentation (this feature)

```text
specs/004-source-and-run/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── cli.md      # qd replace, qd env, qd run, qd outputs; line ranges in show/slides
│   └── http.md     # /api/run* (token), /api/outputs, /events run_* events
└── tasks.md        # /speckit-tasks
```

### Source Code (repository root)

```text
src/
├── env.rs        # environment discovery (uv/pixi), init/add/sync, `wrap(cmd)` for running inside it
├── kernel.rs     # kernel bridge process: spawn via env, JSON-lines protocol, interrupt/restart
├── kernel_bridge.py   # embedded Python bridge (jupyter_client, IPC transport)
├── outputs.rs    # output store: fingerprints, stale detection, figures on disk
├── ops.rs        # + replace (unique match)
├── deck.rs       # + line ranges for slides/blocks
├── render.rs     # preview copy: eval:false + stored outputs; render/export through env
├── cli.rs        # + replace, env, run, outputs commands
├── server.rs     # + /api/run* with session token, /api/outputs, run events
└── ui/
    ├── source.js       # (exists: in-place overlay) — unchanged
    ├── pane.js         # source pane: overlay editor, highlighter, save via replace, sync
    ├── run.js          # Run/Run all/Stop controls, output rendering, env status
    └── editor.css / editor.js / index.html  # split layout, wiring
tests/
├── env.rs        # discovery + init/add (gated on uv/pixi)
├── run.rs        # qd run: text, figure, error, state, interrupt, store, staleness (gated)
├── cli.rs        # + replace, line ranges
└── onslide.rs    # + source pane, sync, Run button, token rejection
```

**Structure Decision**: same crate; one module per concern (env, kernel,
outputs) and two editor modules (pane, run).

## Complexity Tracking

| Addition | Why needed | Simpler alternative rejected because |
|---|---|---|
| Embedded Python kernel bridge | Live sessions with state (clarified FR-008a) | A Rust Jupyter/ZeroMQ client is far more code and a native dependency; re-render per Run has no state |
| uv/pixi wrapping of every code-executing command | Reproducible per-deck environments (user direction) | `pip install` into a shared Python is not reproducible |
| Session token on run endpoints | Code must never run because a page asked (FR-015) | Origin checks alone don't cover every browser/CSRF path for an action this powerful |
| uv / pixi as external tools | Per-deck lockfiles (user direction); the constitution's Constraints list Quarto as the only external runtime dependency | Needed only to *run code* or render decks that have code cells; editing, preview and code-free decks work without them, and qd reports how to install them. Record in the constitution's Constraints at the next amendment |
| Overlay source editor | Highlighted, fast full-source editing without a build step | Plain textarea has no highlighting/run gutter; CodeMirror needs a bundle |
