---
description: "Task list for 004-source-and-run"
---

# Tasks: Source and Run

**Input**: Design documents from `specs/004-source-and-run/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/cli.md, contracts/http.md, quickstart.md

**Tests**: Required (constitution VI). Write each test task first and see it
fail before the implementation task after it. Tests needing **uv** (+ network
for the first lock), **pixi**, Quarto or Chrome skip with a message when the
tool is missing; CI installs all of them.

**Organization**: P1: US1, US2, US3 (with the environments it needs). P2: US4.
P3: US5. Paths relative to repo root.

## Format: `[ID] [P?] [Story] Description`

---

## Phase 1: Setup

- [X] T001 [P] CI in `.github/workflows/ci.yml`: install **uv** (official installer) and **pixi**; keep Python for existing Plotly tests; add the gated tests to the normal `cargo test` step — moved to the environments phase (audit: nothing in the MVP uses uv/pixi); add it with T015
- [X] T002 [P] Fixtures `tests/fixtures/run/`: `basic.qmd` (Python cell printing a value, a matplotlib figure cell, a cell using a variable from the first, a failing cell `1/0`, an `eval: false` cell, an R cell), `slow.qmd` (a `while True` cell), `nocode.qmd`; `README.md` per file — done
- [X] T003 Module skeletons wired into `src/lib.rs`: `src/env.rs`, `src/kernel.rs`, `src/outputs.rs`, `src/kernel_bridge.py` (embedded with `include_str!`); editor modules `src/ui/pane.js`, `src/ui/run.js` added to the `/ui/<name>` table in `src/server.rs` — done for the MVP (`pane.js` served); `env.rs`, `kernel.rs`, `outputs.rs`, `kernel_bridge.py`, `run.js` are created by their phases rather than as empty files

---

## Phase 2: Foundational (blocks every story)

- [X] T004 [P] Write failing tests in `tests/cli.rs`: `qd show`/`qd slides` report `lines: [start, end]` (1-based, inclusive) for slides and blocks, correct across front matter, CRLF files and fenced code containing `##`; code blocks report `lang`, `fingerprint` (contracts/cli.md) — done (`show_reports_line_ranges_and_code_cells`, `line_ranges_count_crlf_lines`)
- [X] T005 Implement line ranges in `src/deck.rs` (from spans) and code-cell fields (`lang` from the fence `{python}`, `#|` options, fingerprint = SHA-256 16 hex of the block text) in `src/cli.rs` JSON — T004 passes — done: `Deck::lines`; `lang`, `exec` (cell vs. shown code), `fingerprint`
- [X] T006 [P] Write failing tests in `tests/cli.rs` for `qd replace --old --new` (contracts/cli.md): unique match replaced byte-exactly; zero matches → `E_CONFLICT`; two matches → `E_AMBIGUOUS`; replacement may add a slide; undo restores bytes; `--old-file`/`--new-file`/stdin; via `run_op` (`replace` op) — done (`replace_swaps_one_unique_occurrence`, incl. the API refusing file/stdin input)
- [X] T007 Implement `ops::replace` in `src/ops.rs` and `qd replace` in `src/cli.rs` (add to `API_OPS`, `qd schema`, skill) — T006 passes — done; CRLF decks: `\n` from the browser is matched as the deck’s line ending; `/api/op` refuses `old-file`/`new-file` and `-` (stdin)

**Checkpoint**: line ranges and `qd replace` exist; stories can start.

---

## Phase 3: User Story 1 — Edit the whole source beside the slides (P1) 🎯 MVP

**Goal**: the whole `.qmd` in a highlighted, editable pane; saves are minimal and conflict-safe.

**Independent Test**: spec US1 Independent Test; quickstart §2 (first half).

- [X] T008 [P] [US1] Write failing browser tests in `tests/onslide.rs`: Source split opens beside the slides with the whole file (front matter included); typing a word in a paragraph and a YAML value, then pausing, saves only those bytes (file diff) and the slides re-render; ⌘Z restores identical bytes; full-width toggle; an agent's `qd edit` elsewhere appears in the pane without losing the cursor or unsaved text; same-region collision shows both versions (conflict bar); a save leaving a fence open keeps the last good render with the error — done (`source_pane_saves_minimal_changes`)
- [X] T009 [US1] Implement `src/ui/pane.js` (research R8): textarea over a highlighted `<pre>` (front matter/YAML, headings, fenced divs, code cells and `#|` options, emphasis, links), visible-lines highlighting per frame; debounced save (pause ≥ 700 ms or blur) computing the changed region expanded to whole lines + one context line each side → `replace` op (widen and retry on `E_AMBIGUOUS`; conflict bar on `E_CONFLICT`); merge external changes into the pane by the same region logic keeping cursor and unsaved text — T008 typing/conflict parts pass — done: textarea over highlighted `<pre>`; saves after 700 ms / blur / ⌘S as one `replace` of the changed lines + 1 context line (widened on `E_AMBIGUOUS`); line-granular 3-way merge with changes from elsewhere; same-line edits → conflict bar (Use mine / Keep theirs); `GET /api/source`
- [X] T010 [US1] Layout in `src/ui/index.html`/`editor.css`/`editor.js` (FR-006): resizable split (drag handle) between slides and source, Source toggle (replaces the old hidden form panel button; the form stays reachable inside the pane's ⋯ menu), full-width button; layout in `localStorage` (try/catch) — remaining T008 parts pass — done: resizable split (drag or ←/→ on the divider), Source toggle, Full width, Form button for the old panel; all remembered per browser

---

## Phase 4: User Story 2 — Source and slides follow each other (P1)

**Goal**: cursor → slide; slide element → source.

**Independent Test**: spec US2 Independent Test; quickstart §2 (second half).

- [X] T011 [P] [US2] Write failing browser tests: cursor moved into slide 3's text shows slide 3 within 0.5 s and keeps focus in the pane; selecting or editing an element on slide 5 scrolls the pane to its line range and highlights it — done (`source_and_slides_follow_each_other`)
- [X] T012 [US2] Implement in `src/ui/pane.js`/`editor.js` using `/api/deck` line ranges (research R10): caret line → slide anchor → `go()` without stealing focus; `select()`/edit sessions → `pane.reveal(lines)` with a highlight band — T011 passes — done: caret line → slide (front matter → title slide), focus stays in the pane; element selection / edit session → highlighted line band

**Checkpoint**: the source-pane MVP (US1 + US2) is usable without any environment.

---

## Phase 5: Environments for running code (FR-017–FR-020; blocks US3–US5)

**Goal**: each deck has its own locked uv (or pixi) environment; everything that executes code goes through it.

- [X] T013 [P] Write failing unit tests in `src/env.rs` (`#[cfg(test)]`): discovery from deck folder up to the Quarto project root — `pixi.toml` / `pyproject.toml [tool.pixi]` → pixi, `pyproject.toml` / `uv.lock` → uv, none → `E_NO_ENV`; `wrap(cmd)` builds `uv run --project <root> --locked …` / `pixi run --manifest-path <root> --locked …`; tool missing → `E_ENV_TOOL` with an install hint
- [X] T014 Implement discovery and `wrap` in `src/env.rs`; new error codes `E_NO_ENV`, `E_ENV_TOOL`, `E_LOCK_STALE`, `E_ENV` in `src/error.rs` — T013 passes
- [X] T015 [P] Write failing integration tests `tests/env.rs` (gated on uv; pixi variants gated on pixi): `qd env init` creates `pyproject.toml` + `uv.lock` with `ipykernel`, `nbclient`, `nbformat`, `pyyaml` (research R2); `qd env add matplotlib` updates manifest + lock; `qd env` reports `lock_current`; editing the manifest by hand → `E_LOCK_STALE` on run/render; `qd env sync` in a fresh copy reproduces the same locked versions (SC-007); `--tool pixi` creates a pixi manifest with PyPI dependencies only
- [X] T016 Implement `qd env`, `qd env init [--tool uv|pixi]`, `qd env add`, `qd env sync` in `src/env.rs`/`src/cli.rs` (shell out to uv/pixi; never pip) — T015 passes
- [X] T017 [P] Write failing test in `tests/env.rs`: a deck with Python cells renders through its environment (`qd render` executes the cells: printed value and figure in the HTML; research R3 verified flow); a deck without code cells renders without any environment
- [X] T018 Route code-executing Quarto calls through `env::wrap` in `src/render.rs` (`render`, `export`, `check --render`) when the deck has code cells — T017 passes; existing export/render tests still pass

---

## Phase 6: User Story 3 — Run a code cell and see its output (P1)

**Goal**: Run / Run all / Stop in the editor; outputs under the cell and on the slide; `.qmd` untouched.

**Independent Test**: spec US3 Independent Test; quickstart §3.

- [X] T019 [P] [US3] Write failing tests for the bridge in `tests/run.rs` (gated on uv): `kernel::Session` starts the embedded bridge in the deck's environment over IPC; executing the fixture cells returns text `42`, an image file, `43` (state kept), and an error with traceback; interrupt stops `slow.qmd`'s cell; restart clears state
- [X] T020 [US3] Implement `src/kernel_bridge.py` (jupyter_client, `transport="ipc"`, JSON lines: execute/interrupt/restart/shutdown, output normalisation per research R4) and `src/kernel.rs` (spawn via `env::wrap`, request/response, timeouts, child cleanup) — T019 passes
- [X] T021 [P] [US3] Write failing unit tests in `src/outputs.rs`: store/load outputs per deck + fingerprint, images written as files, text truncated at 2,000 lines with a side file, atomic writes, stale = fingerprint mismatch
- [X] T022 [US3] Implement `src/outputs.rs` — T021 passes
- [X] T023 [P] [US3] Write failing tests in `tests/serve.rs`: `POST /api/run` without or with a wrong `X-QD-Token` → 403 and no kernel starts; with the token → 202 and SSE `kernel`, `run_started`, `cell_output`, `run_done` events; `run_op` refuses `run`; `/api/outputs` returns stored outputs; `/outputs/<file>` serves figures
- [X] T024 [US3] Implement in `src/server.rs`: session token (random, in the page and the session file with user-only permissions), `/api/run`, `/api/run/interrupt`, `/api/run/restart`, `/api/outputs`, `/outputs/*`, events; one `kernel::Session` per server, started on first run; live mode by default, per-deck fresh mode setting in the cache (FR-008a/b); Run all in order skipping `eval: false`, stopping at the first error (FR-010); non-Python cells → `E_ENGINE` — T023 passes
- [X] T025 [P] [US3] Write failing test in `tests/cli.rs`/`tests/serve.rs`: the preview copy renders code cells with `eval: false` and shows each cell's stored outputs (text, image, error) after it, stale ones marked; the preview never starts a kernel or needs an environment; `.qmd` byte-identical after runs (FR-009)
- [X] T026 [US3] Implement stored outputs in `render::preview_copy` (research R7): `execute: {eval: false, echo: true}` + per-cell raw-HTML output blocks; re-render after `run_done` — T025 passes
- [X] T027 [P] [US3] Write failing browser tests: Run on the print cell shows `42` under it in the pane and on its slide; figure cell shows the image on the slide within 5 s (warm); error cell shows the traceback; Stop interrupts the slow cell; Run all stops at the failing cell and marks it; outputs carry their run number; `.qmd` unchanged
- [X] T028 [US3] Implement `src/ui/run.js`: Run buttons in the pane's gutter beside each code cell and on slide code cells, Run all / Stop / Restart / Restart & run all, kernel state badge ("starting…"), outputs rendered under cells in the pane (text, HTML, images, tracebacks; truncation with "show all"), env status with "Create environment" / "Add package…" (via token-guarded `/api/env*`) — T027 passes

**Deviations (US3)**: outputs show in a dock below the source (the cell under the caret, or the one just run) rather than inline under each cell — a textarea overlay can't open space between lines; slides show them inline. Figures are written to files by the bridge (no base64 in Rust). The "re-run from top" mode is a per-request `fresh` flag (the **Restart & run all** button) rather than a cached per-deck setting; add the setting with US4 if `qd run` needs it. The output store is one JSON file per deck keyed by cell ref (data-model's per-fingerprint folders were more than needed). The preview renders with `quarto --no-execute`.

**Checkpoint**: P1 complete — source pane, sync, and running code in a reproducible environment.

---

## Phase 7: User Story 4 — Agents run code through the CLI (P2)

**Goal**: `qd run`, `qd outputs` with JSON; shared with the open editor.

**Independent Test**: spec US4 Independent Test; quickstart §4.

- [X] T029 [P] [US4] Write failing tests in `tests/run.rs`: with no server, `qd run results/1` runs a one-shot fresh session and returns JSON (contracts/cli.md) with an image `path`; `qd run --all` stops at the first error (exit 1, `E_RUN`); `qd outputs` lists stored outputs with `stale`; `E_NOT_CODE`, `E_ENGINE`; with `qd serve` running, `qd run` uses its session (variables from an editor run are visible) and the editor receives the outputs
- [X] T030 [US4] Implement `qd run` / `qd outputs` in `src/cli.rs` (server path with the session token from the session file; one-shot path via `kernel::Session`) — T029 passes
- [X] T031 [US4] Skill `skills/quartz-deck/SKILL.md`: environment (`qd env init/add/sync`, lockfiles are part of the deck), `qd run`, `qd outputs`, `qd replace`; skill sync test passes

**Deviations (US4)**: tests in `tests/run_cli.rs` (the session test in `tests/run.rs` sets process env). No `--restart` flag: `--fresh` covers it. With a server, `qd run` posts with `"wait": true` and the server answers when the run ends; the plan and the run loop are shared (`kernel::plan`, `kernel::run_plan`), so both paths report the same entries.

---

## Phase 8: User Story 5 — Outputs persist between sessions (P3)

**Goal**: last outputs shown on reopen; stale ones marked.

**Independent Test**: spec US5 Independent Test.

- [X] T032 [US5] Write failing test then implement: after a run, stopping and restarting `qd serve` shows the stored outputs in the pane and on the slide without running anything; editing a cell marks its output stale in both (`tests/onslide.rs`, `src/ui/run.js`, `src/render.rs`)

**Notes (US5)**: persistence and staleness already followed from US3's output store (cache, keyed by cell, fingerprinted) and the preview/pane reading it on load; the test found one bug: `qd stop` returned before the server exited, so a `qd serve` right after overlapped the old one's shutdown (session file removed, renders into the same folder). `qd stop` now returns once the server is gone.

---

## Phase 9: Polish & cross-cutting

- [X] T033 [P] README: "Source and run" section (environments with uv/pixi and lockfiles, source pane, Run, safety); `qd schema` complete
- [X] T034 [P] Constitution amendment via the governance rules (MINOR): Constraints list uv or pixi as external tools needed only to run code / render decks with code (plan Complexity Tracking)
- [X] T035 Security check SC-006: a browser test page on another origin tries `fetch`/form posts to `/api/run`, `/api/env/add` and `/api/op` with `replace`; assert no kernel starts and no file changes; record in `checklists/security.md`
- [X] T036 Run quickstart §1–§5 with the release binary in the browser pane; record in `checklists/quickstart-run.md` (SC-001–SC-003 timings)
- [ ] T037 Agent eval (SC-005): 10 fresh agents asked to fix a failing cell in a deck with an environment; grade fixed output, ≤ 6 commands, no usage errors, `.qmd` changed only in the cell; record in `checklists/agent-eval.md`
- [ ] T038 Final constitution re-check in plan.md; fmt, clippy, full suite green; ROADMAP status
- [ ] T039 Concision audit (`/pr-audit`), last task before the PR is marked ready

---

## Dependencies & Execution Order

- Setup (T001–T003) → Foundational (T004–T007) → US1 (T008–T010) → US2 (T011–T012) — the MVP needs no environment.
- Environments (T013–T018) can run in parallel with US1/US2 after Phase 2; they block US3–US5.
- US3 (T019–T028) needs environments; T019–T022 (bridge, store) can go in parallel with T023–T026 (server, preview) once T020/T022 interfaces are fixed.
- US4 (T029–T031) needs T020, T022, T024. US5 (T032) needs T022, T026, T028.
- Polish last; T039 before each PR.

## Parallel opportunities

- T001 ∥ T002; T004 ∥ T006.
- Environment phase (Rust + uv) ∥ source-pane phases (JS).
- Within US3: bridge + store (T019–T022) ∥ server + preview (T023–T026).

## Implementation strategy

1. MVP = Phases 1–4: the source pane with sync (useful immediately, no environment needed) → PR.
2. Environments + US3 → running code in a reproducible environment → PR.
3. US4 (agents) and US5 (persistence), then polish → PR.
