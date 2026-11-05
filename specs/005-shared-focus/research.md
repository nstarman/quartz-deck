# Research: Shared Focus

All decisions below resolve the plan's open points; no NEEDS CLARIFICATION
remains. Each is grounded in what 003/004 already built (`state.anchor`,
`state.sel`, the source pane's `onCaretLine`, the session token, SSE events).

## R1 — How the focus gets from the editor to `qd`

- **Decision**: the editor page sends its whole focus snapshot to the server
  (`POST /api/focus`, token-guarded) whenever it changes, debounced ~100 ms.
  The server keeps the latest snapshot per editor tab in memory; `qd focus`
  asks the running server (`GET /api/focus`, token from the session file).
- **Rationale**: the server is already the meeting point of the editor and the
  CLI (edits, runs); one small JSON object per change is cheap and keeps
  SC-001 (≤ 0.5 s) easy. Snapshots, not deltas, so a lost request is healed by
  the next one.
- **Alternatives**: writing focus to a file the CLI reads — rejected (FR-010:
  never on disk). The CLI querying the page over the SSE channel on demand —
  needs a request/response protocol over a one-way stream; more code, no gain.

## R2 — "Is an editor open?" and which tab counts

- **Decision**: each editor tab makes a random tab id at load and opens its
  event stream as `/events?tab=<id>`; the server counts a tab as open while
  that stream is connected (a guard dropped when the stream ends). `open` =
  any connected tab. The focus reported is the latest snapshot from a still
  connected tab; a tab reports again when the window gains focus or becomes
  visible, so "most recently used" (FR-006) is simply "latest report".
- **Rationale**: the event stream already exists and ends exactly when the tab
  goes away; no heartbeat traffic.
- **Alternatives**: heartbeats with expiry — extra requests every few seconds
  and a stale window either way.

## R3 — Pointing the editor (`--set`) and "deferred"

- **Decision**: `POST /api/focus/set {ref}` (token) validates the ref against
  the deck (`E_NOT_FOUND` as elsewhere), then emits an SSE event
  `point {tab, anchor, ref}` for the most recent tab. That tab either goes to
  the slide and flashes the element (and reveals its source lines), or — if it
  is editing — shows a dismissable "Go to …" offer. The HTTP answer says
  `moved` or `deferred` from that tab's last reported `editing` flag;
  `no_editor` when none is connected.
- **Rationale**: the page and the server decide from the same flag; no extra
  round trip, so SC-002 is one event hop.
- **Alternatives**: waiting for an acknowledgement from the page — exact, but a
  pending-request table and a timeout path for a race that only changes the
  wording of the result. Revisit only if tests show the two disagree.

## R4 — "Typing in progress" (FR-008)

- **Decision**: the editor reports `editing: true` while an on-slide edit
  session is open (`qdSession() !== null`) or within 3 s of the last keystroke
  in the source pane, and reports `false` once that ends (a timer re-reports
  after the quiet period).
- **Rationale**: matches what can lose a keystroke: moving the slide closes the
  slide edit; scrolling the pane moves the caret.

## R5 — Selected text and elements

- **Decision**:
  - Slide view: the element is `state.sel`; selected text is the selection
    inside the open edit (or the preview frame's selection) if non-empty.
  - Source pane: the textarea selection; the elements are every block whose
    `lines` range intersects the selected lines (blocks' `lines` are in
    `qd show`, already fetched by the editor); with only a caret, the block
    under the caret line.
  - Text is cut at 2,000 characters in the page (`truncated: true`); the server
    clamps again.
- **Rationale**: reuses 004's line ranges; no new parsing in the page.

## R6 — References that go stale (edge case)

- **Decision**: the snapshot carries the deck version the page computed it
  against. On read, the server keeps only refs that still resolve in the
  current deck and reports both versions; an agent sees `version` ≠ the deck's
  and can re-read.
- **Rationale**: cheap, and never hands an agent a ref that errors.

## R7 — One shared focus (FR-012)

- **Decision**: a new editor module `src/ui/focus.js` owns the focus object:
  `set(partial)` merges, reports to the server, and notifies listeners. The
  slide view's selection, slide navigation and the source pane's caret all go
  through it; 004's source↔slide sync becomes a listener instead of a direct
  call. Spec 006 (VS Code) will read and set the same focus through the same
  endpoints.
- **Rationale**: one notion of focus instead of three ad-hoc ones; the
  endpoints are the integration point for later clients.

## R8 — Access rules (FR-009, FR-010)

- **Decision**: all three endpoints use 004's `authorized()` (loopback Host and
  Origin + `X-QD-Token`). A tab opened without the token link cannot report
  (the editor says so once). Focus lives only in server memory; it is gone when
  the server stops.
- **Rationale**: the selected text is deck content; it gets the same
  protection as running code.

## R9 — CLI shape

- **Decision**: `qd focus` (read) and `qd focus --set <ref>`. With no running
  server: `{"open": false}` (exit 0) for a read, `{"result": "no_editor"}` for
  a set. Uses 004's redirect-free loopback client (`to_server`), extended to
  GET.
- **Alternatives**: `qd show-slide` alias — not needed (spec Assumptions).
