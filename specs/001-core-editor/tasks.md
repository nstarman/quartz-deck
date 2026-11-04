---
description: "Task list for 001-core-editor"
---

# Tasks: Core Editor — Quarto reveal.js GUI + LLM-Friendly CLI

**Input**: Design documents from `specs/001-core-editor/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/cli.md, contracts/http.md, quickstart.md

**Tests**: Required — constitution Principle VI (test-first core). Write each
test task first and confirm it fails before the implementation task after it.

**Organization**: by user story (spec.md priorities P1–P5). Single crate;
paths are relative to the repo root.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: parallelizable (different files, no dependency on an incomplete task)
- **[Story]**: US1–US5 from spec.md

---

## Phase 1: Setup

- [X] T001 Create `Cargo.toml` (package `quartz-deck`, bin `qd`, edition 2024, `rust-version = "1.85"`) with deps `clap` (derive), `serde` (derive), `serde_json`, `serde_yaml_ng`, `sha2`, `similar`, `thiserror`; dev-deps `insta`, `assert_cmd`, `tempfile`; empty `src/main.rs` and `src/lib.rs`; `cargo build` passes
- [X] T002 [P] Add `.gitignore` (`/target`, `*.html` render outputs under `tests/`, `*_files/`), `rustfmt.toml` (defaults), and `cargo clippy -- -D warnings` passing on the empty crate
- [X] T003 [P] Create `tests/corpus/` with ≥ 20 reveal.js `.qmd` decks covering research.md R1/R2 constructs (`##` and `#` slides, `---` slides, `slide-level: 3`, fenced divs nested `::::`/`:::`, `.columns`, `.notes`, `. . .`, `{r}`/`{python}` cells, raw HTML, `$$` math, tables, `.absolute` divs and images, `{{< include >}}`, no front matter, CRLF line endings, no trailing newline). Use only hand-written decks or decks from MIT-licensed sources (e.g. quarto-cli test fixtures); record source + license per file in `tests/corpus/README.md` (CLEANROOM.md) — done: 23 hand-written decks (no third-party sources needed)
- [X] T004 [P] Add a license check: `cargo metadata --format-version 1` piped through a test `tests/licenses.rs` that fails if any dependency license is not in the allowlist {MIT, Apache-2.0, BSD-2/3-Clause, ISC, Zlib, Unicode-3.0, CC0-1.0, MPL-2.0} (FR-041, research.md R8)

---

## Phase 2: Foundational (blocks every story)

- [X] T005 Define errors in `src/error.rs`: `enum QdError` with the 11 codes from contracts/cli.md (`E_NO_DECK`, `E_PARSE`, `E_NOT_FOUND`, `E_AMBIGUOUS`, `E_UNSAFE_EDIT`, `E_CONFLICT`, `E_INVALID_ARG`, `E_NO_UNDO`, `E_NO_QUARTO`, `E_RENDER`, `E_NO_SERVER`), each with `code()`, message, `hint`; serializes as `{"error":{"code","message","hint","details?"}}`
- [X] T006 [P] Write failing tests in `src/attrs.rs` (`#[cfg(test)]`): parse/print Pandoc attribute blocks `{#id .c1 .c2 k=v k2="a b"}`; printing an unmodified `Attrs` returns the original text byte-for-byte; set/unset key, add/remove class, set id preserve order of untouched items
- [X] T007 Implement `Attrs { id: Option<String>, classes: Vec<String>, kv: Vec<(String,String)> }` in `src/attrs.rs` with original-text retention so T006 passes
- [X] T008 [P] Write failing round-trip test `tests/roundtrip.rs`: for every `tests/corpus/*.qmd`, `Deck::parse(text)` then concatenate front matter + slide spans → must equal input bytes (SC-002); and insta snapshot of `deck.outline()` (slide ids, kinds, titles, block kinds) per corpus file
- [X] T009 Write failing unit tests in `src/deck.rs` for segmentation rules from research.md R1/R2: slide starts at heading of level `slide-level` (default 2) or `---`; level-1 heading is `section`; `#`-lines inside fenced code/fenced divs/HTML blocks do not start slides; nested fenced divs close by colon count; unterminated fence marks the slide `opaque: true`; `slide-level` read from front matter
- [X] T010 Implement lossless segmenter in `src/deck.rs`: `Deck { path, version, front_matter: Option<Span>, slide_level: u8 (default 2), width: u32 (default 1050), height: u32 (default 700), slides: Vec<Slide> }`, `Slide { id, id_explicit, index (1-based; YAML title slide = 0), kind: heading|rule|section, title: Option<String>, attrs, span, blocks, opaque }`, `Block { id, kind: para|list|code|cell|div|image|table|quote|html|math|pause|notes|box, attrs: Option<Attrs>, span, text }`; invariant "front matter + slide spans == file bytes"; slide IDs per research.md R3 (explicit `{#id}` → Pandoc auto-identifier from title → `@N`); element IDs explicit `#id` or `<slide>/<i>`; T008 and T009 pass
- [X] T011 Implement ref resolution in `src/deck.rs`: slide refs (explicit id, auto-id, `@N`, `first`, `last`) and element refs (`<slide>/<i>`, `#id`) → `E_NOT_FOUND` / `E_AMBIGUOUS` (duplicate IDs); unit tests for each case
- [X] T012 [P] Write failing tests in `src/store.rs` (tempdir): atomic write via temp file + rename in same dir; `version` = first 16 hex of SHA-256 of bytes; write refuses with `E_CONFLICT` if file bytes changed between read and write or `expect` ≠ current version; undo journal JSONL entries `{ts, op, version_before, version_after, before_text}` in `<cache>/<sha256(path)[..16]>/undo.jsonl`; `undo` valid only when current version == last entry's `version_after`, else `E_NO_UNDO`; undone entries move to `redo.jsonl` (with `after_text`), `redo` valid only when current version == top redo entry's `version_before`; any new edit clears `redo.jsonl`
- [X] T013 Implement `src/store.rs` (cache dir = `$QD_CACHE_DIR` or `~/Library/Caches/quartz-deck` / `$XDG_CACHE_HOME/quartz-deck`) so T012 passes
- [X] T014 Implement `Patch { op, replacements: Vec<(Span,String)>, assigned_ids }` + `apply(text, &Patch) -> String` (non-overlapping, applied right-to-left) + `unified_diff(before, after)` via `similar` in `src/ops.rs`; unit tests for overlap rejection

**Checkpoint**: decks parse losslessly; safe writes and undo exist.

---

## Phase 3: User Story 1 — LLM edits a deck through the CLI (P1) 🎯 MVP

**Goal**: every slide-level edit + read command from contracts/cli.md, JSON in/out, plus the Skill.

**Independent Test**: quickstart.md §1, §2, §5 pass with no Quarto installed.

### Tests (write first, must fail)

- [X] T015 [P] [US1] Op snapshot tests in `tests/ops.rs` (insta, on a fixture deck `tests/fixtures/basic.qmd`): `slide.add` (end, `--after`, `--before`), `dup`, `move`, `rm`, `title` (incl. `""` → `## {#id}`), `body`, `set` (attr set/unset, class add/remove), `edit` element, `meta set/unset`; each asserts resulting text AND that the diff touches only the target slide (plus front matter for `meta`) (FR-003, SC-003)
- [X] T016 [P] [US1] ID assignment tests in `tests/ops.rs`: editing a slide with only an auto-id writes `{#<auto-id>}` to that heading only; `---` slide becomes `## {#s-xxxx}`; new slides get `s-<4 hex>` unique in deck; `assigned_ids` reported (research.md R3)
- [X] T017 [P] [US1] Refusal tests in `tests/ops.rs`: structural edit inside an `opaque` slide → `E_UNSAFE_EDIT`; bad attr syntax → `E_INVALID_ARG`; file unchanged after every error
- [X] T018 [P] [US1] CLI integration tests in `tests/cli.rs` (assert_cmd, tempdir): each read/edit command in contracts/cli.md prints one JSON object on stdout with documented keys; `--dry-run` prints diff and leaves file untouched; `--expect` mismatch → exit 1 + `E_CONFLICT` JSON on stderr; usage error → exit 2; `qd undo` restores previous bytes; `DECK` defaults to sole `*.qmd` in cwd or `$QD_DECK`, else `E_NO_DECK`
- [X] T019 [P] [US1] Perf test in `tests/cli.rs` (`#[ignore]` by default, run in quickstart): `qd slides` and `qd move` on a generated 100-slide deck each < 200 ms (SC-004)

### Implementation

- [X] T020 [US1] Implement slide ops in `src/ops.rs` as pure functions `fn(&Deck, &str, args) -> Result<Patch, QdError>`: `add`, `dup` (fresh IDs for slide and boxes), `move`, `rm`, `title`, `body` (replaces all non-box blocks), `set`, `edit`, `meta_set`/`meta_unset` (YAML edit limited to the touched key's lines; refuse with `E_UNSAFE_EDIT` if the key spans complex YAML) — T015–T017 pass
- [X] T021 [US1] Implement `src/cli.rs` with clap: commands and flags exactly as contracts/cli.md (read: `slides`, `show`, `meta`, `check` (static checks only here: duplicate IDs, opaque slides, missing image/include paths), `schema`, `skill`; edit: `add`, `dup`, `move`, `rm`, `title`, `body`, `set`, `edit`, `meta set|unset`, `undo`, `redo`, `new`); global `--human`, edit flags `--dry-run`, `--expect`; body inputs `--md`, `--file`, `-` (stdin); success JSON `{"ok","op","version","diff","assigned_ids","target"}`; errors to stderr; wire into `src/main.rs` — T018 passes — done: deck chosen by global `--deck`/`$QD_DECK` instead of a positional arg (avoids ambiguity with slide refs)
- [X] T022 [US1] Implement `qd new <path> [--title T]` writing a minimal deck (`title`, `format: revealjs`, one `## {#s-xxxx}` slide) in `src/cli.rs`
- [X] T023 [US1] Implement `qd schema` in `src/cli.rs`: JSON Schema (via `schemars`, add dep) of every command's args and output structs; test in `tests/cli.rs` that every subcommand in clap appears in the schema — done via clap introspection instead of `schemars` (no extra dependency)
- [X] T024 [US1] Write `skills/quartz-deck/SKILL.md`: frontmatter `name`/`description`; the loop inspect (`qd slides`, `qd show`) → edit (with `--expect`, `--dry-run`) → `qd check` → look at preview (`qd url <slide>`); ref syntax; error-code table with recovery for each; 3 worked examples (add+move, retitle+background, positioned image); embed with `include_str!` and implement `qd skill [--install DIR]` in `src/cli.rs`
- [X] T025 [US1] Skill/CLI sync test in `tests/cli.rs`: every `qd <cmd>` mentioned in `skills/quartz-deck/SKILL.md` exists in `qd schema` (FR-014)

**Checkpoint**: MVP — an agent can fully edit decks via CLI. Run quickstart §1, §2, §5.

---

## Phase 4: User Story 2 — Live browser preview (P2)

**Goal**: `qd serve` renders via Quarto and live-reloads on any file change.

**Independent Test**: quickstart.md §3.

### Tests

- [X] T026 [P] [US2] Unit tests in `src/render.rs` for the preview copy (research.md R5): each slide-level top-level block wrapped in `::: {data-qd="<slide>/<i>"}`, blocks that already have attrs get `data-qd=…` inserted into their own attr block, nothing inside divs wrapped; untouched corpus deck renders (test gated on `quarto` on PATH) with `data-qd` attributes present and slide count equal to `Deck.slides.len()` (+1 if YAML title)
- [X] T027 [P] [US2] Server integration test `tests/serve.rs` (gated on quarto): start `qd serve --port 0 --detach`, `GET /api/deck` equals `qd slides`, editing the file via `qd title` produces SSE `changed` then `rendered` within 3 s (SC-005); appending an unterminated fence yields `render_error` and `/preview/` still serves the previous output

### Implementation

- [X] T028 [US2] Implement `src/render.rs`: locate `quarto` (else `E_NO_QUARTO` with install hint `https://quarto.org/docs/get-started/`); `preview_copy(&Deck) -> String`; render the copy into `<cache>/preview/` with `quarto render <copy> --to revealjs --output-dir <cache>/preview` run from the deck's directory so relative assets and `_quarto.yml` resolve (copy written as hidden sibling `.<name>.qd-preview.qmd`, removed after render); capture stderr as `E_RENDER` details
- [X] T029 [US2] Add deps `axum`, `tokio` (rt-multi-thread, macros, fs, process), `notify` (verify license per T004), `tower-http` (fs) and implement `src/server.rs` per contracts/http.md: bind `127.0.0.1:<port>` only; `GET /preview/*` serves last good render; `GET /present/` serves deck without editor; `GET /api/deck`, `GET /api/slide/{ref}`; `GET /events` SSE (`changed {version}`, `rendering`, `rendered {version}`, `render_error {message}`); file watcher → 150 ms debounce → single in-flight render, latest wins — done; also `qd stop` added (agents can stop a detached server)
- [X] T030 [US2] Implement `qd serve [--port N] [--open] [--detach]` and `qd url [<slide>]` in `src/cli.rs`; session file `<cache>/session.json` `{deck_path, url, port, pid}`; stale session (dead pid) → `E_NO_SERVER`; `qd url s-1` → `http://127.0.0.1:PORT/#/s-1`
- [X] T031 [US2] Minimal `src/ui/index.html` + `src/ui/editor.js` (embedded with `include_str!`): iframe on `/preview/<deck>.html`, `EventSource('/events')` reloads iframe on `rendered` keeping `#/<slide-id>` hash, shows `render_error` banner (FR-021) — done; plus a read-only slide list for navigation
- [X] T032 [US2] Extend `qd check` in `src/cli.rs` to run a Quarto render to a temp dir and report `E_RENDER` problems when Quarto exists — done as opt-in `qd check --render` (keeps the default check fast)

**Checkpoint**: CLI edits appear in the browser automatically.

---

## Phase 5: User Story 3 — Structural and text editing in the GUI (P3)

**Goal**: slide list, click-to-select, text edit, properties panel, undo/redo — all via `POST /api/op`.

**Independent Test**: quickstart.md §4 (first two bullets); GUI diffs equal CLI diffs.

### Tests

- [X] T033 [P] [US3] Test in `tests/serve.rs`: `POST /api/op` with each US1 op returns byte-identical JSON and file result to the CLI command with the same args; unknown op → `E_INVALID_ARG` (FR-010)

### Implementation

- [X] T034 [US3] Implement `POST /api/op` in `src/server.rs`: body `{"op","args","expect"}` dispatches to the same functions `src/cli.rs` uses (refactor command handlers into `fn run_op(op, args) -> Result<Value, QdError>` shared by both) — done: `run_op` rebuilds the argv and parses it with clap (no handler refactor needed); JSON-only + loopback Host/Origin guard
- [X] T035 [US3] Slide list panel in `src/ui/editor.js` + `src/ui/editor.css`: titles from `/api/deck`, drag-to-reorder (HTML5 drag events) → `move`; buttons add/duplicate/delete → `add`/`dup`/`rm`; click → navigate preview to slide
- [X] T036 [US3] Click-to-select in `src/ui/editor.js`: in the same-origin iframe, map click target to nearest `[data-qd]` or `[id]` element → highlight outline; double-click opens a textarea with the block's source from `/api/slide/{ref}`; confirm → `edit`
- [X] T037 [US3] Properties panel in `src/ui/editor.js`: for slide: title, background-color/image, classes, transition → `title`/`set`; for element: classes/attrs → `set`/`edit`; all writes pass `expect` = last seen version and on `E_CONFLICT` refetch and show a notice — done; element attributes are edited through the element's source text (attrs are part of it)
- [X] T038 [US3] Undo/redo in `src/ui/editor.js`: Cmd/Ctrl+Z → `undo` op, Cmd/Ctrl+Shift+Z → `redo` op (both CLI commands, FR-010) — done (`qd redo` already exists in the CLI)
- [X] T039 [US3] Browser verification (agent browser tool / Playwright) per quickstart §4 bullets 1–2; record result in `specs/001-core-editor/checklists/gui.md`

**Checkpoint**: humans can do all slide/text editing in the browser.

---

## Phase 6: User Story 4 — Drag and resize positioned boxes (P4)

**Goal**: `.absolute` boxes via CLI and drag/resize in the GUI.

**Independent Test**: quickstart.md §4 bullets 3–4.

### Tests

- [ ] T040 [P] [US4] Op snapshot tests in `tests/ops.rs`: `box add` (text → `::: {#b-xxxx .absolute left=… top=… width=… height=…}`; image → `![](path){#b-xxxx .absolute …}`), `box move --at`/`--by`, `box resize`, `box rm`; values written as unitless integers; box with `%`/`em` units is converted to px on move and result includes `"converted_units": true` (research.md R4); geometry parse treats unitless as px
- [ ] T041 [P] [US4] Position fidelity test (gated on quarto) in `tests/serve.rs`: render a deck with a box at 200,100 size 300×50 and assert inline style `top: 100px; left: 200px; width: 300px; height: 50px` (SC-006, spike in research.md R5)

### Implementation

- [ ] T042 [US4] Implement `Geom { top, left, width, height, bottom, right: Option<Length> }`, `Length { value: f64, unit }` and box ops in `src/ops.rs`; add `box add|move|resize|rm` to `src/cli.rs` and `qd show` output `geom` — T040 passes
- [ ] T043 [US4] Drag/resize in `src/ui/editor.js`: on `.absolute` elements show 8 handles; pointer events move/resize the element's inline style live; delta_slide_px = delta_screen_px / `Reveal.getScale()` (from iframe window); Shift keeps aspect ratio; on pointerup send `box move`/`box resize` with rounded ints
- [ ] T044 [US4] "Add box" buttons (text, image path prompt) in properties panel → `box add` at slide centre, 300×100 default
- [ ] T045 [US4] Browser verification per quickstart §4 bullets 3–4, plus `quarto render` stock comparison (±1 px); record in `specs/001-core-editor/checklists/gui.md`

**Checkpoint**: free layout works and renders identically with stock Quarto.

---

## Phase 7: User Story 5 — Render and export (P5)

**Goal**: `qd render` wraps Quarto; clean failure without Quarto.

**Independent Test**: `qd render` output equals `quarto render` output path; `PATH=/usr/bin:/bin qd render` → `E_NO_QUARTO`, while `qd slides` still works.

- [ ] T046 [P] [US5] Tests in `tests/cli.rs`: `qd render` returns `{"ok","output","log"}` and output file exists (gated on quarto); with quarto removed from PATH, `render`/`serve` → `E_NO_QUARTO` with hint and `slides`/`add` succeed (FR-031)
- [ ] T047 [US5] Implement `qd render [--to FORMAT]` in `src/cli.rs` using `src/render.rs` on the original file (not the preview copy) (FR-030)
- [ ] T048 [US5] Render button + "Open presentation" link (`/present/`) in `src/ui/index.html`

---

## Phase 8: Polish & Cross-Cutting

- [ ] T049 [P] Concurrency stress test `tests/concurrency.rs`: 3 threads running random `qd` edits plus an external writer for 10 s; assert file always parses, no write lost (every successful op's change present or later undone), conflicts reported as `E_CONFLICT` (SC-007)
- [ ] T050 [P] Layout drift check: for each corpus deck (gated on quarto), compare per-slide bounding boxes of top-level elements between preview copy and plain render via headless browser; document any drift and apply Lua-filter fallback from research.md R5 if found
- [ ] T051 [P] Update `README.md` with install (`cargo install --path .`), the quickstart loop, and `qd skill --install ~/.claude/skills/quartz-deck`
- [ ] T052 Agent evaluation (SC-001): 10 fresh Claude Code sessions given only the Skill perform the 5-step task in quickstart §6; record pass count in `specs/001-core-editor/checklists/agent-eval.md`; fix Skill/CLI wording until ≥ 9/10
- [ ] T053 Run full `specs/001-core-editor/quickstart.md`, `cargo test`, `cargo clippy -- -D warnings`; confirm constitution check in plan.md still holds

---

## Dependencies & Execution Order

- Phase 1 → Phase 2 → stories. T003 (corpus) needed by T008.
- **US1** depends only on Phase 2. **US2** needs Phase 2 + `run_op`-free read paths (can start in parallel with US1 after T011; T030/T032 touch `src/cli.rs` after T021).
- **US3** needs US1 (ops) + US2 (server, UI shell).
- **US4** needs US1 (ops/cli) for T040/T042; T043–T045 need US3's selection layer.
- **US5** needs T028 (render.rs) from US2.
- Polish after the stories it exercises.

```text
Setup → Foundational ─┬─ US1 ─┬─ US3 ─ US4(GUI)
                      │       └─ US4(CLI ops)
                      └─ US2 ─┴─ US5
```

## Parallel Examples

- Phase 1: T002, T003, T004 together.
- Phase 2: T006 ∥ T008 ∥ T012 (different files), then T007, T010, T013.
- US1 tests: T015, T016, T017, T018, T019 together (T015–T017 share `tests/ops.rs` — split into modules or write sequentially if editing one file).
- US2 ∥ US1 implementation once T011 is done: T026/T028 (render.rs) alongside T020 (ops.rs).
- US4 T040 can be written while US3 GUI work proceeds.

## Implementation Strategy

1. **MVP = Phase 1 + 2 + US1.** Stop and validate: quickstart §1, §2, §5; try the Skill with Claude Code.
2. Add US2 → the agent can watch its own edits in the browser.
3. Add US3, then US4 (CLI box ops first, GUI drag second), then US5.
4. Polish: concurrency, layout drift, agent eval.
