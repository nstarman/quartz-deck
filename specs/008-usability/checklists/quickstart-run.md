# Quickstart run (T040, 2026-11-07)

[quickstart.md](../quickstart.md) §1–§10, on a copy of the audit deck served
with the release build, in the Claude app's browser pane. Each step says whether
it was done by hand in the pane or rests on a browser test (headless Chrome).

| § | Story | How | Result |
|---|---|---|---|
| 1 | Slide list | by hand | Title slide first, 8 thumbnails. The + between Growth and Two views added an untitled slide there with "Click to add title". **Found:** clicking that placeholder did not start typing; fixed (an empty title is editable) and `slide_list_like_a_slide_app` now types into it. After the fix: typed "Added here", saved. Dragged Two views by its ⠿ to the top: it moved; ⌘Z put it back in one step. Esc during a drag: `slide_list_like_a_slide_app`. |
| 2 | Canvas | by hand + test | No reveal.js controls on the canvas (checked in the page); no bar covers content and the canvas doesn't jump: `nothing_covers_the_slide`, `audit_measures`. |
| 3 | Readable | test | `audit_measures`: 0 contrast, size or overlap failures in light, dark and high contrast; [shots-after](../audit/shots-after/) dark-\* and hc-\*. |
| 4 | Present | by hand + test | **Found:** the pane's embedded browser never answers a full-screen request, and the click did nothing visible; fixed: after 1.5 s without full screen (or on a refusal) the editor says "Full screen isn't allowed here: ▾ ▸ Open in browser tab presents in a tab instead" (seen in the pane). Full screen at the current slide, from the beginning, and back to editing: `present_from_here_and_from_start`, which now also covers the refused and the unanswered request. |
| 5 | Sheets and images | test (+ by hand earlier in the session) | Insert ▸ Image… listed `chart.svg` with a preview (by hand on the demo deck, and `own_sheets_and_image_drop`). A dropped PNG lands where dropped and in `images/`: the test's synthetic drop (the pane can't drag a file from the desktop). The link sheet refuses "not a url" with a message: the test. |
| 6 | Words | by hand | Tab title and top row "Quarterly results" with `talk.qmd`; status "Up to date" (shown only when it needs attention, as asked earlier); one row above the slides. During the run the status once read "Disconnected — is qd serve running?" for a moment while the event stream reconnected; it recovered by itself. |
| 7 | Bars | shots + test | Formatting bar with the common few and "Aa" ([light-06](../audit/shots-after/light-06-editing-text.png)); source bar ▶ Run all, ■ Stop, "Python · stopped", ··· ([dark-12](../audit/shots-after/dark-12-source-pane.png)); Insert with Code cell … Quote ([light-08](../audit/shots-after/light-08-menu-insert.png)); `tidier_bars_and_fuller_insert`. |
| 8 | Narrow | by hand (earlier) + test | At 375×812 in the pane: the slide picker lists every slide with ✓ on the current one, properties under ···. `narrow_window` at 420×760. |
| 9 | Polish | test | ⇧-click three slides and drag: they move together, one undo; ⌘-click and Delete removes two; "Saved" beside an edit: `finishing_touches`. |
| 10 | Audit again | script | `QD_SHOTS=shots-after uv run shoot.py`: 71 shots, none failed; [status.md](../audit/status.md) filled in. |

Two problems found and fixed in this run (§1 and §4); one transient
reconnect noted (§6).
