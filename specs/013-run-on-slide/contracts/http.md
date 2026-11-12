# HTTP contract: Run on the slide (013)

Extends 004's run endpoints and events (`specs/004-source-and-run/contracts/http.md`).
No new endpoints. Every run endpoint still requires the session token
(`X-QD-Token`).

## Changed

| What | Change |
|---|---|
| `POST /api/run` | Runs with the deck's engine (research R2). A missing engine, kernel or language is `400 E_ENGINE`, and the hint names what is missing. |
| SSE `cell_output` | Adds `"html"`: the preview's block for that cell (`<div class="cell-output qd-output">`). The editor swaps it in place inside the cell's `[data-qd="REF"]` wrapper. |
| SSE `kernel` | Adds `"engine": {"kind", "label"}`, so the badge can say `R · idle` instead of `Python · idle`. |
| `GET /api/outputs` | Adds `"engine": {"kind", "label", "kernel"?}` or `{"error"}` when the engine can't run here. Each cell in `order` gains `"ojs"`: an Observable JS cell, which runs in the page, never in a session. |
| Preview HTML | Cell wrappers gain the class `qd-cell`, which the editor uses for its "Code cell, not run yet" placeholder (never shown when presenting). Nothing else in the rendered deck changes. |

## Unchanged

- `POST /api/run/interrupt` and `POST /api/run/restart`. They now stop and
  restart whichever engine's session is live.
- The order of events: `run_started`, then `cell_output` per cell in order,
  then `run_done`.
