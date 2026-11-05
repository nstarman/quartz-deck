# Quickstart run (T021, 2026-11-05)

Release binary, the Claude desktop app's browser pane, the 7-slide demo deck
from spec 004's run (under git). Timings are measured end to end:
- **SC-001:** from the person's action in the page to `GET /api/focus` (what
  `qd focus` reads) showing it.
- **SC-002:** from the moment `qd focus --set` is launched (wall clock, shell)
  to Reveal's `slidechanged` in the page (wall clock, same machine).

## §1 Reading the focus (US1)

| Check | Result | Target |
|---|---|---|
| SC-001: select an element (20, then 24 selections across 3 slides) | median 123 ms, p95 188 ms; second run all 111–140 ms | ≤ 0.5 s in 95% ✓ |
| Clicking the Methods list → `qd focus` | `methods/1`, `editing: true` (it opened for typing) | ✓ |
| Words selected while typing; source selection across two blocks; caret into another slide | reported (`where`, both refs, `line` kept) — browser test `focus_follows_the_person` | ✓ |
| Deck edited after the focus was taken | `version` ≠ `deck_version` | ✓ |
| `qd stop` → `qd focus` | `{"open": false}` | ✓ |

One sample in the first run never matched within 3 s: the very first
selection right after the page loaded (the slide was still changing when the
synthetic right-click landed); the 24-sample rerun had none.

## §2 Pointing the editor (US2)

| Check | Result | Target |
|---|---|---|
| SC-002: `qd focus --set <slide>`, 10 slides | slide shown 15–21 ms after the command (all `moved`) | ≤ 1 s ✓ |
| `qd focus --set figure/0` | Figure slide shown, the chart cell outlined, its source lines highlighted | ✓ |
| SC-003: typing while 20 `--set` arrive | all `deferred`, view never moved, typed text intact, "Go there" works — browser test `pointing_moves_or_offers` | ✓ |
| `qd focus --set nope` | `E_NOT_FOUND`, editor unmoved | ✓ |

## §4 Safety (SC-005, SC-006)

100 reads leave the deck byte-identical (`focus_over_http`); the cross-origin
attack page can neither read, plant nor point (`checklists/security.md`).

## Found during the run

Nothing new (the bugs found while building are recorded in PRs #15 and #16).
