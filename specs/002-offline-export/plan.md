# Implementation Plan: Offline Export and PDF

**Branch**: `002-offline-export` | **Date**: 2026-11-04 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/002-offline-export/spec.md`

## Summary

`qd export` renders an **export copy** of the deck (never the deck itself)
whose front matter adds the offline setup from the quarto-revealjs skill —
`embed-resources: true`, the math plugin pointed at `about:blank`, and a header
that loads MathJax 3.2.2 (and plotly.js when needed) from qd's cache — then
copies files that cannot be inlined, runs the **offline check** in headless
Chrome with every non-local request blocked, optionally **prints a PDF**
through reveal's own `?print-pdf` mode, and swaps the result into the export
folder only on success. `qd check --offline` runs the same check on any
rendered deck. Chrome comes from the machine or, failing that, a pinned
Chrome-for-Testing download verified by SHA-256. Every step is a CLI command
with JSON output and goes through `run_op` for the editor.

## Technical Context

**Language/Version**: Rust stable, MSRV 1.89 (unchanged).

**Primary Dependencies (new)**: `headless_chrome` 1.0 (MIT; drives Chrome over
the DevTools protocol: request interception, exceptions, `printToPDF`),
`ureq` (MIT/Apache, HTTPS download of the libraries and browser), `zip`
(MIT, unpack the browser). Existing: `sha2`, `serde_yaml_ng`, `tokio` (not
used by export). See research.md R6 for licenses.

**Storage**: qd cache (`<cache_root>/lib/…`, `<cache_root>/browser/…`) for the
pinned libraries and browser; export folder (default `<deck dir>/export/`)
for the output. The deck and project files are never written.

**Testing**: unit tests for the export copy (front-matter merge, Plotly
renderer injection, offline-incompatible detection, file collection);
integration tests gated on Quarto + a browser (CI's ubuntu runner ships
Google Chrome) for check, export and PDF over a fixture corpus of the known
failure modes; Plotly fixtures gated on Python plotly being installed.

**Target Platform**: macOS (primary), Linux (CI).

**Project Type**: additions to the existing single crate.

**Performance Goals**: export + check + PDF of a 30-slide deck with math and
one Plotly figure < 60 s (SC-004); Quarto render dominates (~2–3 s per deck
measured), Chrome start + check + print ~2–5 s.

**Constraints**: first export needs the network once (libraries; browser if
none installed); later exports work offline. Never modify deck/project files.

**Scale/Scope**: decks up to ~100 slides; one export folder per deck.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design.*

| Principle | How the plan complies | Status |
|---|---|---|
| I. Clean-room | Built from Quarto/reveal.js/MathJax/Chrome DevTools public docs and the user's own quarto-revealjs skill; no slides.com material. | PASS |
| II. `.qmd` is truth | Export renders a temporary copy; the deck, `_quarto.yml` and project files are never written (tested). | PASS |
| III. CLI-first | `qd export`, `qd check --offline`, `qd pdf`, `qd browser` with JSON + stable codes; the editor calls them through `run_op`; Skill + sync test updated. | PASS |
| IV. Live preview | Unchanged; export problems link to preview anchors. | PASS |
| V. Safe writes | Export is built in a staging folder and swapped in only on success (FR-015); no deck writes at all. | PASS |
| VI. Test-first | Tests written before each module; fixtures for every failure mode in the skill's guide. | PASS |
| VII. Simplicity | One new module per concern (`export`, `offline`, `browser`); see Complexity Tracking for the Chrome dependency. | PASS (justified) |

Post-design re-check: PASS — the design adds no deck state and no GUI-only
capability.

## Project Structure

### Documentation (this feature)

```text
specs/002-offline-export/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── cli.md
│   └── http.md
└── tasks.md            # /speckit-tasks
```

### Source Code (repository root)

```text
src/
├── export.rs     # export copy, file collection, staging + swap, orchestration
├── offline.rs    # offline check + PDF via headless Chrome (DevTools protocol)
├── browser.rs    # find Chrome, or download/verify/cache Chrome for Testing
├── assets.rs     # pinned MathJax / plotly.js: locate, download, verify (sha256)
├── cli.rs        # + export / check --offline / pdf / browser commands
├── server.rs     # (unchanged; ops reach export via run_op)
└── ui/           # + Export button, problem list linked to slides
tests/
├── fixtures/offline/   # decks for each failure mode (math, plotly, bg video, iframe, chalkboard, show-notes)
├── export.rs           # export/check/pdf integration (gated on quarto + browser)
└── …                   # existing
```

**Structure Decision**: same single crate; three new modules with one concern
each, called from `cli.rs` like `render.rs` is today.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| `headless_chrome` (~100 transitive crates) | FR-001/020 need request interception, exception capture and `printToPDF` on a running page | Hand-written DevTools client (WebSocket + JSON-RPC) is more code to own; Python+Playwright was rejected by the user (extra runtime); Chrome's `--print-to-pdf` flag prints blank decks (skill finding) and cannot block requests |
| Browser download (`ureq` + `zip`) | FR-042: user chose "installed Chrome or download on first use" | Requiring a manual install fails the "works anywhere" choice; the `headless_chrome` built-in fetcher does not verify checksums (FR-042 requires verification) |
