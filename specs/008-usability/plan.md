# Implementation Plan: Usability — the editor made easy to use

**Branch**: `008-usability` | **Date**: 2026-11-06 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/008-usability/spec.md`

## Summary

The audit ([audit/README.md](audit/README.md)) found 27 problems; the spec
turns them into nine stories. Almost all of the work is in the editor's own
modules and CSS:
- **the slide list:** thumbnails copied from the canvas's own render, a + in
  every gap, untitled new slides with an editor-only placeholder, and a
  pointer-driven drag with a drag image and an opening gap;
- **the canvas:** one placement rule so bars never cover content, and
  reveal.js's own chrome hidden;
- **themes:** contrast and size fixed in CSS, held by an automated
  measurement test;
- **Present:** full screen in place;
- **sheets:** one small module replacing every browser `prompt`, `confirm`
  and `alert`;
- **words and bars:** plain labels in the action list, one top bar, tidier
  bars, a fuller Insert, a narrow layout.

The server gains one thing: listing and adding images beside the deck (`GET`
and `POST /api/images`, `qd images`, `qd image add`). The CLI also gains
several slides for `qd move` and `qd rm` (P3).

Details: [research.md](research.md).

## Technical Context

**Language/Version**: Rust (stable, 2024) for qd; ES modules for the editor
(no build step).

**Primary Dependencies**: none new. The image upload is the raw file body
(no base64), see research R9.

**Storage**: the `.qmd`; dropped images in `images/` beside the deck. No other
state.

**Testing**:
- **Rust unit tests:** image name sanitising and free-name choice, extension
  and magic-byte checks, size limit; several slides for `move`/`rm` as one
  patch; the static scan that no `prompt(`/`confirm(`/`alert(` remains in
  `src/ui/*.js`.
- **`tests/cli.rs`:** `qd images`, `qd image add`, `qd move a b --after c`,
  `qd rm a b`.
- **`tests/serve.rs`:** `GET /api/images`; `POST /api/images` (403 without the
  token, 400 for a bad type or a too-large file, a free name when taken).
- **Browser (`tests/onslide.rs`):**
  - the list: thumbnails present and refreshed after an edit, the title slide
    first, add from a gap and from Enter, the placeholder title, drag with
    the gap and Escape, keyboard move;
  - the canvas: no bar overlap, no reveal chrome, no jump;
  - Present from the current slide and from the beginning (full screen
    requested);
  - sheets in place of every native box (stubbed natives fail the test if
    called);
  - an image dropped on the slide;
  - the top bar, the tab title and the status words;
  - the narrow layout at 420 px;
  - multi-select.
- **Measurement:** `audit_measures` walks every visible control and text in
  light, dark and high contrast on the audit deck: contrast, target sizes,
  bar overlap (SC-003–005).
- **The audit rerun:** `shoot.py` again after the work, with the findings'
  status recorded beside the audit (SC-010).

**Target Platform**: desktop browsers (Chrome, Safari, Firefox current) and VS
Code ≥ 1.90's webview; macOS first, Linux CI.

**Project Type**: CLI + local web editor (+ VS Code extension).

**Performance Goals**:
- thumbnails for 100 slides within 300 ms of a render, without blocking
  typing;
- a drag follows the pointer at 60 fps;
- a sheet opens in under 100 ms.

**Constraints**:
- clean-room, our own design (Apple's guidelines as principles only);
- no build step; no new UI framework;
- every action in the action list (007); every deck edit a `qd` command;
- the deck's own slide styles untouched.

**Scale/Scope**: about 30 findings; mostly `src/ui/` (editor.js, a new
`list.js` for the slide list, `sheet.js`, toolbar.js, run.js, menubar.js,
editor.css, index.html), with small server and CLI additions.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design.*

| Principle | How the plan complies | Status |
|---|---|---|
| I. Clean-room | Our own design; Apple's Human Interface Guidelines used for principles and published numbers only. | PASS |
| II. `.qmd` is truth | Placeholders, thumbnails and status are editor-only; new slides are untitled headings; images go beside the deck only on the person's drop. | PASS |
| III. CLI-first | `qd images` and `qd image add` before the editor's picker and drop; `qd move`/`qd rm` with several slides before multi-select. | PASS |
| IV. Live preview | Thumbnails come from the same render as the canvas; Present uses it full screen. | PASS |
| V. Safe concurrent edits | Every edit stays an op through the store's version check; one drop or multi-move is one patch. | PASS |
| VI. Test-first | Each story's browser tests and the measurement test are written first and seen failing. | PASS |
| VII. Simplicity | No new dependency; thumbnails are DOM copies, not new renders; one placement rule; one sheet module. | PASS |
| Constraints 1.2.0 | `POST /api/images` is token-guarded, JSON-only, type- and size-checked, writes only under `images/`, never overwrites. Code still runs only on the person's own action. | PASS |

Post-design re-check: PASS. The one new write path (images) is recorded in
research R9 with its security note.

## Project Structure

### Documentation (this feature)

```text
specs/008-usability/
├── audit/ (README.md, shoot.py, deck/, shots/)
├── spec.md · plan.md · research.md · data-model.md · quickstart.md
├── contracts/ (http.md, cli.md, ui.md)
└── checklists/requirements.md
```

### Source Code (repository root)

```text
src/
├── images.rs        # list images beside the deck; add one (name, type, size checks; free name)
├── ops.rs           # move/rm: several slides, one patch
├── cli.rs           # qd images, qd image add; qd move/rm with several slides
├── server.rs        # GET/POST /api/images
└── ui/
    ├── list.js      # the slide list: thumbnails, + in gaps, pointer drag, multi-select, narrow picker
    ├── sheet.js     # the editor's own sheets (fields, validation, None, file list)
    ├── editor.js    # bar placement rule, reveal chrome off, Present, status words, image drop
    ├── toolbar.js   # common formats + "Aa" pop-over; colour menu
    ├── run.js       # source bar: Run all, Stop, state in words, ··· menu
    ├── menubar.js   # one top bar (menus or ☰ + title + status + Present + Source)
    ├── actions.json # plain labels; present, present-start; Insert's new blocks
    ├── index.html   # one top bar
    └── editor.css   # theme tokens for form controls and source colours; control height; narrow layout
tests/{cli,serve,onslide,actions}.rs
skills/quartz-deck/SKILL.md  # qd images, qd image add, several slides for move/rm
```

**Structure Decision**: the slide list leaves `editor.js` for its own module
(it is the largest story and `editor.js` is ~1,700 lines); sheets get one
module. Everything else changes in place.

## Complexity Tracking

| Addition | Why needed | Simpler alternative rejected because |
|---|---|---|
| `POST /api/images` (a write outside the `.qmd`) | Dropping an image file must put it beside the deck (US5, SC-008) | Asking the person to copy files into the folder by hand is the friction the audit found |
| Pointer-driven drag instead of HTML5 drag and drop | A drag image, an opening gap and auto-scroll (L2) | HTML5 drag and drop gives no control over the moving image or the gap, and differs in Safari |
| Thumbnails as DOM copies with shared stylesheets | Thumbnails for every slide (L3) at the cost of DOM, not of a reveal.js per slide | One iframe per slide is ~10–20 MB and a script start each |
