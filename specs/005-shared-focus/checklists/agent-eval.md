# Agent evaluation (T022, SC-004, 2026-11-05)

**Task.** 10 fresh general-purpose agents, each with its own copy of a 4-slide
deck. The Results slide has three paragraphs; the middle one is 45 words. They
were told only: the person has the deck open in the qd editor, "has just
selected something in it", and says **"make this shorter"**; use only `qd`
(path given; `qd skill` to learn it); you cannot ask them anything back.

**The person** (scripted, `eval5/person.py`): on each copy's own `qd serve`,
it held an editor tab open (`/events?tab=person`) and reported, through the
same token-guarded endpoint the editor page uses, a selection of the words
"The largest increase was at the eastern meadow" in `results/1`. The agents
started a few minutes after that report, so `age` was realistic, not fresh.

**Grader** (`eval5/grade.py`, scratchpad), from files and the per-copy `qd`
call log, not from the agents' reports. Checks:
- only that paragraph changed;
- it got shorter;
- the agent read `qd focus`;
- no usage errors (exit 2).

## Result — **10/10 PASS** (SC-004: 9 of 10)

| | |
|---|---|
| Edited exactly the selected paragraph, nothing else | 10/10 |
| Made it shorter (45 → ~22 words) | 10/10 |
| Read `qd focus` | 10/10 |
| Usage errors | 0 |
| Per-element guard (`--expect-text`) on the edit | 10/10 |
| `qd` calls | 4 × 4, 5 × 6 (the extra one: an optional `qd check`) |
| Sequence | `skill`, `focus`, `show results`, `edit results/1 --md … --expect-text …` |

None of them asked or hedged about which paragraph was meant.

**Fixed before the run:** the skill said to act on the focus when its `age`
was "a few seconds". A person selects something and then types to the agent,
so a minute or two is normal, and agents following the old wording would
have held back. It now says `age` counts from the last click, a minute or two
is normal, and to say what you're acting on only when it is much older.
