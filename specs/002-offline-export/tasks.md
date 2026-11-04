---
description: "Task list for 002-offline-export"
---

# Tasks: Offline Export and PDF

**Input**: Design documents from `specs/002-offline-export/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/cli.md, contracts/http.md, quickstart.md

**Tests**: Required (constitution VI). Write each test task first and see it
fail before the implementation task after it. Tests that need Quarto, a
browser or Python plotly skip (pass trivially, with a message) when it is
absent; CI provides all three.

**Organization**: by user story (spec.md P1–P4). Paths relative to repo root.

## Format: `[ID] [P?] [Story] Description`

---

## Phase 1: Setup

- [ ] T001 Add deps in `Cargo.toml`: `headless_chrome` (default features off except what DevTools + `print_to_pdf` need; no `fetch`), `ureq` (rustls TLS), `zip`; confirm `cargo test --test licenses` passes (FR-041) and record the crates in research.md R6 if any differ
- [ ] T002 [P] Create `tests/fixtures/offline/` decks, one failure mode each (research R1–R7): `math.qmd` (inline + display math), `fragments.qmd` (3 slides, one with 2 fragments, `.notes`), `bgvideo.qmd` (+ `media/clip.mp4`, a few-KB generated file), `bgiframe.qmd` (+ `apps/app.html`), `missing.qmd` (background video path that does not exist), `chalkboard.qmd` (`chalkboard: true`), `shownotes.qmd` (`show-notes: true`), `skillsetup/` (folder with the quarto-revealjs `_quarto.yml` + `_deck/header.html` + `_vendor/` stubs loading MathJax), `plotly.qmd` (Python cell with a Plotly figure); record purpose per file in `tests/fixtures/offline/README.md`
- [ ] T003 [P] CI in `.github/workflows/ci.yml`: set up Python, `pip install plotly jupyter`, and expose the runner's Google Chrome (`QD_CHROME`) so browser/plotly tests run, not skip

---

## Phase 2: Foundational (blocks every story)

- [ ] T004 Error and problem types in `src/error.rs` / new `src/offline.rs` types: error codes `E_NO_BROWSER`, `E_DOWNLOAD`, `E_OFFLINE`, `E_OFFLINE_INCOMPATIBLE` with hints (contracts/cli.md); `Problem { code: P_NETWORK|P_MISSING_FILE|P_PAGE_ERROR|P_RAW_TEX|P_OFFLINE_INCOMPATIBLE|P_NOTES_VISIBLE, severity: error|warning (P_NOTES_VISIBLE is warning), slide: Option<anchor>, message, hint, url }`, `CheckReport { ok, html, slides, steps, problems }` (`ok` iff no error-severity problem) — data-model.md
- [ ] T005 [P] Write failing tests in `src/assets.rs` (`#[cfg(test)]`): `ensure_mathjax(cache, allow_download)` returns `<cache>/lib/mathjax-3.2.2/tex-svg-full.js`; a cached file with a wrong SHA-256 is rejected and re-fetched (`E_DOWNLOAD` if download forbidden); a download whose bytes do not match `a4354ff9…11cbe6` is deleted and fails `E_DOWNLOAD`; download test gated on network (skip offline); `plotly_from_python()` finds `package_data/plotly.min.js` via `python3 -c` or returns `None`
- [ ] T006 Implement `src/assets.rs` (pinned MathJax URL + full SHA-256 from research R6; download with `ureq` to a temp file, verify, rename; `Asset { name, version, sha256, source }`) so T005 passes
- [ ] T007 [P] Write failing tests in `src/browser.rs`: `find()` order `QD_CHROME` → standard macOS paths → PATH names (`google-chrome`, `google-chrome-stable`, `chromium`, `chromium-browser`) → cached download; `QD_NO_DOWNLOAD=1` and nothing found → `E_NO_BROWSER` with hint; pinned Chrome for Testing table has a version + SHA-256 for mac-arm64, mac-x64, linux64; checksum mismatch → `E_DOWNLOAD`, nothing left in cache; `remove()` deletes only qd's cache folder
- [ ] T008 Implement `src/browser.rs`: discovery; download of the pinned Chrome for Testing `chrome-headless-shell` zip for the platform via `ureq` (progress-free, size reported), SHA-256 verify, unzip with `zip` into `<cache>/browser/<version>/`; record the pinned hashes (fetch each zip once, hash it, commit the constants); `qd browser [--install|--remove] [--no-download]` in `src/cli.rs` returning `{"path","source","version"}` — T007 passes
- [ ] T009 Launch helper in `src/offline.rs`: start Chrome headless via `headless_chrome::Browser` with the found path, fixed window 1280×800, a tab with Fetch interception (continue `file:`, `data:`, `blob:`, `about:blank*`; fail others with `InternetDisconnected` and record URL + current slide), `Runtime.exceptionThrown` listener, `Network.loadingFailed` for `file:` URLs (research R3); shared by check and PDF

**Checkpoint**: libraries and a browser can be obtained and verified; a page can be opened offline.

---

## Phase 3: User Story 1 — Check a deck works offline (P1) 🎯 MVP

**Goal**: `qd check --offline` reports every offline problem with slide, code and hint.

**Independent Test**: default Quarto deck with math → `P_NETWORK` (cdn mathjax) + `P_RAW_TEX`; an offline-ready deck → ok.

### Tests (write first, must fail)

- [ ] T010 [P] [US1] Integration tests `tests/offline.rs` (gated on quarto + browser): render `tests/fixtures/offline/math.qmd` plainly → report has `P_NETWORK` with a `cdn.jsdelivr.net/npm/mathjax` URL and `P_RAW_TEX` on slide anchor `math`, `ok:false`; `about:blank?config=…` never reported (FR-003); `missing.qmd` → `P_MISSING_FILE` with the path and slide; a page that throws (fixture with a `{=html}` script `throw new Error("boom")`) → `P_PAGE_ERROR`; `shownotes.qmd` → `P_NOTES_VISIBLE` warning (ok stays true); counts `slides`/`steps` match the fixture (fragments.qmd: 3 slides + title, steps = slides + fragments)
- [ ] T011 [P] [US1] CLI tests in `tests/offline.rs`: `qd check --offline` exits 1 with `E_OFFLINE` on stderr **and** the full `CheckReport` on stdout; `--html FILE` checks any HTML (FR-004); `QD_CHROME=/nonexistent QD_NO_DOWNLOAD=1` → `E_NO_BROWSER`; editing commands still work in that state (FR-040)

### Implementation

- [ ] T012 [US1] Implement `offline::check(html, browser) -> CheckReport` in `src/offline.rs`: load, await Reveal ready + `MathJax.startup.promise` when present, visit every slide (`Reveal.getSlides()`, `Reveal.slide(h,v)`) stepping each fragment, attribute requests/exceptions to the current slide anchor (load-time ones → `slide: null`), raw-TeX test on each slide's visible text, static scan of `data-background-video|iframe|image` / parallax paths against the HTML's folder (research R3) — T010 passes
- [ ] T013 [US1] `qd check --offline [--html FILE] [--no-download]` in `src/cli.rs`: without `--html`, render the deck to a temp folder (as `qd render`) and check that; print the report; exit 1 + `E_OFFLINE` when not ok; YAML `show-notes` warning from the deck's front matter / `_quarto.yml` (research R7) — T011 passes

**Checkpoint**: MVP — an agent can find out whether a deck will work offline, and where it breaks.

---

## Phase 4: User Story 2 — Export a self-contained deck (P2)

**Goal**: `qd export` → one HTML + needed files, check passed, deck untouched.

**Independent Test**: export a deck with math + background video; offline open works; check ok; `git status` clean for the deck.

### Tests

- [ ] T014 [P] [US2] Unit tests in `src/export.rs`: `export_copy(deck, ctx)` merges into front matter `format.revealjs.embed-resources: true`, `html-math-method {method: mathjax, url: "about:blank"}`, and appends qd's header to `include-in-header` (string or list form preserved); keeps all other keys; deck with `format: revealjs` as a string becomes a map; already-set-up decks (math url `about:blank` in front matter or any `_quarto.yml` up to the project root) get no qd header (research R1); decks with Python plotly cells get the renderer cell after the front matter, others do not (R2); `chalkboard`/`multiplex` → `E_OFFLINE_INCOMPATIBLE` naming key and file (R7); `collect_files` returns background video/iframe/parallax paths relative to the deck
- [ ] T015 [P] [US2] Integration tests `tests/export.rs` (gated on quarto + browser): export `math.qmd`, `fragments.qmd`, `bgvideo.qmd`, `bgiframe.qmd`, `skillsetup/` → each `check.ok`, zero `P_NETWORK`/`P_RAW_TEX`; `files` lists `media/clip.mp4` / `apps/app.html` and they exist in the export; deck `.qmd` and `_quarto.yml` bytes unchanged and no `*.qd-export.qmd` left behind (FR-012); `chalkboard.qmd` → `E_OFFLINE_INCOMPATIBLE`, no export folder; second export after making the deck fail (missing video) → `E_OFFLINE`, previous export folder byte-identical, staging path in `details` (FR-015); `plotly.qmd` (gated on Python plotly) → check ok and the HTML contains no `cdn.plot.ly` / web MathJax

### Implementation

- [ ] T016 [US2] Implement `src/export.rs`: `export_copy` (serde_yaml_ng merge of the front matter, header file `<cache>/lib/qd-offline-header.html` written from a template with absolute library paths), plotly renderer cell (the skill's `_deck/setup.qmd` contents), offline-incompatible detection, `collect_files` — T014 passes
- [ ] T017 [US2] Orchestration `export::run(store, opts)`: ensure assets (T006; plotly only when needed), render the export copy into `<cache>/export-staging/<n>/` via `render::render(..., preview=false)`, rename HTML to `<stem>.html`, copy collected files, run `offline::check`, then on ok swap into `opts.out` (rename new in, remove old after); on failure keep staging and return `E_OFFLINE` with the report — T015 passes
- [ ] T018 [US2] `qd export [--out DIR] [--pdf] [--notes] [--per-slide] [--no-download]` in `src/cli.rs` returning `ExportResult` JSON (`assets` with versions/hashes); `--pdf` wired in Phase 5

**Checkpoint**: one command produces a verified offline deck.

---

## Phase 5: User Story 3 — Print the deck to PDF (P3)

**Goal**: PDF via reveal's print mode; page count = fragment steps (or slides); notes only on request.

**Independent Test**: `fragments.qmd` → pages == steps; `--notes` adds note pages; without, no notes text.

### Tests

- [ ] T019 [P] [US3] Tests in `tests/export.rs` (gated): `qd export --pdf` on `fragments.qmd` → `pdf.pages` == check `steps` and equals the page count read from the PDF structure (`/Type /Page` objects); `--per-slide` → pages == slides; `--notes` → pages == steps + slides-with-notes and the notes text is in the PDF; without `--notes` the notes text is absent; `qd pdf` with no export → `E_NOT_FOUND` hint `qd export`; `.print-only` still shown / `.print-hide` hidden (fixture slide with both)

### Implementation

- [ ] T020 [US3] Implement `offline::print_pdf(html, opts) -> PdfResult` in `src/offline.rs`: open `?print-pdf&pdfSeparateFragments=<!per_slide>` (+ `&showNotes=separate-page` when notes), same offline interception, await Reveal + MathJax, `print_to_pdf { prefer_css_page_size, print_background }`, write file, count pages (research R4); qd's export header also carries the skill's `.print-only`/`.print-hide` CSS
- [ ] T021 [US3] `qd pdf [--html FILE] [--out FILE] [--notes] [--per-slide] [--no-download]` in `src/cli.rs`; `qd export --pdf` prints `export/<stem>.pdf` and adds `pdf` to the result — T019 passes

**Checkpoint**: archive HTML + PDF from one command.

---

## Phase 6: User Story 4 — Editor and agents (P4)

**Goal**: Export from the editor with problems linked to slides; agents via CLI + Skill.

**Independent Test**: editor Export on a deck with a web request shows the problem; clicking it shows the slide; CLI JSON matches.

- [ ] T022 [P] [US4] Tests: `run_op` parity in `tests/cli.rs` for `export` (`{"pdf": true}`) and `check` (`{"offline": true}`) vs the CLI (skip when no quarto/browser); `run_op` refuses `browser`, and `out`/`html` paths outside the deck folder (`E_INVALID_ARG`); `tests/serve.rs`: `POST /api/op` export reply has `href`s for HTML and PDF
- [ ] T023 [US4] Allow `export`, `pdf` and `check --offline` in `API_OPS` with the path restrictions in `src/cli.rs`; server adds `href`s for `html`/`pdf` like `render` in `src/server.rs` — T022 passes
- [ ] T024 [US4] Editor in `src/ui/index.html` + `editor.js` + `editor.css`: Export button (+ "with PDF" checkbox) → `op('export', …)`; a results panel listing problems (code, message, hint) — each with a slide becomes a button that navigates the preview there; links to the exported HTML/PDF
- [ ] T025 [US4] Skill `skills/quartz-deck/SKILL.md`: an "Offline export" section (check → export → pdf loop, problem codes and fixes, first-run download note, `qd browser`); Skill/CLI sync test covers the new commands
- [ ] T026 [US4] Browser verification per quickstart §4 (Export with a failing deck, click problem → slide; export OK deck → links open); record in `specs/002-offline-export/checklists/gui.md`

---

## Phase 7: Polish & Cross-Cutting

- [ ] T027 [P] SC-004 timing test `tests/export.rs` (`#[ignore]`, gated): 30-slide deck with math + one Plotly figure, export + check + PDF < 60 s; record the measurement in quickstart.md
- [ ] T028 [P] SC-001 detection table: one test per failure mode from the quarto-revealjs guide (web MathJax default, KaTeX → raw TeX, Plotly web MathJax/plotly.js, missing background video, missing background iframe) asserting the expected problem code; record results in `specs/002-offline-export/checklists/detection.md`
- [ ] T029 SC-005 agent evaluation: 10 fresh agents with only the Skill, task "export this deck; fix the reported problem; export again" on a deck with a missing background video; logged wrapper + objective grader as in 001; record in `specs/002-offline-export/checklists/agent-eval.md`
- [ ] T030 [P] README: offline export section; contracts/cli.md of 001 links to 002 additions
- [ ] T031 Run quickstart §1–§5, `cargo test`, clippy, fmt; re-check the constitution table in plan.md; CI green

---

## Dependencies & Execution Order

- Phase 1 → Phase 2 (T004–T009) → stories.
- **US1** (check) needs T004, T008 (browser), T009. **US2** (export) needs US1's `check` (T012) and T006 (assets). **US3** (PDF) needs T009 and US2's export folder for `qd pdf` defaults (T020 itself only needs T009). **US4** needs US1–US3.
- Polish last; T028 can start as soon as US1 is done.

```text
Setup → Foundational ─ US1 (check) ─ US2 (export) ─ US3 (pdf) ─ US4 (editor/agents) ─ Polish
                                   └──────── T020 (print_pdf) can start after T009 ┘
```

## Parallel Examples

- Phase 1: T002 ∥ T003.
- Phase 2: T005 ∥ T007 (different files), then T006, T008; T009 after T008.
- US1: T010 ∥ T011 (same file — write sequentially or split modules).
- US2 tests T014 ∥ T015; US3's T020 can be developed alongside US2 once T009 exists.

## Implementation Strategy

1. **MVP = Phases 1–3**: `qd check --offline` tells you (and an agent) whether a deck will work offline and where it breaks.
2. Add US2 (export) → the archive; then US3 (PDF); then US4 (editor + Skill).
3. Polish: timing, detection table, agent eval, docs, CI green.
