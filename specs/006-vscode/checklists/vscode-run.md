# VS Code run (T014): quickstart §1–§2 in the real app

**Run**: 2026-11-05, macOS, VS Code Extension Development Host (extension
loaded from `vscode/`), `qd` from `target/release`, the quickstart demo deck
(`talk.qmd`, 8 slides). No Quarto extension installed, so the editor's own
Run stays visible. Driven by Claude with click-only access; the keyboard
steps are the person's.

## Done by click

- [x] "Open in quartz-deck" (editor title button) opens the panel beside the
      text, connected, in VS Code's dark theme; first render shows in ~6 s
      (cold Quarto). SC-001 (≤ 5 s on a warm machine) not timed precisely.
- [x] An agent's `qd title @2 …` lands in the buffer: the tab is marked
      unsaved, the panel's slide list updates, `talk.qmd` on disk unchanged.
- [x] The panel's Undo reverts it in the buffer.
- [x] "+ Add slide" in the panel adds `## New slide {#new-slide}` to the
      buffer as an unsaved edit; disk unchanged.
- [x] Closing the panel stops `qd serve` and removes the buffer overlay;
      reopening with an unsaved buffer attaches that text.

## Found and fixed

- In a narrow panel the header did not wrap (Render, Export, Source cut off)
  and "+ Add slide" sat at the far end of the side-scrolling slide strip.
  Fixed in `src/ui/editor.css`: the header wraps; in the narrow layout
  "+ Add slide" comes first in the strip.

## Left for the person (needs the keyboard)

- [ ] Typing in the buffer shows in the panel within 3 s, unsaved (SC-002).
- [ ] ⌘Z in the text undoes a panel edit (SC-003, VS Code's undo).
- [ ] ⌘S writes the file; the panel and disk agree.
- [ ] Switching VS Code to a light theme switches the panel.
