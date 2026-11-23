# Contract: the properties column in the editor

## Shape

- `#panel` with `.views` (a tablist: Format, Animate, Deck) above `.tabs`
  (Format's Look, Text, Arrange for an element), then the view's body.
- Shown unless `ui.inspector` is false (View ▸ Properties toggles it).
- `main[data-props]` gives its place from `properties.side`, and the CSS
  lays it out. Its header drags it to a zone (`ui/dock.js`, as spec 022).
- In a window 700 px wide or less, it is below the slide.
- Not in the presenter, the speaker view or exports.

## Format

- **Nothing selected:** background colour (swatches and a colour well),
  background image, hidden, title, classes, the slide's source, and its
  positioned boxes. On the title slide: title, subtitle, author, date.
- **An element:** the Look, Text (text elements only) and Arrange tabs, as
  research R2 sets out. At the foot: Run (code cells), Edit, Markdown,
  Delete.
- Each control writes through the same op as today, so the deck's text is
  unchanged by moving the control.

## Animate

- **Nothing selected:**
  - transition (a select with "default" meaning the deck's);
  - speed (default, fast, slow);
  - "Morph into the next slide";
  - the build list (below).
- **An element:**
  - Appear: a select, with "None" first;
  - Leave: a select;
  - for a list, "One item at a time";
  - the build list, with the element's row marked.
- **Build list:** one row per build, named. Drag a row, or ⌥↑ and ⌥↓ on a
  focused row, to reorder it. Each move is one `qd build order`.

## Deck

- Theme: a select of Quarto's built-in themes, plus the deck's own `.scss`
  if it names one.
- Slide size: 16:9, 4:3 or Custom (width and height).
- Slide numbers: a checkbox.
- Default transition: a select.
- Navigation: spec 018's three modes.

## Removed

- `#slidebar` and `#elbar` are no longer shown.
- The floating text toolbar (`#fmtbar`) stays beside text being typed. The
  Text tab has the same buttons.

## Tests (`tests/onslide.rs`)

- `props_format_slide`: with nothing selected, set the background colour,
  hidden and the title from the column. The deck's text matches the ops'.
- `props_format_element`: select a paragraph, a box, a picture, a diagram
  box and an arrow in turn. Each shows the right tabs, and the Look and
  Arrange settings write as today. No `#slidebar` or `#elbar` is visible.
- `props_text_tab`: while typing, the floating toolbar and the Text tab's
  bold both work, and the selection is kept.
- `props_animate`:
  - set a transition, a speed and morphing;
  - set appear and leave on two elements;
  - reorder the build list;
  - check the deck's text, and in presenting mode, the order the builds
    show in.
- `props_deck`: change the theme, the slide size (with a placed box kept in
  place), slide numbers and the default transition.
- `props_dock`: drag the column to each place, then reload; a narrow window
  puts it below; View ▸ Properties hides and shows it; with the source pane
  on the same side, both show in their set order.
- `props_keyboard`: F6 into the column, arrow keys along the views and tabs,
  Esc back to the slide.
