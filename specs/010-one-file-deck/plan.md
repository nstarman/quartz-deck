# Implementation Plan: One-file deck

**Branch**: `010-one-file-deck` | **Date**: 2026-11-11 | **Spec**: [spec.md](spec.md)

## Summary

Port the `quarto-revealjs` skill's build scripts (nstarman/skills
`quarto-revealjs/assets/*.py`, origin/main 7c4a0d7) into qd's existing 002
pipeline. One new module, `src/inline.rs`, holds the pure HTML transforms
(inline apps/objects/SVG images, dedupe, strip notes, size report). `export.rs`
calls it between render and check and renders into a scratch folder so
Quarto's copied resources never reach the export. `offline.rs` gains generic
missing-file detection, the overflow measurement and the size report, and
the PDF step runs ghostscript when present.

## Technical Context

- Rust stable, edition 2024; existing chromiumoxide check/PDF driver.
- New direct dependencies, all already in `Cargo.lock` (no new crates):
  `regex` (via chromiumoxide), `base64` 0.22 (chromiumoxide), `flate2`
  (ureq, zip), `mime_guess` (tower-http), `percent-encoding` (url).
- Optional external tool: ghostscript (`gs`), only for PDF recompression.
- Tests: unit tests on the transforms in `src/inline.rs`; one integration
  fixture deck exported with Quarto + Chrome (`tests/export.rs`), skipped
  like the other 002 tests when either is missing.

## Constitution Check

- I Clean room: ported from the author's own skill scripts (nstarman/skills) and public
  reveal.js/Chrome behaviour; nothing from proprietary editors. ✅
- II `.qmd` source of truth: export still renders a copy; the deck and
  project files are never written. ✅
- III CLI-first: `--strip-notes` on `qd export`, new report fields in
  `qd schema`, same op through the editor API. ✅
- IV/V: unchanged (export is a build artefact; the atomic swap stays). ✅
- VI Test-first: transforms unit-tested first; fixture test for the whole
  pipeline. ✅
- VII Simplicity: one module, no new crates, no new subcommand; the Python
  scripts' behaviour is kept, not extended. ✅

## Design decisions (research)

- **R1 — where the inliner runs**: inside `qd export`, always. Its job is one
  file; a flag to turn inlining off has no use.
- **R2 — Rust `regex` has no look-around**: the Python `DATAURI` pattern's
  look-behind becomes a leading whitespace capture (attributes follow
  whitespace), its 4000-character minimum and the JavaScript exclusion move
  into code. `LOCALPATH`'s look-ahead becomes a function.
- **R3 — pruning**: Quarto renders into `<staging>/.render/`; only the HTML
  moves up, then the folder is deleted, before the check runs, so the check
  proves the deck does not need the copies.
- **R4 — overflow is a warning**: a box a few pixels over the edge should not
  block an export the night before a talk; the report lists every one.
- **R5 — app proof in tests**: the fixture's 2.5 MB app throws a known error;
  the check's page-error capture sees it only if the app really ran from its
  `blob:` URL.

## Project Structure

```text
src/inline.rs      new: inline(), largest(), local()
src/export.rs      render to .render/, inline, prune, strip-notes, files list
src/offline.rs     missing-file capture, overflow, size, gs recompression
src/cli.rs         --strip-notes, schema text, -notes.pdf default
tests/export.rs    one_file_export fixture test
tests/fixtures/offline/onefile.qmd (+ figures/)
skills/quartz-deck/SKILL.md, README.md, specs/ROADMAP.md
```
