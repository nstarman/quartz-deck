# Quickstart run: the inspector

Run on 2026-11-19 against main after #100, #103 and #105, as browser tests
at 1440 × 900 (and 1000 × 800, 1300 × 900 where a scenario needs it). The
Browser pane was hidden, and a hidden page gets no resize events, so each
scenario was run by the test that drives a real window size.

- [x] §1 **It shows**: `inspector_shows_when_wide`, with the Slide tab and
  the slide bar inside. `inspector_slide_tab` checks there's no row above
  the slide.
- [x] §2 **Slide tab**: `inspector_slide_tab`. The transition from the bar
  writes `transition=fade`, and one undo puts it back.
- [x] §3 **Element tab**: `inspector_element_tabs`. The element bar sits in
  the inspector, not floating, and Appears on click from it adds
  `.fragment`.
- [x] §4 **Text tab**: `inspector_text_and_position` (added in this
  polish). Typing switches to Text, Bold from the inspector's formatting
  bar makes the selected word bold, and Esc saves it.
- [x] §5 **Position tab**: `inspector_text_and_position`. A box's left edge
  typed in Position moves it, as the Properties panel did.
- [x] §6 **Narrow window**: `inspector_follows_the_width`. The selection,
  the tab, a typing session and an unsaved field survive 1440 → 1000 →
  1440, and the element bar floats again below the width.
- [x] §7 **Source pane**: `inspector_and_panes` at 1300 px. The source pane
  and History each hide the inspector and bring it back when closed.
- [x] §8 **Hide and remember**: `inspector_toggle_is_remembered`. ⌥⌘0 hides
  it, a reload keeps it hidden, ⌥⌘0 shows it.
- [x] §9 **Keyboard**: `inspector_keyboard`. F6 to the shown tab and back,
  ← → along the tabs, and Esc from a field back to the slide.

Not run here: trying it by hand on the person's own screens (T018).
