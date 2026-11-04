# Agent evaluation (T037, 2026-11-04)

**Setup.** 10 fresh general-purpose agents, each on its own copy of a 3-slide
deck, given only the task ("change the first paragraph of the Results slide
to exactly …; a colleague is editing this deck in the browser right now —
don't overwrite their work; change it only through qd") and `qd` on PATH
(read the skill with `qd skill`). Meanwhile a scripted **person** edited the
same deck every 1.5 s for ~90 s — 60 edits to the Methods paragraph and to
the list on the *same* Results slide — through the same ops the browser
editor uses (`qd edit … --expect-text`). The person ran the CLI rather than
the browser harness: it is the same operation the editor sends, and it keeps
ten concurrent runs light. Grader: `eval3_grade.py` (scratchpad), from files
and logs, not from the agents' reports.

**Checks per run:** agent's paragraph exactly as asked; the person's last
Methods edit and last list edit both in the final file; the rest of the deck
byte-identical; the deck changed only through qd (undo journal); no usage
errors; whether the agent used the element-level guard.

**Result: 10/10 PASS.**

| | |
|---|---|
| Agent calls per run | 5 (`skill`, `slides`, `show results`, `edit results/0 --md … --expect-text <hash>`, `show results`) |
| Usage errors | 0 |
| Agents using `--expect-text` | 10/10 (the skill's new guidance) |
| Person edits refused / lost | 0 / 0 (600 edits across the runs) |
| Conflicts hit by agents | 0 — the person edited a different element of the same slide between every agent's read and write; with file-level `--expect` each agent would have had to retry |

Every agent noticed, unprompted, that the person's list edit on the same slide
happened between its read and its write and was kept.
