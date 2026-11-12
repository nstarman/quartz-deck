---
description: "Tasks for 010 one-file deck"
---

# Tasks: One-file deck — the build tools move into qd

**Input**: `specs/010-one-file-deck/` (spec.md, plan.md)

**Tests**: required (constitution VI); each story's tests are written first.

## Phase 1: Foundational

- [X] T001 Add `regex`, `base64`, `flate2`, `mime_guess`, `percent-encoding` as direct dependencies (all already in `Cargo.lock`); add `src/inline.rs` with `pub mod inline;`.

## Phase 2: US1 — everything in one file (P1) 🎯 MVP

- [X] T002 [US1] Failing unit tests in `src/inline.rs`: local paths; app inlining (bg iframe, `data-app`, `src`, scripts not searched, gzip round trip, loader before `Reveal.initialize`); `<object>`/SVG `<image>` to `data:`; dedupe of a repeated large URI (and not of JavaScript); missing files listed; entities and `%`-escapes decoded.
- [X] T003 [US1] Implement `inline::inline` (port of `inline_apps.py`). T002 passes.
- [X] T004 [US1] `src/export.rs`: render into `<staging>/.render/`, move the HTML up, inline, delete the render folder; background iframes no longer in `files`; missing files become `P_MISSING_FILE`. Update `ships_files_that_cannot_be_inlined`.
- [X] T005 [US1] Fixture `tests/fixtures/offline/onefile.qmd` + `figures/`, and `one_file_export` in `tests/export.rs` (2.5 MB app written by the test).

## Phase 3: US2 — shared copy (P2)

- [X] T006 [US2] `--strip-notes` on `qd export` (CLI, schema, API); unit test on note stripping; fixture test.

## Phase 4: US3 — check coverage (P2)

- [X] T007 [US3] `src/offline.rs`: `P_MISSING_FILE` for any failed `file:` request to a missing file (deduplicated against background checks); `P_OVERFLOW` warnings (port of `check_overflow.py`, transitions off); `size` in the report from `inline::largest` (port of `size_report.py`, unit-tested).

## Phase 5: US4 — smaller PDFs (P3)

- [X] T008 [US4] Ghostscript recompression in `offline::print_pdf` when `gs` is on PATH (`compressed` in the result); `-notes.pdf` default name for `qd pdf --notes` and `qd export --pdf --notes`.

## Phase 6: US5 — References from Zotero (P3)

- [X] T012 [US5] `src/bib.rs` (port of `bibliography.py`): labels, links, sorting with 2021a/b, warnings, marker patch; unit tests. `qd bib --zotero KEY` through the normal edit path; `E_ZOTERO`. CLI test `bib_from_zotero` against a stand-in server (dry run, write, undo, errors).

## Phase 7: Polish

- [X] T009 Skill, README, `qd schema`, ROADMAP (010 added, later plans renumbered).
- [X] T010 Try on the PPP talk (copied to a temp folder): export, check, PDF; record the result in the PR.
- [X] T011 Concision audit (`/pr-audit`); fmt, clippy, tests.
