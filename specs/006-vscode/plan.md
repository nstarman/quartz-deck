# Implementation Plan: VS Code Integration

**Branch**: `006-vscode` | **Date**: 2026-11-05 | **Spec**: [spec.md](spec.md)

**Input**: Feature specification from `specs/006-vscode/spec.md`

## Summary

A thin, dependency-free VS Code extension (`vscode/`, plain JS) opens the
existing quartz-deck editor in a webview panel beside the `.qmd`. The editor
page is iframed from the `qd serve` it starts, via VS Code's own port
forwarding, so it also works over Remote-SSH.

While the panel is open, a **buffer overlay** makes the VS Code buffer the
truth:
- `Store` reads and writes `<cache>/buffer.qmd` instead of the file. So panel
  edits, agents' `qd` edits and undo all land in the buffer, as unsaved edits
  VS Code's undo reverts.
- The person's typing flows back with a compare-and-swap and a three-way
  merge, so neither side loses work.
- The `.qmd` changes only when VS Code saves (the clarification).

Other parts:
- **Cursor ↔ slide:** reuses spec 005's focus, plus two `postMessage` types.
- **Run:** defers to the Quarto extension when it is installed; otherwise
  spec 004's Run.
- **Palette:** render, export and the offline check run `qd`.

Details: [research.md](research.md).

## Technical Context

**Language/Version**: Rust (stable, 2024) for qd; plain JavaScript (CommonJS)
for the extension; ES modules for the editor (no build step).

**Primary Dependencies**: no new Rust crates (`similar` already used for
diffs). Extension: the `vscode` API only; `@vscode/vsce` to package (dev
tool, run with `npx`).

**Storage**: `<cache>/buffer.qmd` overlay while attached (user-only cache
dir); nothing new in the deck folder.

**Testing**:
- **Rust:** `Store` overlay unit tests; `tests/serve.rs` for open/close,
  CAS/409, `buffer_edit` after panel and CLI edits, the file untouched until
  save, and undo.
- **Extension logic:** `node --test` (merge, applying edits, message
  routing) with a fake `vscode` object.
- **End to end:** the VS Code quickstart run, recorded in a checklist.
- **CI** gains a Node step.

**Target Platform**: VS Code (desktop) on macOS first; Linux CI; Remote-SSH.

**Project Type**: CLI + local web editor + VS Code extension.

**Performance Goals**:
- panel opens ≤ 5 s;
- buffer → panel ≤ 3 s;
- cursor ↔ slide ≤ 0.5 s.

**Constraints**:
- no lost changes on either side;
- the file changes only on save;
- 004/005 safety unchanged;
- no marketplace.

**Scale/Scope**: one panel per deck; a few hundred lines of extension code.

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-checked after Phase 1 design.*

| Principle | How the plan complies | Status |
|---|---|---|
| I. Clean-room | Our own extension against VS Code's public API docs; no Quarto-extension code. | PASS |
| II. `.qmd` is truth | While VS Code is attached the buffer is the person's working copy of the `.qmd`; qd still edits minimal spans and the file is written by VS Code's save. | PASS (by clarification) |
| III. CLI-first | No new agent interface; every `qd` command works on the buffer transparently; palette commands call `qd`. | PASS |
| IV. Live preview | The same editor, in a panel; buffer changes re-render it without a save. | PASS |
| V. Safe concurrent edits | CAS on the overlay, three-way merge, a choice on overlap; VS Code's undo covers panel edits. | PASS |
| VI. Test-first | Rust and `node --test` tests before code; quickstart run for VS Code itself. | PASS |
| VII. Simplicity | No runtime deps, no bundler, one editor reused; the overlay is one switch in `Store`. | PASS |
| Constraints 1.2.0 | Token-guarded endpoints only; local only; messages carry navigation, never code. | PASS |

Post-design re-check: PASS. A Node step in CI is the only new tooling; the
extension adds a new language (JS outside the browser), justified below.

Final re-check after implementation (T023, 2026-11-05): PASS.
- II: `qd render` renders the saved file. `qd export` and `qd check` read the
  attached buffer. The palette asks before a render would miss unsaved text.
- V: CI found a race where the watcher read the buffer outside its lock, so a
  stale read could undo the person's typing. It is fixed in #18.
- VII: cursor following reuses the editor's source-pane path (a `follow`
  message), so there is no second focus module and no server change.
- Not yet run in the real app: quickstart §3–§5 and Remote-SSH (T022). They
  are left for a follow-up session by the person's choice.

## Project Structure

### Documentation (this feature)

```text
specs/006-vscode/
├── plan.md · research.md · data-model.md · quickstart.md
├── contracts/ (http.md, cli.md, extension.md)
└── checklists/requirements.md
```

### Source Code (repository root)

```text
src/
├── store.rs        # overlay: read/write <cache>/buffer.qmd when attached (+ unit tests)
├── server.rs       # /api/buffer/{open,close}, /api/buffer (CAS), buffer_edit events, overlay watch
└── ui/
    ├── editor.js   # framed mode: ?host=vscode, theme, run=external; follow/reveal messages
    └── editor.css  # theme override from ?theme= (light/dark/hc)
vscode/
├── package.json    # manifest: commands, settings, extensionKind, activation
├── extension.js    # activation, commands, panel lifecycle, qd process, palette commands
├── sync.js         # buffer ⇄ overlay: CAS, buffer_edit application, three-way merge (pure)
├── panel.js        # webview HTML (iframe), messages, theme, follow/reveal
├── test/*.test.js  # node --test with a fake vscode
└── README.md
tests/serve.rs      # overlay HTTP tests
.github/workflows/ci.yml   # + Node, node --test vscode/test
```

**Structure Decision**: the extension lives in `vscode/` in the same repo
(shared release, shared tests); the Rust crate is unchanged in shape.

## Complexity Tracking

| Addition | Why needed | Simpler alternative rejected because |
|---|---|---|
| JavaScript extension code outside the browser | VS Code extensions run on its extension host (Node) | no way to integrate with VS Code otherwise |
| Node in CI | to run the extension's unit tests | untested sync logic is where data loss would hide |
