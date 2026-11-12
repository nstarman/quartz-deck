# Action inventory (T001, 2026-11-05)

Every action the editor offered before 007, where it was, and the id it has
in `src/ui/actions.json`. `tests/actions.rs` reads the **Id** column: each id
must be in the list (SC-001). Rows marked *exempt* are not actions, and the
reason is given.

## Header (index.html)

| Control | Key before | Id |
|---|---|---|
| Undo | ⌘Z | `undo` |
| Redo | ⇧⌘Z | `redo` |
| Preview ↗ | — | `preview` |
| Render | — | `render` |
| Export | — | `export` |
| PDF checkbox (with Export) | — | `export-pdf` |
| Source | — | `toggle-source` |

## Slide list

| Control | Key before | Id |
|---|---|---|
| + Add slide (after the current slide) | — | `slide-after` |
| Drag to reorder | — | `slide-move-up`, `slide-move-down` |

## Source pane bar

| Control | Key before | Id |
|---|---|---|
| Run all | — | `run-all` |
| Stop | — | `run-stop` |
| Restart | — | `run-restart` |
| Restart & run all | — | `run-fresh-all` |
| Add Python package… (was + Package) | — | `run-add-package` |
| Set up Python… (was Create environment) | — | `run-env` |
| Full width | — | `source-full` |
| Properties (was Form) | — | `form-panel` |
| (typing) save now | ⌘S | `source-save` |
| (typing) run the cell at the caret | ⇧Enter | `run-cell` |

## Slide properties bar

| Control | Key before | Id |
|---|---|---|
| Background colour picker | — | `slide-bg` |
| ⌀ (remove background colour) | — | `slide-bg` (empty value removes it) |
| Image… | — | `slide-bgimg` |
| Transition select | — | `transition-default`, `transition-none`, `transition-fade`, `transition-slide`, `transition-convex`, `transition-concave`, `transition-zoom` |
| Hidden checkbox | — | `slide-hide` |
| ⋯ (more) | — | `actions-menu` |

## Element bar and element right-click menu (actions.js)

| Control | Key before | Id |
|---|---|---|
| Run cell | — | `run-cell` |
| Edit text | Enter | `edit` |
| Edit markdown (was Edit source) | ⌥Enter | `source` |
| Cut | ⌘X | `cut` |
| Copy | ⌘C | `copy` |
| Paste after | ⌘V | `paste` |
| Duplicate | ⌘D | `dup` |
| Delete | ⌫ / Delete | `delete` |
| Move up | ⌥↑ | `up` |
| Move down | ⌥↓ | `down` |
| Insert before ▸ | — | `insert-paragraph`, `insert-list`, `insert-equation`, `insert-image`, `insert-textbox` (insert after the selection, or at the end) |
| Insert after ▸ | — | same as above |
| Appears on click | — | `fragment` |
| Appearance ▸ Smaller text | — | `class-smaller` |
| Appearance ▸ Incremental list | — | `class-incremental` |
| Appearance ▸ Custom class… | — | `class-custom` |

## Slide and empty-area right-click menu (actions.js)

| Control | Key before | Id |
|---|---|---|
| Add slide before | — | `slide-before` |
| Add slide after | ⇧⌘N | `slide-after` |
| Duplicate slide | ⇧⌘D | `slide-dup` |
| Cut slide (slide list) | ⌘X | `slide-cut` |
| Copy slide (slide list) | ⌘C | `slide-copy` |
| Paste slides after | ⌘V | `slide-paste` |
| Hidden in the presentation | — | `slide-hide` |
| Delete slide… | — | `slide-rm` |
| Background colour… | — | `slide-bg` |
| Background image… | — | `slide-bgimg` |
| Transition ▸ | — | `transition-*` (as above) |
| Speaker notes… | — | `slide-notes` |
| Edit slide source | — | `slide-source` |
| Copy link to slide | — | `slide-link` |
| Insert ▸ (empty area) | — | `insert-*` |

## Keys with no control

| Key before | Where | Id |
|---|---|---|
| Tab / ⇧Tab | slide | `next-element`, `prev-element` |
| ⇧F10, ContextMenu | slide, element | `actions-menu` |
| Esc | element selected | `deselect` |
| Esc | slide, in VS Code | `focus-text` |

## Formatting toolbar while typing (toolbar.js)

| Control | Key before | Id |
|---|---|---|
| Bold | ⌘B | `bold` |
| Italic | ⌘I | `italic` |
| Underline | ⌘U | `underline` |
| Strikethrough | ⇧⌘X | `strike` |
| Inline code | ⌘E | `code` |
| Link | ⌘K | `link` |
| Bulleted list | ⇧⌘8 | `ul` |
| Numbered list | ⇧⌘7 | `ol` |
| Outdent | ⇧Tab in a list | `outdent` |
| Indent | Tab in a list | `indent` |
| Highlight | — | `highlight` |
| Remove text colour | — | `no-color` |
| Smaller text | — | `smaller` |
| Larger text | — | `larger` |
| Align left / Centre / Align right | — | `align-left`, `align-center`, `align-right` |
| Text colour swatches | — | *exempt*: a palette of values, like the background colour picker; it stays in the toolbar |

## Widget mechanics (exempt; on the allowlist in `tests/actions.rs`)

These keys operate a control from inside it. They are not actions in their own right.

| Where | Keys | Why exempt |
|---|---|---|
| menu.js | arrows, Enter, Space, Esc, Tab inside an open menu | navigating the menu itself |
| editor.js splitter | ← → | resizing the focused splitter |
| editor.js form | ⌘Enter | submitting the focused form |
| editor.js `sessionKey` | Enter/⌘Enter/Tab save, Esc ends, ⌥Enter to source, Tab/⇧Tab indent in a list | committing or leaving the edit in progress; moves into the list with US4 (T020), which changes Esc |
| source.js | Esc, ⌘Enter in the source popup | the same, for the source popup; moves with T020 |


Labels renamed in plain words by spec 008 US6 (T025); the ids are unchanged.

Added by spec 008 US7 (T030), Insert menu: Code cell `insert-code`, Callout `insert-callout`, Columns `insert-columns`, Speaker notes `insert-notes`, Table `insert-table`, Quote `insert-quote`.

## Added by 009 (layouts and placement)

| Control | Key | Id |
|---|---|---|
| Place freely (or ⌥-drag) | — | `place` |
| Put back in the flow | — | `unplace` |
| Bring to front | ⌥⇧⌘F | `box-front` |
| Bring forward | ⌥⌘F | `box-forward` |
| Send backward | ⌥⌘B | `box-backward` |
| Send to back | ⌥⇧⌘B | `box-back` |
| Nudge left, right, up, down | arrows (⇧ by 10) | `box-nudge-left` `box-nudge-right` `box-nudge-up` `box-nudge-down` |
| Save slide as layout… | — | `layout-save` |
| Snap to guides | — | `snap` |
| Slide after… / before… (the gallery) | ⇧⌘N | `slide-after` `slide-before` |

## Added by 011 (export formats)

The header's **Export ▾** shows File ▸ Export's items. `export` and
`export-pdf` keep their ids (they moved into the submenu).

| Control | Key | Id |
|---|---|---|
| PowerPoint… | — | `export-pptx` |
| Beamer PDF… | — | `export-beamer` |
| Word… | — | `export-docx` |
| HTML page… | — | `export-html` |
| PDF (LaTeX)… | — | `export-latex` |
| PDF (Typst)… | — | `export-typst` |
| Markdown (GitHub)… | — | `export-gfm` |
| Markdown (CommonMark)… | — | `export-commonmark` |
| Jupyter notebook… | — | `export-ipynb` |
| OpenDocument… | — | `export-odt` |
| EPUB… | — | `export-epub` |
| Other format… | — | `export-other` |
