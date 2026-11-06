# Quickstart run (T026, 2026-11-05)

Quickstart §1–§4, run by hand (driven by Claude) in the Claude app's browser
pane on the release build, with two fresh decks and `QD_CONFIG_DIR` pointed
at a scratch folder so the person's own settings were untouched. §5 (VS Code)
needs the person, like 006's T022.

## §1 Menus (US1): PASS

- With the mouse only: Insert ▸ Slide after added a slide; Slide ▸ Duplicate
  slide copied it; Insert ▸ Paragraph inserted one and opened it for typing;
  with its text selected, Edit ▸ Format ▸ Bold made it `**New paragraph.**`
  (the edit stayed open while the menus were used); View ▸ Source pane showed
  the pane; File ▸ Render wrote `talk.html`; File ▸ Export offline HTML wrote
  `export/talk.html`.
- Run all was skipped: the deck has no code cells (`run_cells_in_the_editor`
  covers it).
- Keyboard: F10 → File; → ×4 → View; Enter opened it with "Source pane"
  focused; Enter ran it (pane hidden) and focus went back to the slide. F10
  then Esc returned focus to where it was.

## §2 Settings (US2): PASS

- ⌘, opened Settings with the six settings at their defaults. Source pane →
  Right and Theme → Dark applied at once; `settings.json` was written with
  mode 600.
- `qd stop`, then `qd serve` on the other deck (new port): both held.
- `qd settings set theme light` from the terminal: the open editor turned
  light within a second.

## §3 Shortcuts (US3): PASS

- ? opened the reference: 33 keys, the same 33 actions that have a key in
  `qd actions`. Searching "slide" left the five slide actions.
- Esc closed it and focus went back to the slide list. ⇧⌘D duplicated the
  slide; ⌘Z undid it. (Every listed key is also run by
  `menu_bar_runs_every_action`.)

## §4 Consistency (US4): PASS, with one fix and one gap

- **Fixed during the run:** with a menu opened by mouse over an edit, the
  first Esc closed the menu *and* ended the edit (the edit was kept, but a
  level was skipped). The editor now stops that Escape at the menu;
  `same_rules_everywhere` checks this case and fails without the fix. After
  the fix: menu, then edit (saved), then selection, one level per Esc.
- **Not verified here: a slide copied in one deck, pasted in another.** ⌘C
  in the slide list said "Slide copied", but ⌘V in the other deck (another
  page) said "Nothing to paste": the editor reads the system clipboard with
  `navigator.clipboard.readText()`, which needs clipboard-read permission. In
  this pane that permission is refused; Chrome asks the person once. Copy
  and paste within one deck, the fresh ids and the single undo are covered
  by `same_rules_everywhere` and `tests/cli.rs` `paste_slides_from_stdin`.
  Fixed afterwards (branch `clipboard-events`): ⌘V now takes the text from
  the browser's own paste event, which needs no permission (and is how VS
  Code's webview delivers ⌘V); `paste_key_uses_the_paste_event` checks it.
  Copying already worked: writing the clipboard needs no permission.
- Small: after undoing a duplicate that was on screen, the preview went to
  the first slide rather than the neighbouring one.

## §5 VS Code (US5): left to the person

- Automated: `framed_editor_takes_theme_and_messages` (no menu bar, ☰ menu,
  F10, ⌘B and ⇧⌘D reach nothing) and `vscode/test/keys.test.js`. Whether VS
  Code itself still takes those keys from the webview needs a real VS Code
  run.
