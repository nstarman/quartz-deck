# Research: One properties column

## R1. Grow spec 019's inspector into the column

**Decision**: the column is `#panel`, the element spec 019's inspector
already turns into a tabbed column (`ui/inspector.js`). This spec changes
four things:
- it is always shown (no wide-window rule), unless View ▸ Properties
  hides it;
- it gets the three views, Format, Animate and Deck;
- the slide bar and element bar stop being shown;
- it docks like the source pane (R5).

**Why**: the inspector already moves the slide bar, element bar and text
toolbar into tabs, follows the selection, has keyboard handling (F6, arrow
keys), and is tested. Building a second column would duplicate all of it.

**Alternative**: a new element beside `#panel`. It would mean two
side panels to keep in step, and the old panel would still be there.

## R2. Format's tabs: what goes where

| Selection | Tabs | What each holds (today's source of it) |
|---|---|---|
| Nothing (a slide) | one view | Background colour and image, transition-free settings (hidden, classes), title, the slide's source, positioned boxes list (slide panel, slide bar) |
| The title slide | one view | Title, subtitle, author, date (title panel) |
| An element | Look | Kind, classes and their presets (smaller text, custom class), a box's background colour, a diagram box's look, link, picture and size; an arrow's colour, head and style (element bar, element panel, diagram and arrow panels) |
| | Text | Only for an element that holds text: the text toolbar's controls (bold, italic, colour, font, size, alignment, lists, indent) (text toolbar) |
| | Arrange | Place freely or put back in the flow, size and position, layer order (front, forward, backward, back), a diagram's pin (Arrange menu, element panel's geometry) |
| | (footer) | Run, edit, markdown, delete, as buttons at the foot (element bar) |

The text toolbar keeps floating beside text being typed. The Text tab holds
a second set of the same buttons, bound to the same `toolbar.js` actions.
Those buttons take no focus, so the text being typed keeps its selection.

## R3. Animate: transitions and builds as Quarto writes them

Nothing new is invented. These are reveal.js features Quarto passes through.

- **Slide transition**: the slide attributes `transition=` (none, fade,
  slide, convex, concave, zoom), `transition-speed=` (default, fast, slow)
  and `auto-animate=true` (morph into the next slide). They are written by
  the existing `qd set SLIDE --attr k=v` / `--unset k`.
- **Appear**: the element gets `.fragment`, plus an effect class: `fade-in`
  (the default, written as no class), `fade-up`, `fade-down`, `fade-left`,
  `fade-right`, `grow`, `shrink`, `strike`, or `highlight-red`, `-green`,
  `-blue`, or `highlight-current-red`.
- **Leave**: reveal.js has one-element appear-then-leave classes,
  `fade-in-then-out` and `fade-in-then-semi-out`, and leave-only classes,
  `fade-out` and `semi-fade-out`. Animate offers "leaves on a later click":
  none, fade out, or fade to half. The class pair is chosen from the appear
  and leave together.
- **A list's items one by one**: `.incremental` on the list (Quarto), as
  today's "Incremental list" action.
- **Build order**: reveal.js orders a slide's fragments by
  `data-fragment-index`, which Pandoc writes from the attribute
  `fragment-index=N`. With none set, the order is source order, which is
  what qd does today. The build list writes `fragment-index` on every
  build of the slide when the order is changed, and removes them all when
  the order is back to source order.

**New CLI ops**:
- `qd build set REF [--appear EFFECT|none] [--leave EFFECT|none]` sets an
  element's fragment classes in one patch, as one undo step.
- `qd build order SLIDE REF…` writes the slide's `fragment-index`es.
- `qd build list SLIDE` lists the slide's builds in order, with each one's
  effects and a short name.

## R4. Deck: dotted keys for `qd meta set`

**Decision**: `qd meta set` and `qd meta unset` accept a dotted key, e.g.
`format.revealjs.theme`. The path's maps are created when missing, and
emptied ones are removed on unset. The Deck view writes these keys:
- `format.revealjs.theme`;
- `format.revealjs.width`, `.height`, as presets 16:9 = 1050×590 and
  4:3 = 1050×788, or custom;
- `format.revealjs.slide-number`;
- `format.revealjs.transition`;
- `format.revealjs.navigation-mode` (spec 018 has `qd navigation`, which it
  uses).

**Why**: Quarto reads these under `format: revealjs:`, and a top-level key
doesn't reach reveal.js for most of them.

**Slide size**: positioned boxes are written in slide pixels, so a new
size could put them off the slide. The Deck view's size change scales
every positioned box and arrow with the slide, in the same patch (FR
"keep their place relative to the slide"). This becomes
`qd meta size W H [--keep-places]`.

## R5. Docking: spec 022's way, for a second pane

**Decision**: `ui/dock.js` drags the source pane by its title bar and
writes `source.side`. It is made to take a pane and its setting, and the
column uses it with `properties.side`: `right` (default), `far-left`,
`left` or `below`. The layout CSS that spec 022 keyed on
`main[data-source]` gains `main[data-props]`.

When both panes want the same side, the order is fixed:
- **right:** slide, properties column, source pane (source furthest out);
- **left:** source pane, properties column;
- **below:** stacked in the same order.

In a window 700 px wide or less both go below the slide, as spec 022 does
for the source pane.

`ui.inspector` (spec 019's on/off) becomes the column's shown/hidden
setting, under the same View ▸ Properties action.

## R6. Several elements selected: not yet

The editor selects one element at a time (diagram boxes aside), so the
spec's "several elements selected" edge case has nothing to act on. This
spec doesn't add multi-select. When it comes, Format will show the settings
the selected elements share. The spec's edge case is marked as waiting on
multi-select.

## R7. Keyboard and labels

The inspector's F6 (focus into the column and back) and its arrow keys
along the tabs extend to the views. The views are a tablist of their own
above the tabs. Every control keeps a visible label or an `aria-label`.
The build list is a list of buttons, reordered by drag or by ⌥↑ and ⌥↓ on
the focused row.

## R8. What leaves the UI

- `#slidebar`: removed from the page. Its controls are rebuilt in Format.
- `#elbar`: removed. Its actions are in Format's footer and the
  right-click menu, as today.
- `#panel`'s old slide and element forms: rebuilt as Format's sections.
- The `inspector.fits()` rule: removed.

The tests that use these through the bars (`bars_set_properties…`,
`tidier_bars…`, the inspector tests) move to the column's controls, with
the same checks on the deck's text.
