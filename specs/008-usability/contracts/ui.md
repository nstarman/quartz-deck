# UI contract (008)

## Top bar (one row)

`[menus or ☰] [deck title · file name] [status] … [▶ Present ▾] [Source]`

- Status words: Saved · Saving… · Rendering… · Up to date · Changed on disk ·
  Disconnected — is qd serve running? (tooltip: the version).
- Tab title: the deck's title (else its file name).
- Present ▾: "From this slide" (default, the button) and "From the
  beginning".

## Slide list

- Title slide first (unnumbered, not draggable), then each slide: thumbnail,
  number, title (else first words, else "Untitled").
- Hover a gap: a + inserts there. A + after the last slide. Enter on a
  focused slide: a slide after it.
- Hover a slide: ⠿ handle. Drag: translucent thumbnail under the pointer, the
  list opens a gap at the landing place, auto-scroll near the edges, Escape
  cancels.
- Keys on a focused slide: ⌥↑/⌥↓ move, Delete removes, ⌘C/⌘X/⌘V as 007.
- ⇧-click / ⌘-click: several slides (P3).
- Under 600 px: a slide picker in the top bar instead.

## Canvas

- No reveal.js menu button, arrows or progress bar while editing.
- An empty slide title shows "Click to add title" (editor only).
- Element and formatting bars: above the element, else below, else docked at
  the top of the canvas area; never over another element.
- The slide bar keeps its row on every slide.

## Actions (added or renamed in `actions.json`)

| id | label | menu | key |
|---|---|---|---|
| present | Present from this slide | View | ⌥⌘P |
| present-start | Present from the beginning | View | ⇧⌥⌘P |
| preview | Open in browser tab | File | — |
| render | Render with Quarto | File | — |
| export | Export ▸ Web page (works offline) | File | — |
| export-pdf | Export ▸ Web page and PDF | File | — |
| insert-code | Code cell | Insert | — |
| insert-callout | Callout | Insert | — |
| insert-columns | Columns | Insert | — |
| insert-notes | Speaker notes | Insert | — |
| insert-table | Table | Insert | — |
| insert-quote | Quote | Insert | — |
| slide-move-up / -down | (as 007) | Slide | ⌥↑ / ⌥↓ in the list |

The VS Code key list check (007) applies to the new keys.

## Sheets

Anchored beside what opened them; fields with inline validation; OK / Cancel;
Escape cancels, Enter confirms. Used for: image (file list with previews +
path/URL field), link (address, validated), background colour and text colour
(swatches, None, Custom…), background image (as image), custom class, Python
packages, delete slide (only when the setting asks), reset settings, About.
