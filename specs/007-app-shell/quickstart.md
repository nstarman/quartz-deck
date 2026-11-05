# Quickstart: App shell (007)

Prerequisites: `cargo build --release`, Quarto, a deck (`qd new talk.qmd`).

1. **Menus (US1).** `qd serve`, then open the printed link.
   - Without right-clicking, use the menus to add a slide, duplicate it,
     insert a paragraph, make it bold, toggle the source pane, Run all
     (if the deck has cells), Render, Export.
   - Press F10, then use the arrows, Enter and Escape to run an item from each
     menu.
2. **Settings (US2).** ⌘, → set the source pane to the right and the theme to
   dark.
   - `qd stop`, then `qd serve` a different deck: both settings hold.
   - `qd settings set theme light`: open editors switch at once.
3. **Shortcuts (US3).** Press ? and search "slide". Try each key listed;
   `qd actions` lists the same.
4. **Consistency (US4).** On a slide, start editing text, open a menu, then
   press Esc repeatedly: menu, then edit (kept), then selection, one level per
   press. Copy a slide, open another deck, paste it, then undo it.
5. **VS Code (US5).** Open the deck in the 006 panel: one ☰ menu, and no
   VS Code key taken (⌘P and ⌘B do VS Code's thing).
