# Quickstart run (T036, 2026-11-04)

Release binary, browser pane (Claude desktop app, ~1330×1180 CSS px), a fresh
7-slide deck with Python cells under git, so `git diff` shows exactly what
was written. uv environment with matplotlib. Timings are measured in the page
(`performance.now()`), from the action to the observable result.

## §1 Environment (FR-017–020, SC-007)

- `qd env init && qd env add matplotlib`: `pyproject.toml` + `uv.lock`, tool
  uv, `lock_current: true` (0.6 s with a warm uv cache).
- Hand-edited manifest (a dependency added without relocking) → `qd run --all`
  refuses with `E_LOCK_STALE`; restoring the file → `lock_current: true`.
- SC-007 (second machine): `tests/env.rs` `uv_environment_lifecycle` syncs a
  fresh copy of deck + `pyproject.toml` + `uv.lock` with `qd env sync` and gets
  a byte-identical `uv.lock` (all locked versions match); pixi variant in CI.

## §2 Source pane (US1, US2)

| Check | Result | Target |
|---|---|---|
| SC-001: YAML title + 3 slides edited in the source pane | each on disk in 0.71–0.78 s | < 2 s ✓ |
| `git diff` | exactly the 4 typed changes | ✓ |
| 4 × `qd undo` | byte-identical to the commit | ✓ |
| SC-002: caret moved between slides (24 random moves) | median 8 ms, p95 10 ms, max 16 ms, 0 over 500 ms | p95 ≤ 0.5 s ✓ |
| SC-002: click a slide paragraph → its source in view (3 slides) | 6–30 ms | ≤ 0.5 s ✓ |

## §3 Run (US3, US5)

| Check | Result | Target |
|---|---|---|
| First run (starts Python in the deck's env) | 3.2 s to output | — |
| SC-003: print cell, warm | ~20 ms to output (3 runs) | ≤ 3 s ✓ |
| SC-003: figure cell, warm, image on its slide | 1.5–2.2 s (re-render included) | ≤ 5 s ✓ |
| Figure cell, first `import matplotlib` | 5.2 s to the slide (3.7 s of it the import itself) | (cell compute > 1 s) |
| `1 / 0` | traceback in the dock in 0.1 s, ▶ turns red | ✓ |
| `while True: pass` → **Stop** | `KeyboardInterrupt` 20 ms after Stop, kernel idle | ✓ |
| SC-004: `.qmd` after all runs | `git diff` empty | ✓ |
| Restart (`qd stop`, `qd serve`, open the new link) | outputs back in the pane (counters, red ▶, stale), figure on its slide, no kernel started; link token gone from the address bar | ✓ |
| Cell edited since its run | output dimmed as stale (pane, dock, slide) | ✓ |

## §4 Agents (US4)

With the editor open: `qd run results/0` → `mode: live`, counter continues
the editor's session (13), `42`; `qd run --all` → stops at `broken/0`, exit 1,
`E_RUN` on stderr, results on stdout; `qd outputs` lists all four with
`stale`. The source pane picked up every agent run (counters 14–16, red ▶ on
the failing cell) without a reload.

## §5 Safety (SC-006)

See `checklists/security.md` (automated cross-origin attack test: nothing
ran, nothing changed).

## Found and fixed during the run

- At this pane width the run bar (Run all … + Package, Python state) did not
  wrap: "+ Package" was clipped and the Python state label spilled over the
  slide bar. The run bar now wraps inside the source bar.
- `qd stop` takes ~2.3 s with an editor tab open (the server waits up to 2 s
  for the tab's event stream before exiting) — expected, recorded.
