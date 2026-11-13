---
description: "Tasks for 014 citations"
---

# Tasks: Citations

**Input**: `specs/014-citations/` (spec.md, plan.md, research.md,
data-model.md, contracts/, quickstart.md)

**Tests**: required (constitution VI, test-first). Each story's tests are
written first and seen failing. Tests that need Quarto skip without it
(`common::have("quarto")`), as `plugins.rs` does today.

**Organization**: in the plan's PR order. US2 comes before US1 because every
later example and test uses `qd bib`:
1. Setup: the constitution 1.4.0;
2. Foundational;
3. US2 (P1, `qd bib` core);
4. US1 (P1, `cite-links`);
5. US3 (P2, `qd-zotero`);
6. US4 (P2, `refs-slide`);
7. US5 (P3, migrate and removing `paper`);
8. Polish.

Paths are relative to the repo root.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: can run in parallel (different files, no dependency on unfinished tasks)
- **[Story]**: US1–US5

---

## Phase 1: Setup (PR 1: constitution)

- [X] T001 Amend the constitution to 1.4.0 with `/speckit-constitution` (research R12), in `.specify/memory/constitution.md`:
  - VII names two plugin kinds: render plugins (Quarto extensions) and source plugins (programs that feed `qd bib` and render nothing);
  - "one `qd` binary, plus plugin binaries, each built in the workspace";
  - the plugin rules (licences, offline use, Run, writes) are unchanged.

  Record it in the Sync Impact Report. Then update the Complexity Tracking row in `specs/014-citations/plan.md` to say the amendment has landed.

**Checkpoint**: PR 1 merged before any plugin code.

---

## Phase 2: Foundational (blocks every story; lands in PR 2)

- [X] T002 Write failing unit tests in `crates/qd/src/store.rs` for an undo entry with `also` (data-model "Undo journal entry"):
  - `write_with_also` writes the deck and one other file;
  - `undo` restores both, and `redo` re-applies both;
  - a file that `also` created is removed on undo;
  - undo refuses with `E_NO_UNDO` when the other file changed since the edit, and keeps the entry;
  - a journal written before 014 (no `also`) still loads.
- [X] T003 Implement `also: Vec<OtherFile>` (`#[serde(default)]`) in `crates/qd/src/store.rs`:
  - `Store::write_many(before, deck_text, others, op, target)`: the others are written atomically first, the deck last, and all are conflict-checked against the versions they were read at;
  - `undo` and `redo` check, then write, every file.

  T002 passes.
- [X] T004 [P] Add `E_SOURCE`, `E_MIGRATE_DIFF` and `E_READ_ONLY` to `crates/qd/src/error.rs`, with codes, hints and exit codes as in `contracts/cli.md`. Add them to `qd schema`'s error list in `crates/qd/src/cli.rs`.
- [X] T005 Turn `Cmd::Bib { zotero }` in `crates/qd/src/cli.rs` into a subcommand group `qd bib [list|pull|embed|extract|convert|install|migrate]`, where bare `qd bib` is `list`. Keep `qd bib --zotero KEY` working, as a hidden flag on the group, until T037. Update `qd schema` and the JSON shapes in `cli.rs`'s output docs. Add an empty `crates/qd/tests/bib.rs` using `mod common;`.

**Checkpoint**: multi-file edits can be undone, and the command group exists.

---

## Phase 3: User Story 2 — References in the deck, in either format (P1) (PR 2)

**Goal**: pull from files, companion file or embedded, BibTeX ↔ CSL-JSON, stable keys, read-only outside the deck folder.

**Independent Test**: spec US2; quickstart §1.

- [X] T006 [P] [US2] Write failing unit tests in `crates/qd/src/bib.rs` for matching and keys (research R6):
  - DOI normalising (case, `https://doi.org/` prefix);
  - URL normalising (scheme, `www.`, trailing `/`, an arXiv `abs`/`pdf` id with its version);
  - title normalising;
  - match order DOI → URL → title;
  - `merge`:
    - a matched entry keeps its `id`, the incoming fields overwrite, deck-only fields are kept, and `changed` lists the fields;
    - incoming duplicates fold, listed in `merged`;
    - missing entries are kept, listed in `gone`;
    - a new entry keeps the incoming `id`, else gets a generated one;
  - generated keys:
    - `bonaca2020`, `bonaca2020a` on a clash;
    - a literal author (`Astropy Collaboration` → `astropycollaboration…`);
    - ASCII folding (`Martínez-Delgado` → `martinezdelgado`);
    - `nd` without a year.
- [X] T007 [US2] Rewrite `crates/qd/src/bib.rs` around CSL-JSON `Value`s (data-model "Reference entry"):
  - `normalise_doi`, `normalise_url`, `normalise_title`, `key_for`, `merge -> Merge`;
  - keep today's `fetch`, `lines` and `patch` beside them for now (they go in T037).

  T006 passes.
- [X] T008 [P] [US2] Write failing CLI tests in `crates/qd/tests/bib.rs` (skip without quarto), on a temp deck and `.bib`/`.json` fixtures in `crates/qd/tests/fixtures/bib/`:
  - **`list_none_and_missing`**: `qd bib` reports `where: none`, and lists a cited key with no entry under `missing`.
  - **`pull_creates_companion`**:
    - the first `pull refs.bib` creates `references.bib` beside the deck, or `_talk/references.bib` when `_talk/` exists;
    - it sets `bibliography:`, and the file starts with the header comment;
    - its JSON matches the contract (`added`, `created: true`);
    - the deck's bytes outside the front matter are unchanged.
  - **`pull_merges_and_keeps_keys`**: pulling an edited copy keeps the keys and lists `changed` fields and `gone`.
  - **`pull_json_and_biblatex`**: CSL-JSON and BibLaTeX inputs both work.
  - **`embed_extract_convert_roundtrip`**:
    - embed, then extract `--format json`, then convert `--format bib`;
    - after each step, `quarto render` gives the same citation text, and `bibliography:`/`references:` are as in the contract;
    - convert removes the old file.
  - **`read_only_outside`**: with `bibliography: ../shared/refs.bib`, `pull` into it, `convert` and `extract --to ../shared/x.bib` fail with `E_READ_ONLY`, while `pull ../shared/refs.bib` into the deck works.
  - **`ambiguous_companions`**: two `bibliography:` files inside the folder make a write fail with `E_AMBIGUOUS`.
  - **`dry_run_expect_undo`**:
    - `--dry-run` writes nothing and reports what would change;
    - a stale `--expect` gives `E_CONFLICT`;
    - `qd undo` after a pull restores both the deck and the companion file, the companion file's absence included;
    - `qd undo` after a convert restores the old file name and format.
- [X] T009 [US2] In `crates/qd/src/bib.rs`, read and write formats through `quarto pandoc` (research R5):
  - `read(path) -> Vec<Value>` (BibLaTeX for `.bib`, direct for `.json`);
  - `write(path, entries, format) -> String`, with the header comment for `.bib`;
  - a missing Quarto gives `E_NO_QUARTO`.
- [X] T010 [US2] In `crates/qd/src/bib.rs`, resolve the deck's references (data-model "Deck references"):
  - from the front matter, find the state (none, companion inside or outside, embedded, both) and the default place;
  - inside means the canonical path is under the deck's folder;
  - several writable files give `E_AMBIGUOUS`;
  - a write to an outside file gives `E_READ_ONLY` with the contract's hint.
- [X] T011 [US2] Implement `list`, `pull FILE`, `embed`, `extract [--to] [--format]` and `convert --format` in `crates/qd/src/cli.rs` and `crates/qd/src/bib.rs`:
  - front-matter changes reuse `ops`' front-matter patch (`qd meta set`'s);
  - `references:` is written as CSL YAML with `serde_yaml_ng`;
  - every write goes through `Store::write_many` (T003), with `--dry-run` and `--expect`;
  - JSON as in `contracts/cli.md`.

  T008 passes.
- [X] T012 [US2] In `crates/qd/src/server.rs`, also watch the file named by `bibliography:` (inside or outside the deck folder), so a pull that changes only the companion file re-renders the preview (Principle IV). Add a test in `crates/qd/tests/onslide.rs` or the server's watcher tests: changing the companion file triggers a reload event.
- [X] T013 [US2] Add `crates/qd/src/plugins.rs` (research R9):
  - `install(deck_dir) -> {installed, kept}`, with the version compare and the `W_EDITED` rule;
  - the list of compiled-in extensions is empty until T018 and T028.

  Wire `qd bib install` and the call from every `qd bib` write. Unit tests: absent, older, same, same-but-edited, newer, each with fake extension data passed in.

**Checkpoint**: PR 2. The deck's references can be pulled, embedded, extracted, converted and undone. The old `--zotero` still works.

---

## Phase 4: User Story 1 — Citations that link to the paper (P1) 🎯 MVP (PR 3)

**Goal**: `@key` reads as today, links to the paper, with an icon and the title on hover. `[[@key]]{.corner}` gives the corner form. Decks render with plain Quarto.

**Independent Test**: spec US1; quickstart §2.

- [ ] T014 [P] [US1] Add `plugins/cite-links/example.qmd` and `plugins/cite-links/references.bib` (CSL-JSON is fine too). The references:
  - 1, 2 and 3 authors;
  - two papers by the same first author in the same year;
  - an arXiv entry with a 10.48550 DOI and a URL;
  - an entry with neither DOI nor URL;
  - a `literal` author.

  The citations: narrative, parenthetical, two keys with a locator, and the corner form.
- [ ] T015 [US1] Write a failing test in `crates/qd/tests/plugins.rs`, **`cite_links_example`**, which renders `plugins/cite-links/example.qmd` with plain Quarto and checks the HTML:
  - the labels "Name (2021)", "A & B (2021)", "A et al. (2021)", and the "a"/"b" suffixes;
  - each `a.cite-link` has the DOI href, or the URL for the arXiv entry, and a `title` of "Label: Title";
  - the two-key citation has two links, with the locator outside them;
  - the corner link has class `cite-corner` and no label text;
  - the entry without a link renders plain, and the render's stderr has `cite-links: no DOI or URL for`;
  - with `cite-links: {icon: false, link: references}` in a second render, there is no icon and the href is `#ref-KEY`;
  - with `csl:` set in the deck, the deck's style wins.
- [ ] T016 [US1] Write `plugins/cite-links/_extensions/cite-links/talk.csl` from scratch (research R2):
  - **citations**: author-date, narrative "Name (year)" / "A & B (year)" / "A et al. (year)", `disambiguate-add-year-suffix`;
  - **bibliography**: authors, year, title, container, DOI or URL.

  Note in a comment at the top that it is our own work and not derived from any CC BY-SA style.
- [ ] T017 [US1] Write the rest of `plugins/cite-links/_extensions/cite-links/`, following `contracts/extensions.md`:
  - `_extension.yml` (version 1.0.0, filter at `post-quarto`);
  - `cite-links.lua`:
    - set `meta.csl` when absent, run citeproc once and clear the metadata (R1);
    - wrap each key's part of each `Cite` in a link with the icon and title, and make the corner form;
    - warn for unlinked keys;
  - `cite-links.css`: the icon from `paper.css`'s mask SVG, renamed classes, and the corner CSS.

  T015 passes.
- [ ] T018 [US1] Compile `cite-links` into `crates/qd/src/plugins.rs` (`include_str!` of each file). Add `plugins/cite-links/README.md` covering markup, options and the licence note. Extend `crates/qd/tests/bib.rs` **`pull_creates_companion`**: the pull installs `_extensions/cite-links`, and a deck citing a pulled key renders linked.
- [ ] T019 [US1] Run the offline check on the rendered example (`qd check --offline` on `plugins/cite-links/example.qmd`) in `crates/qd/tests/plugins.rs`: no CDN, and the icon is inline.

**Checkpoint**: PR 3. A deck cites natively and looks as it does today.

---

## Phase 5: User Story 3 — Pull from Zotero (P2) (PR 4)

**Goal**: `qd bib pull --from zotero KEY` through the first source plugin, `qd-zotero`.

**Independent Test**: spec US3; quickstart §3.

- [ ] T020 [US3] Add the crate `plugins/zotero/` (`Cargo.toml`: package `qd-zotero`, bin `qd-zotero`, with deps `ureq` and `serde_json` at the workspace's versions) and add it to the root `Cargo.toml`'s `members`. Add `plugins/zotero/qd-plugin.json` (data-model) and `plugins/zotero/README.md` covering install, usage and errors.
- [ ] T021 [P] [US3] Write failing tests in `plugins/zotero/tests/zotero.rs`, against a mock server (moved from the existing Zotero mock in `crates/qd/tests`) with `QD_ZOTERO_URL`:
  - paging past 100 items;
  - attachments and notes skipped;
  - the CSL-JSON is printed;
  - `id` from `citationKey`, else from `extra`'s `Citation Key:`;
  - an arXiv DOI dropped when a URL exists;
  - a bad key exits 2 with `E_INVALID_ARG` and makes no request;
  - an unreachable server exits 2 with `E_ZOTERO` and the hint, in under 5 s.
- [ ] T022 [US3] Implement `plugins/zotero/src/main.rs` (research R8):
  - the moved `fetch`, with `include=data,csljson` and a 5 s connect timeout;
  - the key check, the citekey and the arXiv rule;
  - errors as `CODE message` plus the hint on stderr.

  T021 passes.
- [ ] T023 [P] [US3] Write failing CLI tests in `crates/qd/tests/bib.rs` with a stub source (a script `qd-stub` put first on `PATH` by the test):
  - **`pull_from_source`**: it merges the stub's CSL-JSON and keeps the stub's `id`s for new entries, and the JSON has `source`;
  - **`pull_from_missing_source`**: `E_SOURCE` naming `qd-nope`, with the install hint;
  - **`pull_from_failing_source`**: a non-zero exit gives `E_SOURCE` with the stub's stderr, and invalid JSON gives `E_SOURCE`; the deck and companion file are unchanged in both.
- [ ] T024 [US3] Implement `pull --from NAME ARGS…` in `crates/qd/src/bib.rs` and `cli.rs` (research R7, `std::process::Command`, no shell). T023 passes.
- [ ] T025 [US3] One test across the crates, `zotero_end_to_end` in `crates/qd/tests/bib.rs`: build `qd-zotero` (`cargo build -p qd-zotero` via `env!("CARGO")`, or skip if not built), put it on `PATH`, start the mock, and check that `qd bib pull --from zotero ABCD1234` fills the deck's references.

**Checkpoint**: PR 4. Zotero refreshes a deck in one command.

---

## Phase 6: User Story 4 — The References slide (P2) (PR 5)

**Goal**: compact, short and full styles, 1–3 columns, three sizes, scrolling, and an overflow warning.

**Independent Test**: spec US4; quickstart §4.

- [ ] T026 [P] [US4] Add `plugins/refs-slide/example.qmd` and its references: one References slide per style, a 1-column and a 2-column one, one at each size, and one long `.scrollable` list. Reuse the clash pair from T014 so the compact suffixes can be checked.
- [ ] T027 [US4] Write a failing test in `crates/qd/tests/plugins.rs`, **`refs_slide_example`**, which renders the example with plain Quarto and checks:
  - the `#refs` classes (`refs-compact|short|full`, `refs-cols-N`, `refs-size-…`), with defaults compact, 3, smaller;
  - every entry has a link to its DOI or URL;
  - compact labels equal what `@key` renders, the a/b suffix included (cited in the same example);
  - short has "Authors (year), *Venue*";
  - full is citeproc's entry;
  - `id="ref-KEY"` is kept.
- [ ] T028 [US4] Write `plugins/refs-slide/_extensions/refs-slide/` (`_extension.yml`, `refs-slide.lua`, `refs-slide.css`), following `contracts/extensions.md`:
  - the same run-citeproc-once step as `cite-links` (R1);
  - the compact labels from a scratch citeproc run (R3);
  - short from `pandoc.utils.references`;
  - classes on `#refs`; CSS for the columns and sizes.

  Compile it into `crates/qd/src/plugins.rs`, and add `plugins/refs-slide/README.md`. T027 passes.
- [ ] T029 [US4] Overflow (research R10): in `crates/qd/tests/plugins.rs`, the rendered check on the example warns for a long non-scrollable References slide and not for the `.scrollable` one. If the existing check misses either case, fix it in `crates/qd/src/offline.rs`.

**Checkpoint**: PR 5. The References slide looks as the presenter picks.

---

## Phase 7: User Story 5 — Convert every `paper` deck, and remove `paper` (P3) (PR 6)

**Goal**: `qd bib migrate`, with a backup and the before/after check (`--accept` after review). The repository's decks and the PPP talk are converted, and `paper` is removed.

**Independent Test**: spec US5; quickstart §5–§6.

- [ ] T030 [P] [US5] Write failing unit tests in `crates/qd/src/migrate.rs`:
  - shortcode parsing: the arguments, `corner="true"`, an unknown argument giving `E_INVALID_ARG` with the slide;
  - an entry built from a shortcode (data-model "Paper shortcode");
  - the Makefile's `qd bib --zotero KEY` line read;
  - the rewrite patches:
    - inline → `@key`, corner → `[[@key]]{.corner}`;
    - the marker list → `::: {#refs refs-style="compact" refs-columns="3"}`/`:::`;
    - bytes elsewhere unchanged;
  - the HTML comparison: per slide, `(href, text, title)` for `a.paper-ref` before and for `a.cite-link` and `#refs` links after, with each kind of difference (`text`, `href`, `title`, `missing`, `extra`).
- [ ] T031 [P] [US5] Write failing CLI tests in `crates/qd/tests/migrate.rs` (skip without quarto), on fixtures in `crates/qd/tests/fixtures/migrate/`: a small `paper` deck in a temp git repo, with `_extensions/paper`, a Makefile and a marker list, whose shortcode labels match their entries.
  - **`migrate_clean`**:
    - `differences: 0`, no shortcodes left;
    - `_extensions/paper` removed, both extensions installed;
    - the Makefile's `refs` line is `qd bib pull --from zotero KEY`;
    - the companion file created, and the deck renders with plain Quarto.
  - **`migrate_diff_fails_and_restores`**: one hand-written label makes it fail with `E_MIGRATE_DIFF`, every touched file is byte-identical to before, and the report at `report` lists the difference.
  - **`migrate_accept`**: the same deck with `--accept` keeps the result, and the report still lists the difference.
  - **`migrate_unmatched_url`**: a shortcode whose URL is arXiv where the entry has a DOI matches by title; one matching nothing gets a built entry, listed under `unmatched`.
  - **`migrate_needs_backup_outside_git`**: a non-git folder without `--backup` gives `E_INVALID_ARG`; with `--backup DIR`, the files are copied there first.
  - **`migrate_from_stub_source`**: with `--from zotero KEY` and a stub `qd-zotero` on `PATH`, the entries come from it.
- [ ] T032 [US5] Implement `crates/qd/src/migrate.rs` and `qd bib migrate [--from] [--backup] [--accept]` in `cli.rs` (research R11):
  - the backup first;
  - the references from the source or the shortcodes, and the merge;
  - the patches;
  - the install;
  - the Makefile change;
  - the before and after renders through `crate::render`;
  - the comparison and the report in the cache;
  - the restore on a difference unless `--accept`.

  Writes go through `Store::write_many`. T030 and T031 pass.
- [ ] T033 [US5] Convert `tests/skills/quarto-revealjs/fixture/lessons.qmd` with `qd bib migrate`, review its report, and update `tests/skills` expectations that mention `paper`.
- [ ] T034 [US5] Convert the skill's scaffold in `skills/quarto-revealjs/assets/`:
  - `presentation.qmd` cites with `@key` and has the `#refs` slide;
  - the `Makefile`'s `refs` target becomes `qd bib pull --from zotero $(ZOTERO)`;
  - `_extensions/paper` is deleted, and decks get the extensions from `qd bib install`.

  Update `skills/quarto-revealjs/SKILL.md` and its references on citations.
- [ ] T035 [US5] Convert the PPP talk (quickstart §5). This needs the person.
  1. Copy the folder to `2026-PPP Talk.backup-2026-11-13` beside it.
  2. Ask the person to open Zotero. Confirm that `qd-zotero S376YTIE` works against the real Zotero, which also checks `include=csljson` (R8; fall back as R8 says if not).
  3. Run `qd bib migrate --from zotero S376YTIE --backup … --deck ….`
  4. Give the person the report (expect about 11 label and 9 link differences).
  5. Run with `--accept` only on their word, quoted.
  6. Render the talk with plain `quarto render` and open it in the browser pane for them.
- [ ] T036 [US5] Remove `plugins/paper/` and the `skill_copy_of_paper_matches_the_plugin` test, and the `paper` branch of `plugin_examples_render_with_plain_quarto`, in `crates/qd/tests/plugins.rs`. Update `plugins/README.md`.
- [ ] T037 [US5] Remove `qd bib --zotero`, `bib::fetch`, `bib::lines`, `bib::patch` and the marker constants from `crates/qd/src/bib.rs` and `cli.rs`, along with their tests, and the old Zotero mock test in `crates/qd/tests`. Keep `E_ZOTERO`'s text only where `qd-zotero` uses it. `migrate` still reads the old Makefile line (T030).
- [ ] T038 [US5] Run quickstart §6's grep. No `paper` shortcode, extension, `--zotero` or marker list is left outside `specs/` and migrate's own parser and fixtures.

**Checkpoint**: PR 6. `paper` is gone, and every deck cites natively.

---

## Phase 8: Polish & cross-cutting

- [ ] T039 [P] Docs:
  - `skills/quartz-deck/SKILL.md`: `qd bib` commands, source plugins, migrate;
  - `README.md`: citations;
  - `design/code-audit-plugins.md`: G4 note on source plugins;
  - the list of what changed for the nstarman-skills-small marketplace copy, in PR 6's description.
- [ ] T040 Run quickstart §1–§6 and record it in `specs/014-citations/checklists/quickstart-run.md`.
- [ ] T041 Final constitution re-check in `specs/014-citations/plan.md`. fmt, clippy (`--workspace --all-targets -D warnings`), `cargo test --workspace --no-fail-fast` and `node --test vscode/test/*.test.js` green. CI runs `cargo test --workspace`; update `.github/workflows/ci.yml` if it doesn't.
- [ ] T042 Concision audit (`/pr-audit`), the last task before each PR is marked ready.

---

## Dependencies & Execution Order

- **T001 (PR 1)** before anything else.
- **Foundational (T002–T005)** → every story.
- **US2 (T006–T013)** after Foundational. Every later story uses `qd bib`.
- **US1 (T014–T019)** after US2 (T013's `install`, and T008's pull test extended in T018).
- **US3 (T020–T025)** after US2 (T024 uses `merge` and the pull path). It is independent of US1.
- **US4 (T026–T029)** after US1 (the shared citeproc step and the label check against `cite-links`).
- **US5 (T030–T038)** after US1, US3 and US4. T035 needs the person (Zotero open, the report reviewed). T036–T038 come after T033–T035.
- **Polish (T039–T042)** at the end. T042 runs before every PR.

## Parallel opportunities

- T004 ∥ T002 (different files).
- T006 ∥ T008 (unit and CLI tests).
- T014 ∥ T016 (the example deck and `talk.csl`).
- T021 ∥ T023 (`qd-zotero` tests and the stub-source tests).
- After US2, US1 and US3 can be built side by side. US3's files (`plugins/zotero`, `bib.rs` pull-from) barely touch US1's (`plugins/cite-links`, `plugins.rs`).
- T030 ∥ T031.

## Implementation strategy

- **MVP**: PR 1 (constitution), PR 2 (US2) and PR 3 (US1). A deck can then
  cite natively from any exported file, and it looks as it does today.
- **Then**: PR 4 (Zotero), PR 5 (the References slide), and PR 6
  (migrate, the PPP talk, removing `paper`).

Each PR is opened after `/pr-audit` (T042), and merged only after the
person's "merge when green".
