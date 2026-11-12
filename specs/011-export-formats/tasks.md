---
description: "Tasks for 011 export formats"
---

# Tasks: Export formats — every format Quarto and Pandoc can write

**Input**: `specs/011-export-formats/` (spec.md, plan.md, research.md,
data-model.md, contracts/, quickstart.md)

**Tests**: required (constitution VI, test-first). Each story's tests are
written first and seen failing.

**Organization**: by user story, in the spec's priority order. P1: US1
(PowerPoint and Beamer), US2 (tool check and the format list). P2: US3
(documents). P3: US4 (other Pandoc writers). Paths are relative to the
repo root.

**Starts after PR #43 (010 one-file-deck) is merged** (the person's choice;
research R8).

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on unfinished tasks)
- **[Story]**: US1–US4

---

## Phase 1: Setup

- [X] T001 Once #43 is merged, rebase `011-export-formats` onto `origin/main` and push with `--force-with-lease`; if the push is refused, stop and ask. Add `src/formats.rs` (empty, `pub mod formats;` in `src/lib.rs`) and `src/formats.lua` holding the filter from research R2, with the `FORMAT == "beamer"` exception for pauses and the pptx/beamer exception for notes.
- [X] T002 [P] CI: add TeX for the Quarto-backed tests in `.github/workflows/*.yml` (`quarto install tinytex --no-prompt`, cached), so Beamer and LaTeX PDF exports run on CI rather than being skipped. The fixture deck needs jupyter, matplotlib and plotly in the test environment (set up as the `run` fixtures do).

---

## Phase 2: Foundational (blocks every story)

- [X] T003 Write failing tests:
  - **`tests/serve.rs` `code_ops_need_the_token`:** `/api/op` with `render`, `export` or `pdf` and no `X-QD-Token` → 403. With the token → not 403. `show` without the token still works.
  - **`src/formats.rs` units:** run the filter through `quarto pandoc -L src/formats.lua` on a small Markdown sample (when Quarto is present): pauses go except in beamer; notes become a quote headed "Speaker notes" except in pptx/beamer; `.fragment` and `.absolute` divs are unwrapped.
- [X] T004 Make `render::quarto_cmd`, `run_quarto`, `failed` and `clean_log` `pub(crate)` in `src/render.rs`. Add `QdError::NoTex` → `E_NO_TEX` with hint `install TeX: quarto install tinytex` in `src/error.rs`. In `src/server.rs` `api_op`, require `authorized()` for `render`, `export` and `pdf`. In `src/ui/editor.js` `op()`, send `X-QD-Token: run.token`. T003 passes, and the existing browser tests still pass.

**Checkpoint**: code-running ops are token-checked; the filter works on its own.

---

## Phase 3: User Story 1 — PowerPoint and Beamer (P1) 🎯 MVP

**Independent Test**: spec US1; quickstart §2–§3.

- [X] T005 [P] [US1] Write failing tests:
  - **`src/formats.rs` units for `losses(d, fmt)`**, on `tests/fixtures/export/carry.qmd` parsed in memory: for pptx it lists exactly `placed` [#placed], `pause` and `fragment` [#frags], `interactive` [#plotly], `theme` only when the deck sets one. For beamer: `notes` as `hidden` [#cols], `pause` absent, `fragment` [#frags], `background-image` [#bg], `placed`, `interactive`. A deck without notes reports no `notes` loss (FR-013).
  - **`src/formats.rs` units:** `Format` lookup by name; an unknown name is `E_INVALID_ARG` whose hint names `qd export --list`; the default output is `export/<stem>.<ext>`; `echo: false` goes in the metadata only when the deck sets no `echo`.
  - **`tests/export_formats.rs` (new, Quarto-backed, as other render tests) `pptx_and_beamer`:** on a temp copy of the fixture folder, `qd export --to pptx` writes `export/carry.pptx` with 9 slides, the NOTESCOLS note in slide 2's notes, a `<p:bg>` on the Background slide and no ". . ." text. The JSON matches data-model "Export result". `qd export --to beamer` writes a 10-page PDF. The deck's SHA-256 is the same before and after both runs (SC-005).
  - **`tests/export_formats.rs` `failed_export_leaves_nothing`:** a deck whose cell raises → `E_RENDER` naming the slide; a file already at `--out` is untouched; no `.qd-export-*` files are left in the folder.
  - **`tests/cli.rs` `plain_export_unchanged`:** `qd export` with no `--to` gives the same result shape as before; `--to revealjs` is the same as no `--to`; `--pdf` together with `--to pptx` is a usage error.
- [X] T006 [US1] Implement in `src/formats.rs`: the `FORMATS` table for revealjs, revealjs-pdf, pptx and beamer (data-model "Format", spec FR-012 columns); `Feature`/`Keep`; `losses`; `run(store, name, out, dry_run)` per plan "Route and run" (copy, filter and metadata with a Drop guard, the Quarto route, a temp output renamed on success, `changed_during`). `revealjs`/`revealjs-pdf` call `export::run`. In `src/cli.rs` add `Export --to` / `--out` (with the conflicts in contracts/cli.md) and the `export-format` shape in `qd schema`. T005 passes.
- [X] T007 [P] [US1] Write failing browser test `export_menu_pptx` in `tests/onslide.rs`:
  - Export ▾ in the header lists Slides (web page, slides PDF, PowerPoint, Beamer PDF);
  - picking PowerPoint opens the sheet with the losses from the dry run before export;
  - Export runs, the status says "Exporting PowerPoint…", and the result sheet links `export/carry.pptx`;
  - the losses are grouped by slide, and clicking "#placed" selects that slide;
  - File ▸ Export shows the same entries, and `export`/`export-pdf` keep their ids and keys.
- [X] T008 [US1] Editor: the Export ▾ header button and menu (`src/ui/editor.js`, `src/ui/menubar.js`), `export-to-<name>` actions plus the File ▸ Export submenu in `src/ui/actions.json`, the export sheet and the result sheet with the loss list (`src/ui/sheet.js`, `src/ui/editor.css`). Add the notes warning before a Beamer export (FR-014). In `src/server.rs` `api_op`, set `href` for outputs of any extension inside the deck folder. "Show in Finder": reuse an existing reveal helper if there is one, else add `POST /api/reveal` (token, deck folder only; contracts/http.md). T007 passes.

**Checkpoint**: US1 works alone (MVP): PowerPoint and Beamer from the editor and from `qd`, with losses.

As built (MVP PR): the result links the file for download instead of a
"Show in Finder" button (no `/api/reveal`); the output path is changed with
`qd export --out`, not in the sheet; File ▸ Export is the same four entries
in the File menu, not a submenu. Unit tests live in `src/formats.rs`, CLI
tests in `tests/export_formats.rs` (T009's CLI tests are there too).

---

## Phase 4: User Story 2 — Check what a format needs first (P1)

**Independent Test**: spec US2; quickstart §1, §3 (TeX hidden).

- [X] T009 [P] [US2] Write failing tests:
  - **`src/formats.rs` units for `tex_found(path_env, home)`:** found through `xelatex`/`lualatex`/`pdflatex` on PATH, through TinyTeX's bin folder, or not at all.
  - **`tests/cli.rs` `export_list`:** `qd export --list` returns every format in data-model "Format list" order, with `ready` and `hint`; `--list` with `--to` is a usage error.
  - **`tests/cli.rs` `no_tex_fails_fast`:** with `PATH` reduced to Quarto's folder and `/usr/bin:/bin` and `HOME` set to a temp dir, `--to beamer` and `--to pdf` fail with `E_NO_TEX` in under 2 s, write no file, and run no code. `--to typst` still gets past the check.
- [X] T010 [US2] Implement `tex_found`, the check at the start of `run` (FR-015), `list()` with `ready`/`needs`/`hint`, and `--list` in `src/cli.rs` (plus the `export-formats` shape in `qd schema`). T009 passes.
- [X] T011 [P] [US2] Write a failing browser test `export_menu_disables_missing_tools` in `tests/onslide.rs`: the server is started with TeX hidden; Beamer PDF and PDF (LaTeX) are disabled, with "install TeX: quarto install tinytex" as the tooltip; PowerPoint and PDF (Typst) stay enabled.
- [X] T012 [US2] The menu reads `qd export --list` through `/api/op` each time it opens and disables formats that aren't ready, with the hint as the tooltip (through `actions.js` `why()`). T011 passes.

**Checkpoint**: P1 complete.

---

## Phase 5: User Story 3 — Handouts and document copies (P2)

**Independent Test**: spec US3; quickstart §4.

- [X] T013 [P] [US3] Write failing tests:
  - **`src/formats.rs` units:** losses for docx, odt, html, pdf, typst, gfm, commonmark, ipynb and epub follow the spec's table for the fixture; each column is checked.
  - **`tests/export_formats.rs` `documents`:** on the fixture, docx, typst, gfm and ipynb each contain every marker word. NOTESCOLS follows a "Speaker notes" label; no ". . ."; no `plt.plot` source (echo off). gfm lists `export/carry_files/…png` in `files` and the file exists. ipynb's code cells are code cells. The deck hash is unchanged.
- [X] T014 [US3] Add the Documents rows to `FORMATS` (labels "Word", "Web page", "PDF (LaTeX)", "PDF (Typst)", "Markdown (GitHub)", "Markdown (CommonMark)", "Jupyter notebook", "OpenDocument", "EPUB"). Move the Markdown `<stem>_files/` folder next to the output and list it in `files` (FR-009). Add the Documents group to the menu. T013 passes.

**Checkpoint**: all menu formats work.

As built: Quarto's `<name>_files/` folder (Markdown figures, the HTML
page's libraries) is merged into `<out>_files/` beside the output and the
references rewritten, so the HTML page and Markdown of one deck can share
it. The HTML page is labelled "HTML page" to tell it from the offline web
page. File ▸ Export is now a submenu with every format (one action each:
`export-docx`, `export-latex` for `pdf`, …).

---

## Phase 6: User Story 4 — Any other Pandoc writer (P3)

**Independent Test**: spec US4; quickstart §5.

- [X] T015 [P] [US4] Write failing tests:
  - **`src/formats.rs` units:** a writer from Pandoc's list that isn't in `FORMATS` gets group `other` and an ext from the map (`rst`, `org`, `latex→tex`, `asciidoc→adoc`, default `txt`).
  - **`tests/export_formats.rs` `other_writers`:** `--to rst` and `--to org` write files that hold every slide title and the labelled notes; rst's plot is in `export/carry_files/` and listed in `files`. `--to nosuch` fails with `E_INVALID_ARG`.
  - **Browser test `export_other_format`:** Export ▾ ▸ Other format… opens a filterable list; picking `rst` exports it.
- [X] T016 [US4] Read the writer list once per process (`quarto pandoc --list-output-formats`), accept any of them by name through the same Quarto render as the menu formats (research R3, revised 2026-11-12: the two-step Markdown → Pandoc route was built, then dropped), move `<name>_files/` for any text output, and build the "Other format…" sheet in the editor. T015 passes.

---

## Phase 7: Polish & cross-cutting

- [X] T017 [P] Docs: README export section, the qd skill (`qd export --to`, `--list`, losses, `E_NO_TEX`), `specs/ROADMAP.md` 011 status, and the 007 action inventory ("Added by 011").
- [X] T018 Run quickstart §1–§7 in the browser pane and record it in `specs/011-export-formats/checklists/quickstart-run.md`.
- [X] T019 Final constitution re-check in `plan.md`; fmt, clippy, `cargo test --no-fail-fast` and `node --test vscode/test/*.test.js` green.
- [X] T020 Concision audit (`/pr-audit`), the last task before each PR is marked ready.

---

## Dependencies & Execution Order

- **Setup (T001–T002) → Foundational (T003–T004)** → every story.
- **US1 (T005–T008):** after Foundational.
- **US2 (T009–T012):** the CLI half (T009–T010) only needs Foundational. The menu half (T011–T012) needs US1's menu (T008).
- **US3 (T013–T014):** after US1 (`FORMATS`, `run`, the menu).
- **US4 (T015–T016):** after US3 (filter and route shared).
- **Polish:** after the stories it documents.

Within each story: tests first (seen failing), then `formats.rs` and the
CLI, then the server, then the editor.

## Parallel opportunities

- T002 (CI) ∥ T003 (foundational tests).
- T005 (US1 tests) ∥ T009 (US2 CLI tests): different test functions.
- T010 (TeX check, `--list`) ∥ T008 (editor menu): different files.

## Implementation strategy

1. **MVP:** Setup, Foundational, US1 and US2's CLI half. PowerPoint and
   Beamer from `qd` and the editor, token-checked ops. One PR.
2. **P1 rest:** US2's menu half (disabled formats). Can ride with the MVP
   PR if small.
3. **P2:** US3, the document formats. One PR.
4. **P3:** US4, other writers. One PR, then the polish PR.

Each PR passes the concision audit (T020) before it is marked ready, and
is merged only after the person approves it.
