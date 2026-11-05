# Implementation Plan: Shared Focus

**Branch**: `005-shared-focus` | **Date**: 2026-11-05 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/005-shared-focus/spec.md`

## Summary

The editor gets one shared **focus** object (current slide, selected
elements, selected text, source caret line, whether the person is editing),
owned by a new `src/ui/focus.js`. It reports a snapshot to the running server
whenever it changes; the server keeps the latest snapshot per connected tab in
memory. `qd focus` reads it (`{"open": false}` with no editor), and
`qd focus --set <ref>` sends a `point` event to the most recent tab, which goes
there and flashes the element — or, while the person is editing, offers
"Go to …" instead (`deferred`). All endpoints use 004's token and loopback
rules; nothing is stored. 004's source↔slide sync is rewired through the same
focus, which spec 006 (VS Code) will also use. Details: [research.md](research.md).

## Technical Context

**Language/Version**: Rust (stable, edition 2024) for the CLI and server; plain
ES modules for the editor (no build step).

**Primary Dependencies**: none new. axum (server), ureq (CLI → server), as
already used.

**Storage**: none — focus is in server memory only (FR-010).

**Testing**: `cargo test`. Unit tests for snapshot validation/clamping and ref
dropping; HTTP tests in `tests/serve.rs` (token rules, open/closed, latest tab,
stale refs, set outcomes); browser tests in `tests/onslide.rs` (select on slide
and in source → `qd focus`; `--set` moves and flashes; `--set` while typing is
deferred with no lost keystrokes, 20×; the cross-origin test extended to the
focus endpoints).

**Target Platform**: macOS first; Linux CI.

**Project Type**: CLI + local web editor (single Rust crate).

**Performance Goals**: focus visible to `qd focus` ≤ 0.5 s after a change
(SC-001); `--set` shows the slide ≤ 1 s (SC-002).

**Constraints**: local only; token-guarded; no disk; selected text ≤ 2,000
chars; reading never changes anything.

**Scale/Scope**: one deck per server, a handful of tabs.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design.*

| Principle | How the plan complies | Status |
|---|---|---|
| I. Clean-room | Our own code on our own editor; no third-party editor code. | PASS |
| II. `.qmd` is truth | Focus never touches the deck; `--set` only moves the view. | PASS |
| III. CLI-first | `qd focus` / `--set` with JSON, in `qd schema` and the skill; the editor's own sync uses the same focus. | PASS |
| IV. Live preview | Pointing moves the person's live editor; source↔slide sync unchanged in behaviour. | PASS |
| V. Safe concurrent edits | `--set` never interrupts an open edit (deferred + offer); refs are version-checked. | PASS |
| VI. Test-first | Failing tests per story first (HTTP, browser, CLI), as in 004. | PASS |
| VII. Simplicity | No new dependency; snapshots over the existing SSE + token; no ack protocol (R3). | PASS |
| Constraints 1.2.0 | Token-guarded endpoints only the editor link carries; local only. | PASS |

Post-design re-check: PASS — three small endpoints, one event, one module.

Final re-check after implementation (T023, constitution 1.2.0): PASS.
- I: our own code only.
- II: focus never touches the deck; 100 reads leave it byte-identical
  (`focus_over_http`); pointing only moves the view.
- III: `qd focus [--set]` in `qd schema` and the skill; the editor's own
  source↔slide sync goes through the same focus (FR-012).
- IV: pointing shows the slide in 15–21 ms; the focus reaches `qd focus`
  in ~120 ms (quickstart run).
- V: pointing never interrupts typing (20× deferred, nothing lost); refs are
  version-checked and stale ones dropped.
- VI: every story landed with failing tests first (unit, HTTP, browser).
- VII: no new dependency; both PRs passed the concision audit, which also
  caught 8 real bugs before merge.
- Constraints: all focus endpoints behind the session token and loopback
  rules; memory only (`checklists/security.md`).

## Project Structure

### Documentation (this feature)

```text
specs/005-shared-focus/
├── plan.md
├── research.md
├── data-model.md
├── quickstart.md
├── contracts/
│   ├── cli.md
│   └── http.md
└── checklists/requirements.md
```

### Source Code (repository root)

```text
src/
├── focus.rs          # NEW: snapshot type, validation/clamping, ref dropping (unit-tested)
├── server.rs         # /api/focus (GET/POST), /api/focus/set, tab tracking on /events, `point` event
├── cli.rs            # `qd focus [--set REF]`; to_server gains GET
└── ui/
    ├── focus.js      # NEW: the shared focus object; reports, listeners, `point` handling, "Go to …" offer
    ├── editor.js     # slide selection/navigation → focus.set; flash on point
    ├── pane.js       # caret/selection → focus.set; editing flag
    └── editor.css    # flash + offer styles
skills/quartz-deck/SKILL.md   # "this" → qd focus first; --set to show
tests/
├── serve.rs          # HTTP focus tests
├── onslide.rs        # browser focus tests (+ cross-origin test covers focus)
└── focus_cli.rs?     # only if serve.rs can't host the CLI cases
```

**Structure Decision**: single crate as in 001–004; one new Rust module for
the testable focus logic and one UI module for the page side.

## Complexity Tracking

None: no constitution violations to justify.
