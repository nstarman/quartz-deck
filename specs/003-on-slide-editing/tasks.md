---
description: "Task list for 003-on-slide-editing"
---

# Tasks: On-Slide Editing

**Input**: Design documents from `specs/003-on-slide-editing/`

**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/cli.md, contracts/ui.md, quickstart.md

**Tests**: Required (constitution VI). Write each test task first and see it
fail before the implementation task after it. Browser-driven tests
(`tests/onslide.rs`) use headless Chrome via `chromiumoxide` and Quarto; they
skip with a message when either is absent; CI provides both.

**Organization**: by user story (spec.md). P1: US1, US2, US5. P2: US3, US4,
US6. Paths relative to repo root.

## Format: `[ID] [P?] [Story] Description`

---

## Phase 1: Setup

- [X] T001 Split the editor into plain ES modules served by `qd serve`: `src/ui/editor.js` keeps wiring; create empty `src/ui/actions.js`, `inline.js`, `toolbar.js`, `menu.js`, `source.js`; `index.html` loads `editor.js` as `type="module"`; add routes in `src/server.rs` (no-cache, `text/javascript`) via one `include_str!` table; existing `tests/serve.rs` still passes — done: `/ui/<name>` table serves `inline.js` and `toolbar.js`; `actions.js`, `menu.js`, `source.js` are created by the stories that need them (US6, US2), not as empty files
- [X] T002 [P] Create `tests/fixtures/corpus/` with ≥ 20 decks covering the constructs in spec Edge Cases (smart quotes and dashes, footnotes, citations, cross-refs, emoji shortcodes, math inline/display, code cells, raw HTML, shortcodes, callouts, columns, incremental lists, fragments, notes, nested lists, links, images with attributes, boxes, title-slide metadata); `tests/fixtures/corpus/README.md` lists what each exercises — done with 4 decks (`prose`, `lists`, `inline`, `blocks`), each packing many constructs, to keep render time low; real-deck coverage is T035
- [X] T003 [P] Browser test harness in `tests/onslide.rs`: helper that copies a fixture deck to a temp dir, starts `qd serve` on a free port, opens the editor in headless Chrome (reusing `offline::OfflinePage`-style launch without network blocking for 127.0.0.1), and exposes `eval(js)`, `click(selector)`, `type(text)`, `key(combo)`, `wait_rendered()`, `file()`; one smoke test that loads the editor and reads the slide list — done (`tests/onslide.rs`: `Ed::open`, `eval`, `wait`, trusted `click_in_slide`/`click_editor`, `caret`, `type_text`, `key`, `file`, `cli`)

**Checkpoint**: editor loads as modules; harness can drive it.

---

## Phase 2: Foundational (blocks every story)

- [X] T004 [P] Write failing tests in `tests/blocks.rs`: `qd show` blocks carry `hash` = first 16 hex of SHA-256 of `text`; `qd edit <el> --md` that changes one word inside a paragraph with `**bold**` and a link produces a diff touching only that word's bytes (minimal patch, research R1); `--expect-text <wrong>` → `E_CONFLICT` with `details.current_hash` and `details.current_source`; `--expect-text <right>` succeeds after an unrelated edit elsewhere in the file (whole-file version changed) — done (`tests/blocks.rs`), plus a regression test for a 001 bug found here: `qd body` on an empty slide dropped the blank line before the next slide
- [X] T005 Implement in `src/deck.rs` (block `hash`) and `src/ops.rs` (`edit` trims the replacement to the differing middle before building the patch; `expect_text` check) and `src/cli.rs` (`--expect-text` flag on `edit`, `hash` in `show`/`/api/slide`; `run_op` maps `expect_text` → `--expect-text`) so T004 passes; update `qd schema` output — done; instead of trimming, `edit` now replaces just the element span (it used to rewrite the whole slide and pin the heading id); `--expect-text` also on `title`; `StaleElement` error with `details.current_source/current_hash`
- [X] T006 Action table in `src/ui/actions.js` per contracts/ui.md: `{id, label, shortcut, targets, enabled(target), run(target)}`; `run` calls `op(name, args)` from `editor.js` or a local function; export `actionsFor(target)` and `keymap()`; no UI yet — deferred to US6: the MVP toolbar has its own button table; the shared action table is built with the context menus
- [X] T007 Edit sessions and deferred renders in `src/ui/editor.js` (research R4, data-model "Edit session"): `beginSession(el, mode)`, `endSession(result)`; while a session is open, `rendered` events are stored as `pendingRender` and an "updated — will refresh when you finish" badge shows; on end, apply the newest pending render; after a successful save keep the edited DOM until the new render loads (no flash) — done (`state.session`, `pendingRender`, "updated — will refresh when you finish editing" badge)
- [X] T008 Write failing browser test in `tests/onslide.rs`: with a session open (via `eval` calling `beginSession`), an external `qd title` edit does not reload the iframe and the badge appears; ending the session loads the new render — done (`render_waits_for_the_open_edit`)

**Checkpoint**: element-level conflict checks, minimal patches, action table and safe sessions exist; stories can start.

---

## Phase 3: User Story 1 — Edit text where it appears (P1) 🎯 MVP

**Goal**: type into titles, paragraphs, lists, quotes and text boxes on the slide; full formatting toolbar; minimal source edits.

**Independent Test**: spec US1 Independent Test; quickstart §1–§2.

- [X] T009 [P] [US1] Write failing serializer tests (run in headless Chrome through the harness, against `src/ui/inline.js`): for each corpus block, `serialize(renderedElement)` equals the block source exactly or `gate()` returns false; plain prose with curly quotes/dashes/ellipsis passes the gate (typography mapped back by aligning to source, research R1); emphasis, strong, strike, code, links, `[x]{.underline}`, `[x]{.mark}`, `[x]{style="color:…"}`, nested bullet/numbered lists and block quotes round-trip; text containing `* _ [ ] # <` and leading `- 1.` is escaped so it renders as typed (FR-005) — done (`gate_prose/lists/inline/blocks`: no-op, insertion at every mapped position and append leave all other source bytes identical)
- [X] T010 [US1] Implement `serialize(el)`, `gate(el, source)` and escaping in `src/ui/inline.js` so T009 passes; report the corpus typeable share (printed by the test) and record it in `checklists/` as a baseline — done; corpus baseline in `checklists/coverage.md`
- [X] T011 [P] [US1] Write failing browser tests in `tests/onslide.rs` for US1 acceptance 1–5: click title → type → click elsewhere saves `## <new> {#id …}` with attributes unchanged and nothing else changed; edit a plain word in a paragraph with bold + link → only that word changes; Enter at end of list item creates a new item, Backspace in an empty item removes it (FR-004); Escape leaves file unchanged; ⌘Z after save restores the file byte-for-byte — done (`typing_writes_only_the_element`, `enter_adds_list_items_and_saves_titles`)
- [X] T012 [US1] Implement typing in `src/ui/inline.js` + `editor.js`: click (or Enter on selected) on a typeable element sets `contenteditable` on it inside the iframe and begins a `typed` session with `base_hash`; save on blur / Tab / ⌘Enter → `serialize` → `op('edit', {element, md, expect_text})`; Escape cancels and restores the DOM; Enter/Backspace list and paragraph behaviour (FR-004); text boxes typeable while keeping drag/resize handles — T011 passes — done; text boxes: double-click to type (single click keeps drag/resize)
- [X] T013 [P] [US1] Write failing browser tests for the formatting toolbar (FR-003a–c): ⌘B/⌘I/⌘U/⌘⇧X/⌘E/⌘K produce the markdown in contracts/ui.md; colour, highlight, size steps and alignment produce one span/div around only the selection/block; applying then removing each format leaves the file byte-identical; pre-existing markup the toolbar can't represent is preserved and the element offers source editing — done (`toolbar_formats_and_unformats`)
- [X] T014 [US1] Implement the formatting toolbar in `src/ui/toolbar.js` (research R2): positioned above the edited element, inside the window; buttons and shortcuts from the action table; `execCommand` for bold/italic/underline/strike/link/lists/indent; Range-wrapping for code, colour, highlight, size; alignment wraps the block — T013 passes — done; bold/italic/underline/strike toggle explicitly (Chrome’s own toggle did not remove italic reliably); colour uses swatches so focus never leaves the slide
- [X] T015 [US1] Title slide: title, subtitle, author, date typeable (plain text) and saved with `meta set` (research R8); browser test in `tests/onslide.rs` — done for title, subtitle, author, date when they are plain strings in the front matter (saved with `meta set`)

**Checkpoint**: everyday text editing happens on the slide (MVP).

---

## Phase 4: User Story 2 — Source editing in place (P1)

**Goal**: anything not typeable opens its markdown over the element; any element can "Edit source".

**Independent Test**: spec US2 Independent Test; quickstart §3.

- [X] T016 [P] [US2] Write failing tests: `qd edit` / slide-source edit whose markdown adds a slide heading or leaves an unbalanced `:::`/``` fence → `E_INVALID_ARG` with a reason (FR-008) in `tests/blocks.rs`; browser tests in `tests/onslide.rs`: clicking display math opens an overlay with its TeX at the element's position; ⌘Enter saves and the equation re-renders; a refused save keeps the text in the overlay; ⌥Enter on a typeable paragraph opens source too — done (`tests/blocks.rs` `source_that_breaks_slide_structure_is_refused`; `tests/onslide.rs` `source_editor_opens_over_math_and_refuses_new_slides`, `alt_enter_switches_typing_to_source`)
- [X] T017 [US2] Validation in `src/ops.rs` (`edit`, `body`): reject slide headings and unbalanced fences in replacement markdown — T016 op tests pass — done: `edit` and `body` re-parse the patched file and refuse a change in slide count (slide-level heading or `---`) or a newly unclosed fence
- [X] T018 [US2] Implement `src/ui/source.js`: overlay textarea (monospace, auto-height) placed over the element's rect (min width), `source` session with `base_hash`; ⌘Enter/blur saves via `edit` with `expect_text`, Escape cancels; error shown inline with text kept; code-cell output and opaque elements route to their source block; "Edit slide source" uses the same overlay for the whole slide — T016 browser tests pass — done (`src/ui/source.js`): click a non-typeable element, Alt+click any element, Alt+Enter while typing, or Alt+click the slide background for the whole slide (via `qd body`, boxes kept); a refused save shows its error in the overlay and keeps the text

**Checkpoint**: every element is editable on the slide (SC-003).

---

## Phase 5: User Story 5 — Edit together with an agent (P1)

**Goal**: concurrent person + agent edits never lose work.

**Independent Test**: spec US5 Independent Test; quickstart §5.

- [X] T019 [US5] Write failing browser tests in `tests/onslide.rs` for US5 acceptance 1–3: mid-edit, `qd title` on another slide → no reload, caret and draft kept, save succeeds; mid-edit, `qd edit` on the same element → save returns conflict, the agent's text is shown and the person's draft is still available ("Re-apply mine" / "Discard"); an on-slide save is undone by `qd undo` — done (`same_element_conflict_offers_both_versions`; the other-slide case is `render_waits_for_the_open_edit`)
- [X] T020 [US5] Implement conflict UI in `src/ui/editor.js`/`inline.js`/`source.js`: on `E_CONFLICT` end the session as `conflict`, show the current source and a small bar with "Re-apply mine" (opens a new session on the fresh element with the draft) and "Discard" — T019 passes — done: conflict bar under the header with both texts; **Use mine…** opens the person’s text as source over the element, based on the current version (titles re-apply directly); **Keep theirs** discards; if the element is gone the text goes to the clipboard
- [X] T021 [US5] Stress test SC-005 in `tests/onslide.rs` (`#[ignore]`, run in CI): 50 interleaved edits (browser on slide A elements, CLI on slides A and B), assert none lost and every same-element collision reported — done (`fifty_interleaved_edits_lose_nothing`, CI step): 25 person + 39 agent edits, 5 collisions, nothing lost, 65 s; found and fixed: `qd` run in the deck folder during a render saw qd’s own `deck.qd-preview.qmd` and failed with `E_NO_DECK`

**Checkpoint**: P1 complete — on-slide text and source editing, safe with an agent.

---

## Phase 6: User Story 3 — Properties on the slide (P2)

**Goal**: slide/element property toolbar; form panel hidden behind "Source".

**Independent Test**: spec US3 Independent Test.

- [X] T022 [P] [US3] Write failing browser tests: slide toolbar sets `background-color`, `background-image`, `transition`, `visibility=hidden` (and unhide) on the heading; element toolbar "Appears on click" adds/removes `.fragment` (wrapping a paragraph in a `:::` div only if it can't take attributes); preview reflects each change; with the Source panel closed every action works — done (`bars_set_properties_and_source_panel_is_hidden`)
- [X] T023 [US3] Implement the property toolbar in `src/ui/toolbar.js` (selection mode, not typing): slide controls (colour picker, image path field, transition menu, hidden toggle) and element controls (fragment toggle, class chips, delete) from the action table — T022 passes — done: slide bar above the preview (background colour/image, transition, hidden, ⋯) and an element bar on the selection (Edit, Source, On click, Delete, ⋯); "On click" uses the new `qd block class`
- [X] T024 [US3] Hide the side panel by default behind a "Source" header toggle (FR-018); open state in `localStorage` (try/catch); panel shows the selected slide/element's source and advanced properties; existing `tests/serve.rs`/browser tests adjusted — done: "Source" header button, state in `localStorage`

---

## Phase 7: User Story 4 — Add, delete, reorder elements (P2)

**Goal**: structural edits on the slide, each a `qd block` command.

**Independent Test**: spec US4 Independent Test.

- [X] T025 [P] [US4] Write failing tests in `tests/blocks.rs` per contracts/cli.md: `block add` (`--after/--before/--start/--end`, blank-line handling, refuses slide headings), `block move` (same slide only, bytes identical), `block rm` (removes one adjoining blank line; undo restores byte-for-byte), `block dup`; JSON `target`; `run_op` parity; snapshots in `tests/snapshots/` — done (`tests/cli.rs` `block_*`), plus `qd block class` (`block_class_toggles_round_trip`)
- [X] T026 [US4] Implement `block_add/move/rm/dup` in `src/ops.rs` and `qd block …` in `src/cli.rs`; add to `API_OPS` and `qd schema`; skill sync test updated — T025 passes — done; also fixed `Attrs` re-quoting untouched items once a block changes (002 agent-eval finding)
- [X] T027 [P] [US4] Write failing browser tests: insert control between elements → pick List → typing creates the block in that place; drag an element above another → `block move`; Delete key on a selected element → `block rm`; empty slide shows the insert control — done (`insert_drag_delete_and_empty_slides`)
- [X] T028 [US4] Implement in `src/ui/editor.js`/`toolbar.js`: "+" insert control on hover between elements and on empty slides (paragraph, list, image, text box, equation — FR-019; image asks for a path, text box uses `box add`); a new block starts a typed session; drag handle on the selected element with drop indicator (vertical) → `block move` — T027 passes — done: + and ⠿ handles beside the hovered element (insert menu; pointer drag with drop line); "+ Add content" on empty slides; inserted text/list opens for typing with its placeholder selected, an equation opens as source

---

## Phase 8: User Story 6 — Context menus (P2)

**Goal**: right-click everywhere gives the full action set.

**Independent Test**: spec US6 Independent Test.

- [X] T029 [P] [US6] Write failing browser tests: right-click a paragraph, a slide in the list and an empty slide area → menus list exactly the actions of contracts/ui.md for that target, inapplicable ones disabled (Move up on first element); arrows/Enter/Escape work; Shift+F10 opens the menu for the selection; menu stays inside the window near edges; each action's effect equals its `qd` command (compare file after menu action vs. after the CLI on a copy) — done (`context_menus_list_and_run_actions`)
- [X] T030 [US6] Implement `src/ui/menu.js` (research R6): one menu component (ARIA `menu`/`menuitem`, submenus for Insert/Appearance/Transition, shortcuts right-aligned, disabled state), opened from `contextmenu` in the iframe document, the slide list and the stage; items from `actionsFor(target)` — T029 passes — done (`src/ui/menu.js`); right-click in the slide, the slide list or an empty slide area; Shift+F10 / ContextMenu key; menus take keyboard focus from the preview
- [X] T031 [US6] Clipboard (FR-022, research R7): Copy/Cut put the element's markdown as `text/plain` (+ rendered `text/html`); Paste after inserts with `block add`; cut = copy + `block rm`; browser test round-trips an element through copy/paste to another slide byte-for-byte — done (`copy_and_paste_carry_the_markdown`); keyboard ⌘C/⌘X/⌘V/⌘D, ⌫, ⌥↑/⌥↓, Enter, ⌥Enter on the selected element

---

## Phase 9: Polish & cross-cutting

- [X] T032 [P] Fragments and notes (research R8): while editing, all `.fragment`s on the current slide are made visible and restored after; "Speaker notes…" in slide toolbar/menu opens the `::: notes` block in the source overlay (creating it with `block add --end` if missing); browser tests — done: the editor preview shows every fragment (with a faint left marker) so hidden content can be clicked and edited; the Preview ↗ deck is unchanged. "Speaker notes…" (slide menu) edits the `::: notes` block in the source box over the slide, creating it if missing (`fragments_notes_and_keyboard_selection`)
- [X] T033 [P] Accessibility (FR-017): keyboard path through elements (Tab/arrows to move selection, Enter to edit, Escape to leave), accessible names on toolbar buttons and menu items, focus returns to the element after menus close; checklist `checklists/a11y.md` filled by hand in the browser pane — done: Tab / Shift+Tab in the slide move the selection with an announcement; labels added for the slide-bar Transition/Hidden and the PDF checkbox; Escape no longer opens Reveal’s overview; `checklists/a11y.md` (screen-reader pass not done)
- [X] T034 [P] Skill and docs: `skills/quartz-deck/SKILL.md` gains `qd block …`, `--expect-text` and "the person may be editing on the slide; prefer element edits with `--expect-text`"; README editor section rewritten for on-slide editing; skill sync test passes — done: README "Editing on the slide" section; skill: `qd block …` incl. `class`, `--expect-text`, speaker notes
- [X] T035 Coverage and byte-identity over the corpus (SC-002/003): test edits one word in every typeable block and opens source for every other block, asserting the file is byte-identical outside the element and that undo restores it; record the typeable share in `checklists/coverage.md` — done: see `checklists/coverage.md` (22/33 typeable; splice checked at every position)
- [X] T036 Run quickstart §1–§6 with the release binary in the browser pane; record results in `checklists/quickstart-run.md`; usability check SC-006 (a 6-step task with the Source panel closed, timed) — done: `checklists/quickstart-run.md` — six-step task ✓ in ~2 min with the Source panel closed; fixed Escape→overview and missing labels found during the run
- [X] T037 Agent eval (as in 002): 10 fresh agents asked to change a paragraph while a scripted "person" edits another element through the browser test harness; grade no lost edits and no usage errors from logs; record in `checklists/agent-eval.md` — done: `checklists/agent-eval.md` — 10/10, 5 calls each, 0 usage errors, all with `--expect-text`, 600 concurrent person edits, none lost
- [X] T038 Final constitution re-check recorded in plan.md; `cargo fmt`, `clippy -D warnings`, full test suite green; update specs/ROADMAP.md status — done: final constitution check in plan.md; ROADMAP status updated
- [X] T039 Concision audit with `/pr-audit` (constitution 1.1.0): sub-agents review the 003 diff — Rust, editor JS/CSS, tests/fixtures, CI and dependencies — with the ponytail review rules; apply verified findings, record skipped ones in the PR description — before the 003 PR is marked ready (also before any earlier PR cut from this branch)

---

## Dependencies & Execution Order

- Setup (T001–T003) → Foundational (T004–T008) → stories.
- US1 (T009–T015) is the MVP and needs T005–T007.
- US2 (T016–T018) needs T007; independent of US1's serializer.
- US5 (T019–T021) needs US1 and US2 sessions.
- US3 (T022–T024) needs T006 (actions); independent of US1/US2.
- US4 (T025–T028): T025–T026 (ops) can run any time after Phase 2; T027–T028 need T012 (typed session for new blocks).
- US6 (T029–T031) needs T006 and the ops it lists (US4's `block` ops for structure items; items for missing ops show disabled until then).
- Polish after the stories it touches.

## Parallel opportunities

- T002 ∥ T003; T004 ∥ T006 (different files).
- After Phase 2: US2's T016–T017 (Rust) ∥ US1's T009–T010 (JS serializer); US4's T025–T026 (Rust ops) ∥ any UI story.
- Within a story, `[P]` test tasks can be written together.

## Implementation strategy

1. MVP = Phases 1–3 (type on the slide with the toolbar) → stop and demo.
2. Add US2 (source overlay) → every element editable.
3. Add US5 → safe with an agent; P1 done, PR.
4. US3, US4, US6 in any order (P2), then Polish.
