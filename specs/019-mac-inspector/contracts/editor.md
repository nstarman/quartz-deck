# Contract: the inspector in the editor

## Layout

- `<aside id="panel">` becomes the inspector. It sits right of the stage
  (left of it with the source pane on the right, as the panel does today).
- The width is 320 px, the same as today's panel.
- `<main class="inspector">` means it shows. Without the class, everything
  is as before this feature.

## Structure

```html
<aside id="panel" aria-label="Inspector">
  <div role="tablist" aria-label="Inspector">
    <button role="tab" id="tab-slide" aria-controls="tp-slide" aria-selected="true">Slide</button>
    <button role="tab" id="tab-title"     …>Title</button>     <!-- title slide only -->
    <button role="tab" id="tab-element"   …>Element</button>
    <button role="tab" id="tab-text"      …>Text</button>
    <button role="tab" id="tab-position"  …>Position</button>
  </div>
  <section role="tabpanel" id="tp-slide" aria-labelledby="tab-slide">…</section>
  …
</aside>
```

Tabs that don't apply have `hidden`.

## What each tab holds

| Tab | Holds (all existing controls, R2) |
|---|---|
| Slide | `#slidebar`'s controls; the slide panel's fields (title, background, image, transition, classes, body); Add box |
| Title | the title panel's fields |
| Element | `#elbar`'s buttons; kind and ref; the diagram or arrow fields; Markdown with Apply |
| Text | `#fmtbar`; a note when not typing |
| Position | Move up, Move down; the position and size fields (boxes) |

## Behaviour

- **Hidden bars**: while it shows, `#slidebar`, `#elbar` and `#fmtbar` are
  never shown floating, and the slide bar's row above the slide is gone.
- **Edits**: every control calls the same handler as before (same `qd` op,
  same undo step).
- **Tab choice**: the selected tab follows the selection (data-model.md).
  A click or the arrow keys change it.
- **Hiding by width**: when the window or the panes leave less room than
  needed (R3), the class goes off, the nodes go back where they were, and
  the selection, the typing session and any unsaved field stay as they
  were.

## Actions (`actions.json`)

| id | label | menu | key | notes |
|---|---|---|---|---|
| `inspector` | Inspector | View | `Alt+Mod+I` | toggle; sets `ui.inspector` |
| `form-panel` | Properties panel | View | — | unchanged id; on a wide window shows the inspector |

## Keys

- **F6** moves focus between the slide and the inspector (`keys.js` list).
- **Arrow keys** move between tabs.
- **Escape** in a field returns focus to the slide.
