# Quickstart: check the inspector

Uses a made-up deck only.

## Setup

```bash
cargo build --release
cp specs/008-usability/audit/deck/talk.qmd /tmp/inspector-demo.qmd   # any scratch folder
target/release/qd serve /tmp/inspector-demo.qmd
```

Open the printed editor link in a browser window at least 1400 px wide.

## Scenarios

1. **It shows**: the inspector is on the right with the Slide tab. There is
   no slide bar above the slide.
2. **Slide tab**: set the transition to `fade`. The slide's heading in
   the file gets `transition="fade"`, and ⌘Z undoes it.
3. **Element tab**: click a paragraph. Nothing floats over the slide, and
   the Element tab shows Edit, Markdown, Appears on click and Delete. Turn
   on Appears on click: the paragraph gets `.fragment`, as from the element
   bar.
4. **Text tab**: double-click the paragraph to type. The inspector switches
   to Text. Select a word and press Bold there: the word is bold and the
   caret stays in the text.
5. **Position tab**: select a box (Insert ▸ Text box). Type a new left
   edge: the box moves as from today's Properties panel.
6. **Narrow window**: shrink the window below 1200 px. The inspector hides,
   the selection stays, and the element bar floats over it as before. Widen
   it again: the inspector comes back on the same tab.
7. **Source pane**: open View ▸ Source pane with the window at 1400 px. The
   inspector hides if the slide would get narrower than 640 px.
8. **Hide and remember**: press ⌥⌘I. The inspector hides and the bars float.
   Reload: it stays hidden. Press ⌥⌘I again to bring it back.
9. **Keyboard**: press F6 to move focus to the inspector, use the arrows to
   change tabs and Tab to reach a field, then press F6 to return to the
   slide.
