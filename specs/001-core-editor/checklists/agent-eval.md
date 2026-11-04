# Agent evaluation (T052, SC-001)

Target: an LLM agent given only the Skill completes a 5-step edit task with
zero syntax errors on its first attempt in ≥ 9 of 10 trials.

**Result (2026-11-04): 10/10 passed, 0 errors of any kind.**

## Setup

- 10 fresh subagents (Claude Sonnet, no prior context), run in parallel.
- Each got: the path to `skills/quartz-deck/SKILL.md`, its own folder with
  `tests/fixtures/basic.qmd` as `deck.qmd` and a real `figures/plot.png`, and a
  `bin/qd` wrapper that logs every call (exit status, error code, arguments).
- Same prompt for all (no qd syntax given):
  1. add a slide "Methods" after Intro with bullets "Data" and "Model";
  2. move Conclusion directly after Methods;
  3. rename "Results" to "Findings";
  4. give Findings the background colour #1a1a2e;
  5. put `figures/plot.png` on Findings as a positioned box at left 600, top 150, 400×300 px.
- Rules: change the deck only through qd; no preview server.

## Grading (objective, not the agents' own reports)

Per run, from the final deck and the call log (`eval_grade.py`):
slide order `Intro, Methods, Conclusion, Findings, (rule)`; Methods has a list
with Data and Model; Findings has `background-color="#1a1a2e"`; a box with
`figures/plot.png` and geom 600/150/400/300; `qd check` ok; deck equals the last
journaled qd write (no hand edits); no exit-2 / `E_USAGE` / `E_INVALID_ARG`.

| Run | Pass | qd calls | Errors |
|---|---|---|---|
| 01–10 | 10/10 | 6–8 each (incl. read-only `slides`/`check`) | 0 |

## Observations

- All ten converged on essentially the same five commands:
  `add --after intro --title "Methods" --md $'- Data\n- Model'`,
  `move conclusion --after methods`, `title results "Findings"`,
  `set results --attr background-color="#1a1a2e"`,
  `box add results --image figures/plot.png --at 600,150 --size 400,300`.
- Caveat: one model, one phrasing, and the task resembles the Skill's worked
  examples, so this shows the Skill is sufficient, not that it is robust to
  harder or ambiguous tasks. A follow-up eval should vary model and task.
- Only 3/10 passed `--expect` although the Skill says to; harmless here (no
  concurrent writer), but the guidance is not being followed.
- 6/10 ran `qd check`; all of those correctly identified that its warning
  (`missing file 'plot.png'`) came from the fixture's pre-existing box, not theirs.
