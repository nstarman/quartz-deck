# Data model: An inspector on the right when the window is wide

Nothing is stored in the deck. The feature adds one setting and some page
state.

## Setting

| Key | Kind | Default | Meaning |
|---|---|---|---|
| `ui.inspector` | bool | `true` | The person wants the inspector when it fits (R5). |

Declared in `crates/qd/src/settings.rs` beside `ui.form_panel`; read and set
with `qd settings` and the editor's settings store (spec 007).

## Page state (editor only)

- **shown**: `ui.inspector` is true and the width check passes (R3). It is
  the class `inspector` on `<main>`.
- **tab**: one of `slide`, `title`, `element`, `text`, `position`.
  - Set from the selection: nothing selected gives `slide` (or `title` on
    the title slide); an element gives the last element tab; typing gives
    `text`.
  - Changed by the person's click or the arrow keys.
  - Not saved.
- **lastElementTab**: the element tab last chosen, kept for the session.

## Which tabs show

| What is selected | Tabs |
|---|---|
| Nothing, ordinary slide | Slide |
| Nothing, title slide | Title |
| A text element, list, quote, callout | Element, Text, Position |
| A box (positioned) | Element, Text, Position |
| A code cell, equation, raw HTML, table | Element, Position |
| A diagram box or arrow | Element (with its fields), Position |

Position lists move up/down for every element and position and size only
for boxes, as today's element panel does.
