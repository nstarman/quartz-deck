# Editor verification (T026, US4)

Run 2026-11-04 in the Claude desktop browser pane against `qd serve` on
`tests/fixtures/offline/math.qmd` plus a slide `clip` whose background video
`media/missing.mp4` did not exist.

- [x] Header shows **Export** and a **PDF** checkbox next to Render / Preview.
- [x] Export with the file missing → panel: "Not exported: 1 problem(s); the
      previous export is unchanged", listing `P_MISSING_FILE · clip` with the
      message and hint.
- [x] Clicking the problem → preview shows slide `clip`.
- [x] After adding the file, Export with PDF → "Exported: 4 slides, works
      offline; PDF 4 pages", with links; `/preview/export/deck.html` (5.7 MB,
      text/html) and `/preview/export/deck.pdf` (application/pdf) both 200.
- [x] Export folder: `deck.html`, `deck.pdf`, `media/missing.mp4`, `.qd-export`;
      no `.qd-` leftovers in the deck folder.

Found and fixed during this run:
- A failed export still showed "Open offline deck ↗" (nothing had been
  written) → links only on success.
- Navigating to the **last** slide landed on the one before it: the pane had
  narrowed after Reveal was ready, switching it into scroll view, where hash
  navigation to the last slide cannot scroll further. The editor now always
  disables scroll view at ready (previously only if it was active then).
