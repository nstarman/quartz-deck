# Quickstart run (T036, 2026-11-04)

Release binary, browser pane at 1280×760, a fresh deck under git so the diff
shows exactly what the editor wrote. Source panel closed throughout.

## SC-006: six-step task without the Source panel

| # | Task | How | Result |
|---|---|---|---|
| 1 | Retitle "Methods" → "Methods and data" | click title, type, Enter | ✓ `## Methods and data {#methods transition=fade}` (old auto id pinned, by design) |
| 2 | Add a sentence to the paragraph | click, type, click away | ✓ only the typed text changed (the click landed before the final `.`, so the run reads "here And there.." — operator slip, written exactly as typed) |
| 3 | Bold a list word | click, double-click word, **B** | ✓ `- **one**` |
| 4 | Slide transition | slide bar Transition ▸ fade | ✓ `transition=fade` on the heading |
| 5 | List appears on click | right-click ▸ Appears on click | ✓ wrapped in `::: {.fragment}` |
| 6 | Insert a paragraph after the list | hover **+** ▸ Paragraph, type | ✓ `A closing remark.` |

All six without help, about 2 minutes of interaction (well under SC-006's 3).
`git diff` shows only those changes.

## Quickstart sections

- §1 typing, §2 toolbar: covered above and by `typing_writes_only_the_element`,
  `toolbar_formats_and_unformats`.
- §3 source in place: equation edited in place earlier this session; refusal of
  a new slide heading by `source_editor_opens_over_math_and_refuses_new_slides`.
- §4 menus and structure: steps 4–6 above; `context_menus_list_and_run_actions`,
  `insert_drag_delete_and_empty_slides`.
- §5 with an agent: conflict bar exercised by hand; `same_element_conflict_offers_both_versions`
  and the 50-edit stress test.
- §6 coverage: `checklists/coverage.md`.

## Found and fixed during the run

- Escape after closing a menu also reached Reveal, which opened its overview
  grid. Reveal's Escape binding is now off in the editor, and keys the editor
  handles stop there.
- The Transition select and the Hidden/PDF checkboxes had no accessible names
  (read as "default"/"on"); explicit labels added.
