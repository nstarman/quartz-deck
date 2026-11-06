# Audit status (008)

One row per finding in [README.md](README.md). Status: `open`, `fixed` (with
evidence), or `kept` (with the reason). Evidence is filled in by T039 after
the audit rerun.

| Finding | Requirement | Task | Status | Evidence |
|---|---|---|---|---|
| L1 Adding a slide | FR-002, FR-003 | T008 | fixed | |
| L2 Dragging slides | FR-004, FR-005 | T009 | fixed | |
| L3 No thumbnails | FR-001 | T006 | fixed | |
| L4 Title slide not listed | FR-001 | T007 | fixed | |
| L5 Multi-select | FR-025 | T037 | open | |
| C1 Bars cover the title | FR-006 | T012 | fixed | `nothing_covers_the_slide`, `audit_measures` |
| C2 reveal.js chrome | FR-007 | T013 | fixed | `nothing_covers_the_slide` |
| C3 The canvas jumps | FR-008 | T013 | fixed | `nothing_covers_the_slide` |
| C4 Slide bar controls | FR-020 | T027 | open | |
| C5 Insert lacks blocks | FR-023 | T030 | open | |
| C6 Hover handles | FR-011 | T014 | open | |
| E1 Formatting bar | FR-021 | T028 | open | |
| E2 Where the hint lives | FR-026 | T037 | open | |
| E3 Code cells and Run | (012) | — | open | Run in the source pop-up is 012's |
| B1 Box handles | FR-011 | T031 | open | |
| S1 Run bar | FR-022 | T029 | open | |
| S2 ▶ Run 17 px | FR-011 | T014, T029 | fixed | min-height 20 px |
| D1 Browser boxes | FR-014–FR-016 | T021 | open | |
| D2 Settings | (US9) | T037 | open | |
| T1 Dark Transition menu | FR-009, FR-010 | T014 | fixed | `audit_measures` (themed form controls) |
| T2 Source colours | FR-010 | T014 | fixed | `audit_measures`; values and ratios in research R10 |
| T3 Small controls | FR-011 | T014 | fixed | `audit_measures` (checkbox 20 px, select 28 px); box handles are B1 (US7) |
| T4 Muted text | — | — | kept | passes (5.3:1, 5.67:1) |
| T5 HC selection colour | FR-012 | T014 | kept | the slide is white in every theme: the blue outline is 5.87:1 on it, the HC yellow would be 1.07:1 |
| H1 Header and tab title | FR-017 | T024 | open | |
| H2 No Present | FR-013 | T016 | open | |
| H3 Three bars | FR-018 | T024 | open | partly: the name and menus share one row, the status and Source button are gone (2026-11-06, the person); Present and the deck title come with US6 |
| H4 Jargon | FR-019 | T025 | open | |
| N1 VS Code ☰ row | FR-018 | T024 | open | partly: ☰ now sits in the one top row beside the name |
| N2 Narrow window | FR-024 | T033 | open | |
