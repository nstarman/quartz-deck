# UI contract: actions (spec 003)

One action table drives the context menus, the toolbars and the keyboard
(research R6). Every action that changes the deck maps to a `qd` command
(FR-014). ⌘ = Ctrl on Windows/Linux.

## Element (right-click an element on the slide)

| Action | Shortcut | `qd` equivalent | Disabled when |
|---|---|---|---|
| Edit | Enter | `edit <el>` | element not typeable (then "Edit source" is the default) |
| Edit source | ⌥Enter | `edit <el>` | — |
| Cut | ⌘X | copy + `block rm <el>` | — |
| Copy | ⌘C | (clipboard) | — |
| Paste after | ⌘V | `block add <slide> --after <el>` | clipboard empty |
| Duplicate | ⌘D | `block dup <el>` | — |
| Delete | ⌫ | `block rm <el>` | — |
| Move up / down | ⌥↑ / ⌥↓ | `block move <el> --before/--after <prev/next>` | first / last element |
| Insert before / after ▸ paragraph, list, image, text box, equation | — | `block add …` / `box add …` | — |
| Appears on click | — | `edit` adding/removing `.fragment` | element can't take attributes (then wraps in a div) |
| Appearance ▸ smaller, incremental, absolute, custom class… | — | `edit` (class change) | — |

## Slide (right-click a slide in the list, or empty slide area)

| Action | Shortcut | `qd` equivalent |
|---|---|---|
| Add slide before / after | ⌘⇧N (after) | `add --before/--after` |
| Duplicate slide | ⌘⇧D | `dup` |
| Hide / Show | — | `set --attr visibility=hidden` / `set --unset visibility` |
| Delete slide | — | `rm` |
| Background… | — | `set --attr background-color=… / background-image=…` |
| Transition ▸ none, fade, slide, convex, concave, zoom | — | `set --attr transition=…` |
| Speaker notes… | — | `edit`/`block add` of the `::: notes` block |
| Edit slide source | — | `body` / slide source edit |
| Copy link to slide | — | (clipboard: `qd url <slide>`) |

## Formatting toolbar (while typing)

| Button | Shortcut | Markdown written |
|---|---|---|
| Bold | ⌘B | `**…**` |
| Italic | ⌘I | `*…*` |
| Underline | ⌘U | `[…]{.underline}` |
| Strikethrough | ⌘⇧X | `~~…~~` |
| Inline code | ⌘E | `` `…` `` |
| Link | ⌘K | `[…](url)` |
| Bulleted / numbered list | ⌘⇧8 / ⌘⇧7 | `- ` / `1. ` |
| Indent / outdent | Tab / ⇧Tab (in lists) | nested list indentation |
| Text colour | — | `[…]{style="color:#rrggbb"}` |
| Highlight | — | `[…]{.mark}` |
| Align left / centre / right | — | `::: {style="text-align:…"}` around the block |
| Size smaller / larger | — | `[…]{style="font-size:0.8em"}` steps 0.6–2.0em |

Keys while editing: Escape cancels, ⌘Enter or Tab or clicking elsewhere
saves. Context menus: arrows, Enter, Escape, Shift+F10 / ContextMenu key.
