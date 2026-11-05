# Agent evaluation (T037, SC-005, 2026-11-04)

**Task.** Fresh general-purpose agents, each on its own copy of a 4-slide deck
with a locked uv environment (matplotlib), were told only: the code on the
"Results" slide fails; fix it so the chart is drawn; use only `qd` (path
given; `qd skill` to learn it); don't edit the file any other way or change
anything else. The bug: `plt.bar(range(len(value)), values)` — a `NameError`
whose fix needs the variable name from another slide's cell.

**Grader** (`eval4/grade.py`, scratchpad), from files and logs, not the
agents' reports. Each copy had its own `qd` wrapper logging every call and
its exit code.
- Fixed: a fresh `qd run results/0 --fresh` by the grader succeeds and
  draws an image.
- Nothing else changed: the deck outside the cell body is byte-identical.
- No usage errors (exit 2).
- The change went through a `qd` edit command.
- At most 6 `qd` calls, counting `qd skill`.

## Round 1 — 2/10 PASS (SC-005 not met)

| | |
|---|---|
| Correct fix (`len(values)`), nothing else changed | 10/10 |
| Usage errors | 0 |
| `qd` calls | 6, 6, 7, 7, 7, 7, 8, 8, 8, 8 |
| `--expect` on the edit | 10/10 |

Where the extra calls went:
- 4 runs called `qd replace --help`: the skill named `qd replace` only in
  passing, with no flags or match rule.
- A second `qd show` to check the variable's name on the Setup slide.
- A `qd run` before the fix to see the error.
- One run ended with a `qd check`.

**Changes from round 1** (commit "Agent eval round 1 findings"):
- `qd run` results carry the deck's `version` (runs never change the file),
  so the following `--expect` needs no extra read.
- The skill gains a `qd replace` row (exact, unique match; `--expect`) and a
  "Fix a failing code cell" worked example (`run --all` → `replace` →
  `run --all`). It uses different code from this eval's, so it does not hand
  over the answer.

## Round 2 (10 fresh agents) — **9/10 PASS** (SC-005 met: 9 of 10)

| | |
|---|---|
| Correct fix, nothing else changed | 10/10 |
| Usage errors | 0 |
| `--help` calls | 0 |
| `--expect` on the edit | 10/10 |
| `qd` calls | 6 × 9, 7 × 1 |
| Typical sequence | `skill`, `slides`, `show results`, `run --all`, `replace --old … --new … --expect …`, `run --all` |

The one run at 7 calls also read the Setup slide (`show setup`) to confirm
the variable's name before editing — a careful, reasonable extra read.

**Noticed by an agent, left alone as asked:** the cell's last line
`plt.xticks(...)` returns tick objects that show as a text output above the
chart. It is the deck's code, not qd's (Jupyter prints the last expression);
an author would add `;`.
