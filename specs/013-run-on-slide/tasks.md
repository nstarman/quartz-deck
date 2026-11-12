---
description: "Tasks for 013 run on the slide"
---

# Tasks: Run on the slide

**Input**: `specs/013-run-on-slide/` (spec.md, plan.md, research.md,
data-model.md, contracts/, quickstart.md)

**Tests**: required (constitution VI, test-first). Each story's tests are
written first and seen failing. Tests that need R or Julia skip when the
tool is missing (`common::have`), and CI installs them.

**Organization**: by user story, in the spec's priority order:
- P1: US1 (▶ on the slide), US2 (Run in the pop-up);
- P2: US3 (the deck's engine: Jupyter kernels and knitr);
- P3: US4 (Observable JS);
- P4: US5 (native Julia).

Paths are relative to the repo root.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on unfinished tasks)
- **[Story]**: US1–US5

---

## Phase 1: Setup

- [X] T001 Add `src/ui/runslide.js` (an empty module, imported by `src/ui/editor.js` and served like the other `/ui/` modules) and `tests/engine.rs` (an empty test file using `mod common;`).

---

## Phase 2: Foundational (blocks every story)

- [X] T002 Write failing tests:
  - **`src/render.rs` unit**: `output_html` puts `data-qd-cell="REF"` on the `qd-output` block (it takes the ref), and the stale marks stay as they are.
  - **`tests/onslide.rs` `cell_output_carries_html`**: in a Python deck with an environment (skip without uv), a run's SSE `cell_output` has `html` with that block, and the preview's block for a cell with stored output carries `data-qd-cell`.
- [X] T003 Pass the cell's ref into `render::output_html` (`src/render.rs`). In the run handler in `src/server.rs`, add `html` to `cell_output`. Update `specs/004-source-and-run/contracts/http.md`'s event list with a pointer to 013's contract. T002 passes.

**Checkpoint**: every run's result can be put into the slide without a re-render.

---

## Phase 3: User Story 1 — Run a cell from the slide (P1) 🎯 MVP

**Independent Test**: spec US1; quickstart §1, §4.

- [X] T004 [P] [US1] Write failing browser tests in `tests/onslide.rs`:
  - **`run_from_slide`**:
    - hovering a cell shows a ▶ named "Run cell REF";
    - clicking it gives the cell `qd-running`;
    - then the slide's `qd-output[data-qd-cell=REF]` has the new output before the preview re-renders (the frame's `document` is the same object), and the current slide is unchanged;
    - ⇧Enter with the cell selected runs it too.
  - **`run_states`**:
    - a cell that raises gets `qd-error`, with the traceback in its block;
    - editing a cell's code in the source pane gives `qd-stale`;
    - ■ on a running cell (`time.sleep(30)`) stops it, giving `qd-stopped`.
  - **`no_run_without_token`**:
    - `/present/` and the editor opened without `#t=` show no ▶;
    - `POST /api/run` without the token is 403 (already true; the test pins it).
- [X] T005 [US1] Build the overlay in `src/ui/runslide.js`:
  - **The ▶**: on the hovered or selected cell, or a running one (positioned like 003's hover bar). It runs through `run.runCells([ref])`, and becomes ■ while running (`/api/run/interrupt`).
  - **State classes** on the cell element, from `run_started`, `cell_output` and `run_done`, and stale from `/api/outputs`.
  - **The in-place swap** of `qd-output[data-qd-cell]`, or an insert after the cell when it had none.

  Also:
  - `src/ui/actions.json`: `run` gets the key `Shift+Enter` where an element is selected; add `run-stop-cell`.
  - `src/ui/actions.js`: the element menu's `run-stop-cell`, visible while that cell runs.
  - `src/ui/editor.css`: the ▶ and the state styles.

  T004 passes.

**Checkpoint**: US1 works alone for Python decks.

---

## Phase 4: User Story 2 — Change the code and see the result, on the slide (P1)

**Independent Test**: spec US2; quickstart §2.

- [X] T006 [P] [US2] Write failing browser tests in `tests/onslide.rs`:
  - **`popup_run`**:
    - the pop-up on a code cell shows "Run ⇧Enter", and a math pop-up doesn't;
    - after changing the code, ⇧Enter writes the deck (one undo step, Edit ▸ Undo returns it), runs the cell, and the slide's output updates while `#srcedit` stays open;
    - unchanged text runs without writing (the deck's version is the same).
  - **`popup_run_refused`**: changing the file underneath first (a CLI edit to the same cell) makes Run show the conflict in the pop-up's error line, run nothing (no `run_started`), and keep the typed text.
  - **`run_ordering`**: two quick runs of a cell whose output is the run's own counter end with the second run's output on the slide.
- [X] T007 [US2] In `src/ui/source.js`, add `openSource(rect, text, {save, cancel, run?})`: a **Run ⇧Enter** button and the key when `run` is given. Then:
  - in `src/ui/editor.js`, pass `run` for code cells: save through the existing path, then `runCells([ref])` only if the save succeeded;
  - in `src/ui/runslide.js`, keep the newest run id per cell and ignore older `cell_output`s (FR-007).

  T006 passes.

**Checkpoint**: P1 complete (MVP: one PR, US1 + US2).

As built:
- **No `data-qd-cell` attribute.** Each cell's output block already sits
  inside its wrapper, `[data-qd="REF"]`, so the editor finds the block
  there. `render::output_div` is the bare block, and it is what
  `cell_output.html` carries.
- **Placeholder for invisible cells.** Revealjs hides code by default
  (`echo: false`), so a cell that has never run showed nothing on the slide,
  and there was nothing to point at. The preview now marks cell wrappers
  `.qd-cell`, and in the editor (not when presenting) an empty one shows
  "Code cell, not run yet".
- **The ▶** sits at the cell's top right for the hovered or selected cell.
  The element bar's Run button for a selected cell stays as it was.
- **⇧Enter** is the existing `run-cell` action, now also where an element is
  selected. It runs the source pane's caret cell only when the key comes
  from the source pane. No `run-stop-cell` action was needed: the ▶ turns
  into ■, and Run ▸ Stop still works.
- **Order of runs (FR-007).** `run_cells_over_http` fires five runs back to
  back and checks that they report in the order asked, and that the last
  one is stored. It passed every time without changes: the session takes
  them in order. So there is no queue in the server and no guard in the
  editor.

---

## Phase 5: User Story 3 — Run cells with the deck's own engine (P2)

**Independent Test**: spec US3; quickstart §3.

- [X] T008 [US3] CI: add R to `.github/workflows/ci.yml` (`r-lib/actions/setup-r@v2`, then `install.packages(c("knitr", "rmarkdown"))`). Probe research R4 there: run a scratch `kernel_bridge.R` in a workflow step that prints the start time, a plot's pixel size at `fig-width` 10 × 5 and 96 dpi, a variable carried between two `execute`s, and the reply to SIGINT during `Sys.sleep(30)`. Record the results in research.md R4.
- [X] T009 [P] [US3] Write failing tests:
  - **`src/engine.rs` units**:
    - parse `quarto inspect` JSON into `Engine` (`knitr`, `jupyter`, `markdown` → `ojs` or `none`), with `fig` from `formats.revealjs.execute`;
    - the kernel name from `jupyter: NAME` and from `jupyter: {kernelspec: {name}}`;
    - the cache key changes with the front matter and with the cell languages, and not with cell bodies.
  - **`tests/engine.rs`**:
    - **`named_kernel`**: a `qd-test` kernelspec in a temp `JUPYTER_PATH` (`kernel.json`: the usual `python -m ipykernel_launcher -f {connection_file}` argv, plus `"env": {"QD_TEST_KERNEL": "yes"}`). A deck with `jupyter: qd-test` runs a cell that prints `os.environ["QD_TEST_KERNEL"]`, and `qd run` reports `"engine": {"kind": "jupyter", "kernel": "qd-test"}`.
    - **`missing_kernel`**: `jupyter: nosuch` gives `E_ENGINE`, with Quarto's list of known kernels in the message.
    - **`knitr_cells`** (skips without `Rscript`):
      - three dependent `{r}` cells run one by one with `qd run`, and the third sees the first's variable;
      - a `plot(1:10)` cell gives a PNG of 960×480 px;
      - `--timeout 2` on `Sys.sleep(30)` gives `stopped`.
    - **`no_r`**: with `PATH` lacking R, an `{r}` deck's `qd run` is `E_ENGINE`, and the hint says to install R.
  - **`tests/onslide.rs` `run_knitr_from_slide`** (skips without R): the ▶ on an `{r}` cell shows its plot on the slide, and the badge reads `R · idle`.
- [X] T010 [US3] Implement:
  - **`src/engine.rs`**: `Engine`, `detect(deck_path, deck)` through `quarto inspect`, cached under `<cache>/engine/`, and `command(engine, env)`, the session process for each engine:
    - Python: the deck's environment, as 004;
    - other Jupyter decks: `uv run --no-project --with jupyter_client python -I`;
    - knitr: `R --no-echo --no-restore --no-save` with `kernel_bridge.R` on stdin bootstrapped from `-e`.
  - **`src/kernel.rs`**:
    - `Session::start(cmd)`;
    - `cell_code` accepts any executable language, and the error is `E_ENGINE` only when the language doesn't fit the engine;
    - interrupt sends SIGINT for R.
  - **`src/kernel_bridge.py`**: the kernel by name, or by language through `KernelSpecManager`.
  - **`src/kernel_bridge.R`**: new (research R4).
  - **`src/server.rs`**: one session per engine, reset when the engine changes, and `engine` in the `kernel` event and in `/api/outputs`.
  - **`src/cli.rs`**: `engine` in the `qd run` result.
  - **`src/error.rs`**: the `E_ENGINE` hint names what is missing.
  - **`src/ui/run.js`**: the badge shows the engine's label.

  T009 passes.

**Checkpoint**: Jupyter kernels and knitr run from the slide, the source pane and `qd run`.

As built:
- **When qd asks Quarto.** A deck of Python cells with no `engine:` or
  `knitr:` key runs as in 004 without calling `quarto inspect`, so Python
  decks stay as fast as before and `qd run` on them needs no Quarto. A
  `jupyter:` key with Python cells is read from the front matter directly.
  Every other deck asks Quarto once per front matter and set of languages
  (cached).
- **A missing kernel.** For a named kernel that isn't installed, the bridge
  reports it with the kernels it found. A deck that went through `quarto
  inspect` gets Quarto's own message instead.
- **The R probe.** It was the `knitr_cells` test running in CI, plus a step
  that prints its timing. There was no separate scratch script. The results
  are in research R4.

---

## Phase 6: User Story 4 — Observable JS cells (P3)

**Independent Test**: spec US4; quickstart §5.

- [ ] T011 [US4] Probe research R9 in the browser pane: find how Quarto's OJS runtime on the preview page redefines one cell from new source, and whether it re-runs the cells that depend on it. Record the entry point, or that there is none, in research.md R9.
- [ ] T012 [P] [US4] Write a failing browser test, `run_ojs_from_slide`: an `{ojs}` cell `x = 1` and a dependent `x * 2`. Change the first to `x = 5` in the pop-up and press ⇧Enter: the dependent cell shows 10, and no `/api/run` request was made.
- [ ] T013 [US4] In `src/ui/runslide.js`, give `{ojs}` cells a ▶ that redefines the cell in the page through the entry point from T011, or else re-renders the preview and keeps the slide. In `src/engine.rs`, `ojs` decks need no session. T012 passes.

---

## Phase 7: User Story 5 — Quarto's native Julia engine (P4)

**Independent Test**: spec US5; quickstart §5.

- [ ] T014 [US5] CI: add Julia (`julia-actions/setup-julia@v2`). Probe research R10 there: `quarto inspect` on an `engine: julia` deck, and a scratch `kernel_bridge.jl` that keeps state, captures a plot's `image/png` and answers SIGINT. Record the results in research.md R10.
- [ ] T015 [P] [US5] Write failing tests in `tests/engine.rs`, **`julia_cells`** (skips without `julia`): an `engine: julia` deck runs three dependent cells, keeping state, and a missing Julia gives `E_ENGINE` with the install hint.
- [ ] T016 [US5] Add `src/kernel_bridge.jl` and the `julia` kind in `src/engine.rs` (the command with the deck's `--project` when the front matter names one). T015 passes.

---

## Phase 8: Polish & cross-cutting

- [ ] T017 [P] Docs:
  - README: Run on the slide, and engines;
  - the qd skill: `qd run`'s `engine` and the `E_ENGINE` cases;
  - `specs/ROADMAP.md`: 013's status;
  - the 007 action inventory: "Added by 013".
- [ ] T018 Run quickstart §1–§5 in the browser pane, and record it in `specs/013-run-on-slide/checklists/quickstart-run.md`.
- [ ] T019 Final constitution re-check in `plan.md`. fmt, clippy, `cargo test --no-fail-fast` and `node --test vscode/test/*.test.js` green.
- [ ] T020 Concision audit (`/pr-audit`), the last task before each PR is marked ready.

---

## Dependencies & Execution Order

- **Setup (T001) → Foundational (T002–T003)** → every story.
- **US1 (T004–T005)** after Foundational.
- **US2 (T006–T007)** after US1. The pop-up's run reuses the overlay's state and its ordering.
- **US3 (T008–T010)** after Foundational. It is independent of US1 and US2 apart from `run_knitr_from_slide`, which needs the ▶. T008 (CI and the probe) comes first.
- **US4 (T011–T013)** after US1 and US2 (the ▶ and the pop-up's Run).
- **US5 (T014–T016)** after US3 (`engine.rs` and the generalised `Session`).
- **Polish** after the stories it documents.

Within each story: tests first (seen failing), then Rust, then the server, then the editor.

## Parallel opportunities

- T004 (US1 tests) ∥ T009's units (US3): different files.
- T008 (CI R probe) can start alongside US1.
- T012 (US4 test) ∥ T015 (US5 test).

## Implementation strategy

1. **MVP:** Setup, Foundational, US1 and US2: Run on the slide and in the pop-up, for Python. One PR.
2. **P2:** US3, Jupyter kernels and knitr, with R in CI. One PR.
3. **P3/P4:** US4, Observable JS, then US5, native Julia. One PR each, in that order (clarification).
4. **Polish:** docs, the quickstart run, the re-check. One PR.

Each PR passes the concision audit (T020) before it is marked ready, and is
merged only after the person approves it.
