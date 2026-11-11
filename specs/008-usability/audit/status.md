# Audit status (008)

One row per finding in [README.md](README.md). Status: `open`, `fixed` (with
evidence), or `kept` (with the reason). Evidence is from the rerun after the
fixes (T039, 2026-11-07): `QD_SHOTS=shots-after uv run shoot.py`, all 71 shots, in
[shots-after/](shots-after/), and the browser tests named.

| Finding | Requirement | Task | Status | Evidence |
|---|---|---|---|---|
| L1 Adding a slide | FR-002, FR-003 | T008 | fixed | a + in every gap and at the end, untitled new slides ([light-02](shots-after/light-02-content.png)); `slide_list_like_a_slide_app` |
| L2 Dragging slides | FR-004, FR-005 | T009 | fixed | ⠿ handle, drag image, an opening gap, Esc cancels ([light-11](shots-after/light-11-slide-list-drag.png)); `slide_list_like_a_slide_app` |
| L3 No thumbnails | FR-001 | T006 | fixed | thumbnails of every slide ([light-02](shots-after/light-02-content.png)) |
| L4 Title slide not listed | FR-001 | T007 | fixed | listed first, unnumbered ([light-15](shots-after/light-15-title-slide.png)) |
| L5 Multi-select | FR-025 | T037 | fixed | ⇧-click and ⌘-click; drag, Delete, copy and cut act on the set; `qd move a b c`, `qd rm a b`; `finishing_touches` |
| C1 Bars cover the title | FR-006 | T012 | fixed | `nothing_covers_the_slide`, `audit_measures` (0 overlaps); [light-06](shots-after/light-06-editing-text.png) |
| C2 reveal.js chrome | FR-007 | T013 | fixed | `nothing_covers_the_slide` |
| C3 The canvas jumps | FR-008 | T013 | fixed | `nothing_covers_the_slide` |
| C4 Slide bar controls | FR-020 | T027 | fixed | one 28 px height, a name on each, Background opens the swatch sheet with None ([light-02](shots-after/light-02-content.png)); `tidier_bars_and_fuller_insert` |
| C5 Insert lacks blocks | FR-023 | T030 | fixed | code cell, callout, columns, speaker notes, table, quote ([light-08 Insert](shots-after/light-08-menu-insert.png)); `tidier_bars_and_fuller_insert` |
| C6 Hover handles | FR-011 | T014 | fixed (size); kept (marker) | + and ⠿ now 28 px, tooltips "Insert after" and "Drag to move" ([light-03](shots-after/light-03-hover-element.png)); the line showing where an insert goes was not built: the new element appears there at once and is one undo away |
| E1 Formatting bar | FR-021 | T028 | fixed | the common few up front, colours and "Aa" as pop-overs ([light-06](shots-after/light-06-editing-text.png)); `tidier_bars_and_fuller_insert` |
| E2 Where the hint lives | FR-026 | T037 | fixed | "Saved" beside the element after an edit; `finishing_touches` |
| E3 Code cells and Run | (012) | — | open | Run in the source pop-up and on the slide is 012's |
| B1 Box handles | FR-011 | T031 | fixed | 10 px to see, 24 px to hit, in screen pixels ([light-19](shots-after/light-19-positioned-box.png)); `tidier_bars_and_fuller_insert` |
| S1 Run bar | FR-022 | T029 | fixed | ▶ Run all, ■ Stop, "Python · stopped", ··· ([dark-12](shots-after/dark-12-source-pane.png)); `tidier_bars_and_fuller_insert` |
| S2 ▶ Run 17 px | FR-011 | T014, T029 | fixed | min-height 20 px; `audit_measures` |
| D1 Browser boxes | FR-014–FR-016 | T021 | fixed | all nine replaced by the editor's sheets; `no_native_boxes` scans for them; `own_sheets_and_image_drop` |
| D2 Settings | (US9) | T037 | fixed | Theme first, one width ([light-13](shots-after/light-13-settings.png)); `finishing_touches` |
| T1 Dark Transition menu | FR-009, FR-010 | T014 | fixed | themed form controls ([dark-02](shots-after/dark-02-content.png)); `audit_measures` |
| T2 Source colours | FR-010 | T014 | fixed | `audit_measures`; values and ratios in research R10 ([dark-12](shots-after/dark-12-source-pane.png)) |
| T3 Small controls | FR-011 | T014 | fixed | `audit_measures` (every control ≥ 20 px); box handles are B1 |
| T4 Muted text | — | — | kept | passes (5.3:1, 5.67:1) |
| T5 HC selection colour | FR-012 | T014 | kept | the slide is white in every theme: the blue outline is 5.87:1 on it, the HC yellow would be 1.07:1 |
| H1 Header and tab title | FR-017 | T024 | fixed | the deck's title and file in the top row, the tab named after the deck ([light-02](shots-after/light-02-content.png)); `one_top_bar_in_plain_words` |
| H2 No Present | FR-013 | T016 | fixed | ▶ Present and ▾ in the top row ([light-02](shots-after/light-02-content.png)); `present_from_here_and_from_start` |
| H3 Three bars | FR-018 | T024 | fixed | one top row: menus, title, Present; the status shows only when it needs you ([light-02](shots-after/light-02-content.png)) |
| H4 Jargon | FR-019 | T025 | fixed | plain names in every menu and bar; `one_top_bar_in_plain_words` checks none of the old ones remain |
| N1 VS Code ☰ row | FR-018 | T024 | fixed | ☰ in the one top row beside the title ([vscode-01](shots-after/vscode-01-panel.png)) |
| N2 Narrow window | FR-024 | T033 | fixed | a slide picker in the top row, properties under ··· ([narrow-01](shots-after/narrow-01-start.png)); `narrow_window` |

Result: 27 of the 30 findings fixed (C6 without its insertion marker), T4 and T5 kept with the reason, and E3 left to 012.
