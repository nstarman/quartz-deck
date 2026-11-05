# Research: VS Code Integration

No NEEDS CLARIFICATION remains (the spec's one question was answered: panel
edits stay unsaved in the VS Code buffer until the person saves).

## R1 — What the extension is made of

- **Decision**: plain JavaScript (CommonJS) using only the `vscode` API, in
  `vscode/` at the repo root; no runtime npm dependencies. Packaged as a
  `.vsix` with `npx @vscode/vsce package` (a dev-time tool, not a dependency).
- **Rationale**: matches the editor UI (plain JS, no build step, Principle
  VII); the extension is thin glue around the existing editor and `qd`.
- **Alternatives**: TypeScript + bundler — a build pipeline for a few hundred
  lines; a second editor written for VS Code — two editors to maintain.

## R2 — The panel

- **Decision**: a VS Code webview panel whose page is a full-size iframe of
  the editor link `qd serve` prints (`…/#t=<token>`), reached through
  `vscode.env.asExternalUri` so it also works in remote workspaces. Query
  parameters tell the editor it is framed: `?host=vscode&theme=dark|light|hc`
  (`&run=external` when the Quarto extension is installed). Theme changes are
  sent with `postMessage`.
- **Rationale**: one editor for browser and VS Code (spec assumption);
  `asExternalUri` is VS Code's own port forwarding (FR-016).
- **Alternatives**: rendering the editor's HTML inside the webview directly —
  the editor expects to be served by `qd serve` (SSE, preview, assets) and
  would need rewriting for webview resource URIs.

## R3 — "The buffer is the truth while the panel is open" (FR-008/009/010)

- **Decision**: a **buffer overlay**. While a VS Code panel is open on a deck,
  the server keeps the buffer's text in `<cache>/buffer.qmd`, and the session
  file says so. `Store` reads and writes the overlay instead of the `.qmd`
  whenever it exists. So every qd path goes to the buffer unchanged: panel
  ops, CLI edits by agents, `qd undo`, the source pane, `qd focus` refs and
  rendering. The `.qmd` on disk changes only when VS Code saves (FR-010).
- **Rationale**: `Store` is already the single choke point for reads, writes,
  locking, conflict checks and the undo journal. Switching its backing text
  keeps every guarantee without touching each command, and needs no routing
  of CLI ops through the server.
- **Alternatives**: routing every CLI command through the server when VS Code
  is attached — every command gains a second code path. Saving the buffer
  before each panel edit — rejected by the clarification.

## R4 — qd's changes → the VS Code buffer

- **Decision**: the server watches the overlay. When it changes for any reason
  other than the extension's own update (a panel op, an agent's `qd edit`,
  `qd undo`), it computes a minimal diff of old → new overlay text (`similar`,
  already a dependency). It then sends a `buffer_edit {base, version, edits}`
  event. The extension applies it as one `WorkspaceEdit`, which marks the
  document dirty and can be undone with VS Code's undo (FR-007).
- **Rationale**: VS Code's own undo stack and dirty state come for free; the
  diff keeps cursors and folds stable.

## R5 — The person's typing → the overlay, and races

- **Decision**: the extension posts the buffer text (debounced ~150 ms) with
  `base` = the overlay version it last applied. The server writes it only if
  the overlay is still at `base` (compare-and-swap under the store lock). On a
  mismatch (a qd edit landed meanwhile) it answers 409 with the current
  overlay. The extension then three-way-merges base / buffer / overlay:
  - changes in different places are both kept;
  - changes to the same lines bring up a small choice in VS Code (keep mine /
    take theirs / compare), as spec 003's conflict bar does in the browser.
- **Rationale**: the same three-way rule as the source pane (spec 004), so
  "never lose either side" means the same thing everywhere.

## R6 — Cursor ↔ slide (FR-011–013)

- **Decision**:
  - The VS Code cursor and selection are posted to `/api/focus` as their own
    tab (`tab: "vscode:<deck>"`, `where: "text"`, the selected text, the
    line, refs from the line via `qd`'s block line ranges). So `qd focus`
    sees them (spec 005: the latest tab wins).
  - The panel follows the VS Code cursor via a `follow {line}` message from
    the extension to the iframe. The editor maps the line to a slide, the
    same mapping as its own source pane.
  - Selecting in the panel sends `reveal {lines}` to the parent, and the
    extension reveals that range in the text editor without taking focus.
  - `qd focus --set` already reaches the panel; the extension also hears
    `point` and reveals the source.
- **Rationale**: reuses 005's single focus; only two small message types are
  added.

## R7 — Run, render, export (FR-014/015)

- **Decision**: if the Quarto extension (`quarto.quarto`) is installed, the
  panel opens with `run=external` and the editor hides its Run UI. Otherwise
  spec 004's Run works unchanged. Palette commands run `qd render`,
  `qd export [--pdf]` and `qd check --offline` on the deck and show the JSON
  result (output path or problems) in a notification and an output channel.
- **Rationale**: no competing Run buttons; the CLI already does the work.

## R8 — Lifecycle and remote

- **Decision**:
  - `extensionKind: ["workspace"]`, so the extension runs where the files
    are: locally, or on the SSH host.
  - It spawns `qd serve --port 0` for the deck and reads the first JSON line
    for the link, then registers the overlay with `POST /api/buffer/open`
    (token).
  - On panel close it calls `qd stop`, which removes the overlay. A
    `WebviewPanelSerializer` restores panels after a restart.
- **Rationale**: one server per deck panel, cleaned up with it (FR-004).

## R9 — Safety (FR-017)

- **Decision**:
  - The overlay endpoints use 004's `authorized()` (token, loopback
    host/origin).
  - The editor accepts `postMessage` only from `window.parent`, and only
    `follow`, `theme` and `run` messages: navigation and display, never
    edits or code.
  - The extension accepts only `reveal` (and `ready`) from its own webview.
  - The overlay file sits in qd's user-only cache dir.
- **Rationale**: no new path to run code or read the focus.

## R10 — Testing

- **Decision**:
  - **Rust:**
    - unit tests for the overlay in `Store`;
    - HTTP tests in `tests/serve.rs`: open/close, CAS and 409, `buffer_edit`
      after a panel op and after a CLI `qd edit`, the disk untouched until a
      simulated save, undo on the overlay.
  - **Extension logic** (merge, applying `buffer_edit`, message routing):
    small pure modules tested with `node --test` against a fake `vscode`
    object. No npm dependencies; CI adds a Node step.
  - **End-to-end in VS Code:** a quickstart run, recorded in a checklist,
    driven in the real app.
- **Alternatives**: `@vscode/test-electron` end-to-end tests downloading VS
  Code in CI — heavy and flaky for a personal-use extension; revisit if the
  extension grows.
