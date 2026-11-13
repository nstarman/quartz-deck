# Implementation Plan: Citations

**Branch**: `014-citations` | **Date**: 2026-11-13 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `/specs/014-citations/spec.md`

## Summary

Decks move to Quarto's own citations, and `paper` goes.
- **Two render plugins**, as Quarto extensions in `plugins/`:
  - `cite-links` links each citation to its paper, with an icon and the
    title on hover. It adds the corner form, and brings `talk.csl`, a style
    of our own that reproduces today's labels.
  - `refs-slide` gives the References slide its compact, short or full
    style, its columns and its size.

  Both run citeproc themselves, from a post-quarto filter, because Quarto
  runs its own citeproc after every filter entry point (R1).
- **`qd bib`** becomes a command group that edits the deck's references,
  with every write a qd edit:
  - `pull FILE` and `pull --from NAME ARGS`;
  - `embed`, `extract`, `convert`;
  - `install`;
  - `migrate`.

  qd keeps the references as CSL-JSON in memory, and Pandoc from Quarto
  does the format changes. Undo covers the companion file (R4).
- **`qd-zotero`**, the first source plugin, is a small Rust binary in the
  workspace that prints a Zotero collection as CSL-JSON.
- **The conversion**: `qd bib migrate` converts every `paper` deck, the PPP
  talk among them. It renders before and after and compares them. Any
  difference fails the conversion, until the person accepts the report
  (`--accept`, from the Clarifications). Then `paper`, `qd bib --zotero` and
  the marker list are removed.

## Technical Context

**Language/Version**: Rust stable (edition 2024, rust-version 1.89); Lua for the two Quarto filters (Pandoc 3's Lua API); CSL 1.0.2 XML for `talk.csl`

**Primary Dependencies**: no new crates. `qd-zotero` uses `ureq` and
`serde_json`, already in the lockfile. At run time:
- `quarto` (its Pandoc converts formats; its citeproc renders);
- Zotero 7 with its local API, only for `pull --from zotero`.

**Storage**:
- **the references**: the companion `.bib` or `.json` file, or the deck's
  front matter;
- **undo journal entries**: in qd's cache, as today, with an optional
  `also` list of other files (R4);
- **migrate backups and reports**: in qd's cache, or in `--backup DIR`.

**Testing**:
- `cargo test`:
  - unit tests for merge and keys, URL and title normalising, key
    generation, extension install and version compare, and shortcode
    parsing;
  - CLI integration tests for each `qd bib` subcommand: JSON output, the
    resulting files, `--dry-run`, `--expect` and undo;
  - `plugins.rs` renders every `plugins/*/example.qmd` with plain Quarto,
    and checks the HTML for links, icons, labels, the a/b suffix, the
    corner form and the three styles;
  - `qd-zotero` against the mock server;
  - `migrate` on a fixture, including one with an injected difference
    (fail, restore byte for byte) and the same one with `--accept`.
- The offline check on both example decks.

**Target Platform**: macOS first; nothing Linux-hostile (child process via `PATH`, no shell)

**Project Type**: CLI + local web editor (existing), plus Quarto extensions and one small CLI plugin

**Performance Goals**: `pull --from zotero` with Zotero closed fails in under 5 s (SC-006); migrate's two renders are bounded by Quarto (about a minute for the PPP talk)

**Constraints**:
- decks render with plain `quarto render` once the two extensions are in
  `_extensions/`;
- offline at render and present time;
- no Python needed to cite;
- byte-preserving patches;
- no writes outside the deck's folder or qd's cache.

**Scale/Scope**: decks with tens to about a hundred references (PPP: 15 entries, 35 in-text citations)

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status |
|---|---|
| I. Clean room | ✅ Only Pandoc, Quarto and CSL public docs. `talk.csl` is written from scratch, and no CC BY-SA style is copied. |
| II. `.qmd` is the truth | ✅ Front-matter and shortcode edits are minimal patches. The companion file is named by `bibliography:`, which Quarto reads, so there is no hidden state. qd rewrites the companion file whole, and its header comment says so (FR-021). |
| III. CLI first | ✅ Everything is a `qd bib` subcommand with JSON output and stable error codes (`E_SOURCE`, `E_MIGRATE_DIFF`, `E_READ_ONLY`). No editor change. `qd schema` and the skill are updated. |
| IV. Live preview | ✅ The preview renders through Quarto with the deck's extensions, so citations show there unchanged. A pull changes the deck or its companion file. The watcher already re-renders on a deck change. A change to the companion file alone needs the watcher to also watch the file `bibliography:` names, which is one added path, a US2 task. |
| V. Safe concurrent edits | ✅ Every write is atomic and conflict-checked. Undo spans the deck and the companion file, and refuses if either changed (R4). |
| VI. Test first | ✅ Tests come before code in each story. Round trips: embed then extract, and bib to json to bib, leave the rendered citations identical (SC-004). |
| VII. Simplicity, plugins | ✅ Amendment 1.4.0 (R12, PR 1): source plugins, and a second binary in the workspace. After it: ✅ two small extensions, one small binary, no registry, no manifest reader at run time, no new crates. |

Gate result: **pass, provided the 1.4.0 amendment is the first PR** (Phase 0
of tasks). Recorded in Complexity Tracking.

**Re-check after Phase 1 design**: unchanged. The design adds no
abstraction beyond `bib::merge`, `plugins::install` and the undo entry's
`also` list, and each has more than one caller (pull, embed, extract,
convert and migrate all use merge and install, and undo and redo both use
`also`).

## Project Structure

### Documentation (this feature)

```text
specs/014-citations/
├── plan.md              # this file
├── research.md          # R1–R12, measured on Quarto 1.11.9
├── data-model.md        # entries, deck references, merge result, migrate report
├── quickstart.md        # run-through on a scratch deck and the PPP talk
├── contracts/
│   ├── cli.md           # qd bib … commands, JSON, errors
│   └── extensions.md    # cite-links and refs-slide markup and options; source plugin protocol
└── tasks.md             # /speckit-tasks
```

### Source Code (repository root)

```text
Cargo.toml                       # workspace members += "plugins/zotero"
crates/qd/src/
├── bib.rs                       # rewritten: CSL entries, merge, keys, formats (via quarto pandoc), embed/extract/convert, pull, sources
├── migrate.rs                   # new: paper shortcodes → citations, backup, before/after check
├── plugins.rs                   # new: the two extensions compiled in, install + version compare
├── store.rs                     # undo entry gains `also` (other files)
├── cli.rs                       # `qd bib` subcommands; old --zotero removed in the last PR
├── error.rs                     # E_SOURCE, E_MIGRATE_DIFF, E_READ_ONLY
└── server.rs                    # watch the companion file too
crates/qd/tests/
├── bib.rs                       # new: CLI tests for qd bib
├── migrate.rs                   # new
└── plugins.rs                   # example renders for cite-links and refs-slide; paper's copy check removed at the end
plugins/
├── cite-links/                  # _extensions/cite-links/{_extension.yml, cite-links.lua, cite-links.css, talk.csl}, example.qmd, references.bib, README.md
├── refs-slide/                  # _extensions/refs-slide/{_extension.yml, refs-slide.lua, refs-slide.css}, example.qmd, README.md
├── zotero/                      # Cargo.toml (qd-zotero), src/main.rs, tests/, qd-plugin.json, README.md
└── paper/                       # removed in the last PR
skills/quarto-revealjs/          # assets: paper removed, scaffold cites with @key, Makefile refs → qd bib pull --from zotero; SKILL.md docs
skills/quartz-deck/SKILL.md      # qd bib commands
tests/skills/quarto-revealjs/fixture/lessons.qmd   # converted
.specify/memory/constitution.md  # 1.4.0, first PR
```

**Structure Decision**: the existing workspace gains one member,
`plugins/zotero`. The extensions live in `plugins/` as Quarto extensions,
compiled into `qd` from there, so there is one copy of each. The skill's
scaffold no longer carries an extension copy. A deck set up from the skill
gets the extensions from `qd bib install`, which removes the copy-sync test.

## PRs (order of work)

1. **Constitution 1.4.0** (R12).
2. **US2, `qd bib` core**:
   - formats, merge and keys, embed, extract and convert, `pull FILE`;
   - read-only outside the deck folder, undo with `also`, `install`
     (empty until US1), watching the companion file.

   The old `--zotero` stays.
3. **US1, `cite-links`**: the extension, `talk.csl`, the example, the
   render tests, and installing it.
4. **US3, `qd-zotero`** and `pull --from`.
5. **US4, `refs-slide`**: the extension, the example, the render and
   overflow tests.
6. **US5, `migrate`**:
   - the fixtures and the skill scaffold converted;
   - the PPP talk converted (backup, the person opens Zotero, the report to
     the person, `--accept` on their word);
   - `paper`, `--zotero` and the marker list removed; the docs updated.

US2 comes before US1 because the example decks and tests of every later PR
use `qd bib`. Each PR passes `/pr-audit`.

## Complexity Tracking

| Violation | Why Needed | Simpler Alternative Rejected Because |
|---|---|---|
| A second binary (`qd-zotero`) beside `qd` (VII: "one `qd` binary") | Citation managers are plugins (the person's decision). Zotero's live source must not need a qd release to change, and Rust keeps citing Python-free. | Zotero compiled into `qd` (S2): a new manager would need a qd release. A Python plugin would require Python just to cite. Settled by amendment 1.4.0 (landed in PR 1, before any code). |
| The filters run citeproc themselves | Quarto runs citeproc after every filter entry point, and listing `citeproc` first crashes Quarto 1.11.9 (R1). | JS post-processing: the links would be lost in PDF and PPTX exports, and the deck would depend on JS. |
