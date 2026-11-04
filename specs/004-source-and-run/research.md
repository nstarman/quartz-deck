# Research: Source and Run

Decisions for spec 004. Each: Decision / Rationale / Alternatives. Items
marked **verified** were run on this machine on 2026-11-04 (uv 0.13.7,
Quarto 1.11.9, Python 3.12 via uv).

## R1. Per-deck environments: uv by default, pixi as the alternative

**Decision**: The environment root is the nearest folder, from the deck up to
its Quarto project root, containing `pixi.toml` / `pyproject.toml` with
`[tool.pixi]` (→ **pixi**) or `pyproject.toml` / `uv.lock` (→ **uv**). If none
exists, `qd env init` creates one with **uv** in the Quarto project root (or
the deck's folder). Commands run *inside* the environment through the tool:
`uv run --project <root> --locked <cmd>` or
`pixi run --manifest-path <root> --locked <cmd>`. qd never calls `pip`.

pixi decks keep Python packages as **PyPI dependencies**
(`pixi add --pypi …`); pixi itself still provides the Python interpreter from
conda-forge (that is how pixi works), but no conda *packages* are added by qd.

**Rationale**: The user's direction (spec Clarifications): one lockfile per
presentation, reproducible on another machine. `--locked` makes every run and
render refuse when the lock is stale instead of silently re-resolving
(FR-019).

**Alternatives**: plain venv + `pip freeze` (no real lock, platform drift);
conda environments (heavy, not wanted); a qd-private environment in its cache
(not reproducible by others, invisible in the repo).

## R2. Base packages (verified)

**Decision**: `qd env init` creates the environment with `ipykernel`,
`nbclient`, `nbformat`, `pyyaml` (what Quarto's Jupyter engine and the kernel
bridge need), then `uv lock`. The deck's own packages (numpy, matplotlib,
plotly…) are added with `qd env add`.

**Verified**: `uv init --bare` + `uv add ipykernel nbclient nbformat
matplotlib` → `pyproject.toml` + 1,282-line `uv.lock` in 15 s; Quarto render
failed with `No module named 'yaml'` until `pyyaml` was added — then succeeded.

## R3. Rendering through the environment (verified)

**Decision**: Every Quarto invocation that may execute code (`qd render`,
`qd export`, `qd check --render`) is wrapped: `uv run --locked quarto render …`
(or `pixi run --locked quarto render …`); Quarto finds the environment's
Python on PATH. Decks without code cells keep running Quarto directly.

**Verified**: `uv run quarto render deck.qmd` executed both cells (the printed
`42` and a matplotlib figure appear in the HTML), 9 s.

## R4. Live kernel through a small Python bridge (verified)

**Decision**: `qd` starts one **kernel bridge** per deck: a short Python script
embedded in the binary, run as `uv run --locked python -I <bridge.py>` inside
the deck's environment. The bridge uses `jupyter_client` (installed with
`ipykernel`) to start a `python3` kernel over **IPC sockets**
(`transport="ipc"`, no TCP), and talks to qd with JSON lines on stdin/stdout:
`execute {id, code}` → `{id, outputs:[…], status}`, `interrupt`, `restart`,
`shutdown`. Outputs are normalised: `stream` → text, `execute_result` /
`display_data` → richest of `image/png` (saved as a file), `text/html`
(tables, Plotly), `text/plain`; `error` → name, value, traceback.

**Rationale**: No ZeroMQ/Jupyter-protocol crate in Rust; the bridge runs with
exactly the deck's locked packages; JSON lines are trivial to test.

**Verified**: cold start ~8 s (environment + kernel); printed text, a 22 KB
PNG, state carried between executions (`x + 1` → 43), `ZeroDivisionError`
reported; with TCP the kernel warned about unencrypted transport — with
`transport="ipc"` the warning is gone.

**Alternatives**: Rust ZeroMQ client (new native dependency, protocol code);
`quarto render` per Run (seconds per run, no state — kept as the "re-run from
top" mode, R5).

## R5. Sessions and the two run modes

**Decision**: `qd serve` owns the deck's live session (one bridge, started on
first Run, stopped with the server or by Restart). **Live mode** (default)
executes the requested cell in that kernel; each output records its run
counter so out-of-order runs are visible (FR-008a). **Re-run-from-top mode**
(per-deck setting kept in qd's cache — a UI preference never changes the deck)
restarts the kernel and executes cells 1…k. `qd run` from the CLI talks to the
running server (same session, outputs show in the editor, FR-014); with no
server it starts a one-shot bridge (fresh session, run from the top) and
writes the outputs to the store.

## R6. Output store and staleness

**Decision**: Outputs live in qd's cache, never in the `.qmd` (FR-009):
`<cache>/outputs/<deck-id>/<cell-fingerprint>/{outputs.json, 1.png, …}`.
Cell fingerprint = SHA-256 of the cell's source text (options included); the
store also maps cell ref → last fingerprint, so an output whose fingerprint no
longer matches the cell is **stale** (FR-011). Large outputs are truncated in
the JSON (first 2,000 lines, full text in a side file).

## R7. Outputs in the slides without executing in the preview

**Decision**: The editor's preview copy (spec 001) stops executing code: it
renders with `execute: {eval: false, echo: true}` and inserts, after each
code cell, that cell's stored outputs as raw HTML (`::: {.cell-output-display}`
with an `{=html}` block; images as data URIs or cached file paths). The
preview stays a pure Pandoc render (fast, no kernel, no environment needed),
and shows exactly what the last Run produced, stale outputs marked. Final
`qd render` / `qd export` still execute through the environment (R3), so the
deliverable is always the real thing.

**Rationale**: Re-executing every cell on every keystroke-render is slow and
makes the preview depend on the kernel; separating "edit preview" from
"final render" matches how notebooks work.

## R8. Source pane editor without a framework

**Decision**: A `<textarea>` with a highlighted `<pre>` underneath (same font
and metrics, textarea text transparent, caret visible) — the classic overlay
editor. A small highlighter (~120 lines JS) colours front matter (YAML),
headings, fenced divs, code cells and their `#|` options, emphasis and links.
Highlighting runs per frame on the visible lines only, so hundreds of slides
stay responsive. Each code cell's Run button and outputs are drawn in a gutter
layer aligned with the cell's lines.

**Alternatives**: CodeMirror 6 (excellent, but needs a bundled build — the
constitution keeps the editor build-free unless justified; can be revisited if
the overlay proves insufficient); `contenteditable` (unreliable for code).

## R9. Saving source edits: `qd replace`

**Decision**: A new op `qd replace --old <text> --new <text>`: replace the
**unique** occurrence of `old` in the deck with `new`; refuse with `E_CONFLICT`
if `old` occurs zero times (changed elsewhere) or `E_AMBIGUOUS` if more than
once. The source pane sends the changed region expanded to whole lines plus
one line of context on each side, so `old` is almost always unique; on
ambiguity it widens the context and retries. This works without knowing the
file version (other edits elsewhere don't block it — same spirit as
`--expect-text`, spec 003 R3) and is useful to agents too (a literal
search-and-replace). Structure changes are allowed here (the source pane may
add slides); undo works as for every op.

**Alternatives**: byte offsets + file version (any edit elsewhere conflicts);
sending the whole file (loses minimality, races with agents).

## R10. Source ↔ slide sync

**Decision**: `/api/deck` and `qd show` report each slide's and block's line
range (`lines: [start, end]`, 1-based). The source pane maps cursor line →
slide (show it in the preview) and selection / on-slide edit → block line
range (scroll and highlight).

## R11. Safety: code runs only on explicit action

**Decision**: Run endpoints (`POST /api/run`, `/api/run/interrupt`,
`/api/run/restart`) require a per-session secret token: generated by
`qd serve`, written into the session file (user-only permissions) for the
CLI, and embedded in the editor page. Other pages cannot read the page
(same-origin policy) so cannot learn the token; the token travels in a custom
header, which also forces a CORS preflight the server refuses; the existing
Host/Origin loopback checks stay. `run_op` (the generic op endpoint) never
accepts `run`. The preview never executes code (R7).

## R12. Testing

**Decision**: Unit tests for environment discovery (fixtures with
`pyproject.toml`/`uv.lock`/`pixi.toml`), `qd replace`, the output store and
staleness, line ranges. Integration tests gated on `uv` + network (first
lock): `qd env init`/`add`, `qd run` (text, figure file, error, state,
interrupt), render through the environment. Browser tests (existing harness):
source pane typing → `qd replace` bytes, sync, Run button → output under the
cell and on the slide, token rejection from another origin. pixi tests run
only where `pixi` is installed; CI installs both uv and pixi.
