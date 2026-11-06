# Menus (007)

The order is the order in `actions.json`. Keys are shown as on macOS; ⌘
becomes Ctrl elsewhere. The final keys are fixed in implementation, from an
inventory of today's keys. This is the shape:

- **File:** Settings… (⌘,) · Render · Export offline HTML · Export with PDF ·
  Check offline · Open preview ↗ · (010 adds export formats here)
- **Edit:**
  - Undo (⌘Z) · Redo (⇧⌘Z)
  - Cut · Copy · Paste · Duplicate (⌘D) · Delete (⌫)
  - Edit text (Enter) · Edit source (⌥Enter) · Move up/down (⌥↑/⌥↓) · Select
    next/previous element (Tab/⇧Tab)
  - Format ▸ (Bold, Italic, Underline, Strikethrough, Code, Link, lists) ·
    Appearance ▸ (Appears on click, Smaller text, Incremental list, Custom
    class…)
- **Insert:** Paragraph · List · Equation · Image… · Text box · Code cell ·
  Slide after (⇧⌘N) · Slide before
- **Slide:**
  - Duplicate (⇧⌘D) · Delete… · Move up/down · Hidden ✓
  - Background colour… · Background image… · Transition ▸
  - Speaker notes… · Edit slide source · Copy link
- **View:** Source pane ✓ · Source full width ✓ · Form panel ✓ · Previous/next
  slide (←/→) · Theme ▸
- **Run:** Run cell · Run all · Stop · Restart · Restart & run all · Add
  package… · Create environment. These are all `runs_code` and hidden when the
  run bar is external (spec 006).
- **Help:** Keyboard shortcuts (?) · About quartz-deck (version, licence)

Shortcut reference: a dialog with one section per menu, rows of label, key
and where it works, and a search box. It is opened with ? and closed with Esc.
