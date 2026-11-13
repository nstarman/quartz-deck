# Implementation Plan: Run on the slide

**Branch**: `013-run-on-slide` | **Date**: 2026-11-12 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/013-run-on-slide/spec.md`

## Summary

Code cells become runnable where they are shown.
- **The ▶ on the slide**: the editor's overlay puts a ▶ on each cell in the
  preview. The cell shows its state (running, error, stopped, stale), and
  when a run ends, its output block on the slide is swapped in place, using
  the HTML the server already renders for the preview.
- **The source pop-up**: on a code cell it gets **Run ⇧Enter**. Run saves the
  edit as an ordinary qd edit, runs the cell and stays open.
- **Engines**: qd asks Quarto which engine the deck uses (`quarto inspect`,
  cached per front-matter change) and runs cells in a live session for that
  engine:
  - a Jupyter kernel (004's bridge, now with any named or matching kernel,
    and hosted by uv when the deck has no Python environment);
  - one R process running `evaluate` for knitr decks.

  Both speak 004's protocol, so Run all, Stop, Restart, stored outputs and
  `qd run` stay one code path.
- **Later stories**: Observable JS cells (evaluated in the page) and Quarto's
  native Julia engine come next, in the order the clarification set.

## Technical Context

**Language/Version**: Rust stable; plain ES modules for the editor; an embedded R script (new) beside the embedded Python bridge
**Primary Dependencies**: no new crates. At run time:
- `quarto` (as before; `quarto inspect` for the engine);
- `uv` (004) to host the Jupyter bridge for non-Python decks;
- the person's R with knitr and rmarkdown, which bring `evaluate` and `jsonlite`, for knitr decks;
- later, the person's Julia.

**Storage**: unchanged: stored outputs beside the deck (004). The engine
answer is cached in qd's cache (`<cache>/engine/<hash>.json`).

**Testing**: `cargo test`:
- units for engine detection and parsing `inspect` output;
- CLI tests for `qd run` with a named test kernelspec (temp `JUPYTER_PATH`),
  and with R where `Rscript` exists;
- browser tests for the ▶, the states, the pop-up's Run and the order of
  results.

CI installs R. Julia comes with US5.

**Target Platform**: macOS first, Linux
**Project Type**: CLI plus a localhost web editor (single binary)
**Performance Goals**: a sub-second cell's new output on the slide within 2 s of the click (SC-002); the engine decided in under 50 ms after the first `inspect` (cached)
**Constraints**:
- code runs only on the person's click or key, with the session token (FR-008);
- nothing is written into the `.qmd` except the person's saved edit (SC-006);
- the presented deck and exports are unchanged;
- R and Julia are the person's own installs (clarification).

**Scale/Scope**:
- an R bridge (about 80 lines);
- engine detection (about 80 lines);
- `Session::start` generalised;
- the overlay ▶ and states (about 120 lines of JS);
- the pop-up's Run;
- for US4 and US5, an OJS hook and a Julia bridge.

External tools: Quarto 1.11 (as CI pins), uv (004), and for knitr decks R ≥
4.1 with knitr and rmarkdown. No new crates, so no licence changes.

## Constitution Check

| Principle | Check |
|---|---|
| I. Clean room | Built from Quarto's documented CLI (`inspect`), Jupyter's documented client protocol (004) and R's `evaluate`/`png`. No code copied from Quarto's engines. Pass. |
| II. `.qmd` is the truth | Runs never write the deck (FR-009). The pop-up's save is an ordinary edit. Outputs stay beside the deck. Pass. |
| III. CLI first | `qd run` gets the same engine choice and reports it (`engine` in the result). The editor's ▶ calls the same `/api/run` that `qd run` uses when a server is running. Pass. |
| IV. Live preview | The output block is swapped in place, then the preview re-renders as in 004. The current slide and selection are kept. Pass. |
| V. Safe concurrent edits | The pop-up's Run saves with `--expect`, and a refused save runs nothing (FR-006). Older results never replace newer ones (FR-007). Pass. |
| VI. Test first | Each story's tests come first: engine-detection units, a named-kernel CLI test, R tests (skipped without R; run in CI), and browser tests for the ▶, states, pop-up and ordering. Pass. |
| VII. Simplicity | One protocol and one `Session` type for every engine, one output renderer, Quarto's own engine choice. No new crates. Pass. |
| Constraints: code runs only on an explicit action, with the token | ▶ is editor-only overlay; `/api/run` already requires the token; `/present/` has no ▶ (SC-005 test). Pass. |
| Constraints: per-deck locked Python environments | Unchanged for Python. R and Julia use the person's install, as clarified; locking them is a later spec. Pass, noted in Complexity Tracking. |

## Project Structure

### Documentation (this feature)

```text
specs/013-run-on-slide/
├── plan.md, research.md, data-model.md, quickstart.md
├── contracts/cli.md, contracts/http.md, contracts/ui.md
├── checklists/requirements.md
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
src/
├── engine.rs        # NEW: Engine {kind, kernel, language, fig}; detect() via `quarto inspect`,
│                    #      cached by front matter + cell languages; the session command per engine
├── kernel.rs        # Session::start(cmd) (any engine); cell_code() takes any executable language;
│                    #      interrupt by message (Python) or SIGINT (R)
├── kernel_bridge.py # kernel chosen by name or by language (KernelSpecManager)
├── kernel_bridge.R  # NEW: JSON lines over stdin/stdout; evaluate::evaluate in globalenv; png plots
├── render.rs        # output_html(): data-qd-cell="<ref>" on the block
├── server.rs        # session per engine; cell_output gains `html`; kernel event gains `engine`
├── cli.rs           # qd run: engine choice, `engine` in the result
└── error.rs         # E_ENGINE hint names what is missing
src/ui/
├── runslide.js      # NEW: ▶ overlay on preview cells, states, in-place output swap, run ordering
├── source.js        # openSource(..., {run}): Run ⇧Enter in the pop-up's bar
├── editor.js        # wiring; the pop-up's run for code cells
├── run.js           # badge shows the engine (Python/R/Julia); shared run ids
├── actions.js, actions.json   # `run` gets ⇧Enter on a selected cell; `run-stop-cell`
└── editor.css       # ▶ and state styles
tests/
├── engine.rs        # NEW: detection, named kernel (temp JUPYTER_PATH), R (skips without Rscript)
└── onslide.rs       # run_from_slide, run_states, popup_run, run_ordering, no_run_without_token
.github/workflows/ci.yml   # R + knitr + rmarkdown (US3); Julia (US5)
```

**Structure Decision**: one new Rust module (`engine.rs`) and one new editor
module (`runslide.js`), next to 004's run code. Everything else extends files
that already own the behaviour.

## Re-check after building (T019)

| Principle | As built |
|---|---|
| I. Clean room | Built from Quarto's `inspect` command, `jupyter_client`, R's `evaluate`, and Base Julia. The OJS ▶ uses only a page reload; nothing calls into Quarto's runtime. Pass. |
| II. `.qmd` is the truth | Runs never write the deck; the pop-up's Run saves as one ordinary edit. Outputs stay in qd's cache. The placeholder and state marks exist only in the editor's preview. Pass. |
| III. CLI first | `qd run` uses the same engine choice and reports `engine`. Errors name what's missing, including an `{ojs}` cell asked for from the CLI. The editor calls the same `/api/run`. Pass. |
| IV. Live preview | Outputs are swapped in place, on the same slide. A new render loads behind an open source pop-up, but still waits while text is typed on the slide. Pass. |
| V. Safe concurrent edits | The pop-up's Run saves with the element check, and a refused save runs nothing. Five quick runs report in the order asked, and the last one asked is stored. Pass. |
| VI. Test first | Tests were written first: units, `tests/engine.rs`, and browser tests for the ▶, states, pop-up, ordering, OJS, R, and no token. R and Julia tests run in CI. The two pop-up tests weren't run before their code (noted in PR #56). Pass. |
| VII. Simplicity | One protocol and one `Session` for Jupyter, R and Julia; one output renderer (`render::output_div`); Quarto's own engine choice. No new crates. OJS uses a reload instead of re-running one cell. Pass. |
| Constraints: code runs only on an explicit action, with the token | ▶ is editor-only. `/api/run` needs the token. The pop-up's Run now requires a real key press or click (fixed in the quickstart run). Pass. |

Quickstart run: `checklists/quickstart-run.md`. Three fixes came out of it:
- the pop-up's trusted-input check;
- the OJS message from the CLI;
- the OJS placeholder text.

Not built:
- R and Julia environments aren't locked (a later spec, as clarified);
- front-matter `julia: exeflags` aren't passed;
- figure size isn't applied to Julia plots.

## Complexity Tracking

| Item | Why needed | Simpler alternative rejected because |
|---|---|---|
| A second bridge in R (and later Julia) | knitr decks have to run R the way knitr does, with state between cells (FR-012) | IRkernel through Jupyter needs an extra install most R users lack. `Rscript` per run loses state |
| R and Julia not locked per deck | the clarification: the person's own install now, locking later | locking now (renv, Julia projects) would double `qd env`'s tools before anyone has asked for it |
