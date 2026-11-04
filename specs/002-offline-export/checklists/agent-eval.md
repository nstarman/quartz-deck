# Agent evaluation (T029, SC-005)

Target: an agent given only the Skill exports a deck, reads the check result
and fixes one reported problem without help in ≥ 9 of 10 trials.

**Result (2026-11-04): 10/10 passed, 0 usage errors, 3 qd calls each.**

## Setup

- 10 fresh subagents (Claude Sonnet), in parallel, each with the Skill path,
  its own folder and a logging `bin/qd` wrapper (exit status, error code, args).
- Deck "Field notes": math on two slides; slide `footage` has
  `background-video="media/clips.mp4"` while the folder has `media/clip.mp4`
  (a typo). A plain export fails with `P_MISSING_FILE` on `footage`.
- Prompt (no qd syntax): "produce an offline, self-contained export … if the
  export reports problems, fix them and export again until it succeeds";
  change the deck only through qd.

## Grading (objective)

`eval2_grade.py`: `export/deck.html` + `.qd-export` exist; an independent
`qd check --offline --html export/deck.html` passes; the typo is fixed (path via
qd, or the file copied); any deck change equals a journaled qd write; no exit-2
/ `E_USAGE` / `E_INVALID_ARG` (the expected first `E_OFFLINE` is not a mistake).

| Runs | Pass | Fix chosen | qd calls | Errors |
|---|---|---|---|---|
| 01–10 | 10/10 | `qd set footage --attr background-video=media/clip.mp4` (all) | 3 each: export → set → export | 0 |

## Observations

- All ten read the `P_MISSING_FILE` problem, its `slide` and `url`, compared
  with the folder, and chose the path fix — the hint's second suggestion.
- **Finding:** all ten noticed that `qd set` re-quoted an untouched attribute on
  the same heading (`background-video-muted="true"` → `=true`). Same meaning,
  but diff noise in the user's file (constitution II prefers untouched bytes).
  Follow-up: keep each attribute item's original text when it is not changed.
- Caveats as in 001's eval: one model, one phrasing, one problem type.
