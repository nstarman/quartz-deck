# Data model: App shell (007)

## Action (`src/ui/actions.json`, one entry per action)

| Field | Type | Rule |
|---|---|---|
| `id` | string | unique, kebab-case; the handler key in the editor |
| `label` | string | menu text; ends in "…" when it opens a dialog |
| `menu` | string[] | path, e.g. `["Slide", "Transition"]`; order in the file is menu order; `"-"` entries are separators |
| `key` | string or string[] | like `"Mod+Shift+N"`; `Mod` is ⌘ on macOS, Ctrl elsewhere |
| `vscode_key` | string, string[] or null | replaces `key` in VS Code's panel; `null` = menu-only there |
| `where` | string[] | subset of `anywhere`, `slide`, `element`, `text`, `source`, `slides` (the slide list) |
| `toggle` | bool | shown as a checkbox item |
| `runs_code` | bool | only on the person's own click/key (spec 004); never bound to a key that fires while typing |
| `qd` | string? | the matching CLI command, e.g. `"qd dup <slide>"` |

Validation (Rust test over the embedded file): ids unique; every key parses;
no two actions share a key in overlapping `where`; no effective panel key
is in `vscode/default-keys.json`; every `runs_code` action has `where` ≠
`text`.

## Setting (definitions in `src/settings.rs`)

| Key | Values | Default |
|---|---|---|
| `source.side` | `left`, `right` | `left` |
| `source.open` | `last`, `open`, `closed` | `last` |
| `source.save` | `pause`, `explicit` | `pause` |
| `theme` | `system`, `light`, `dark`, `hc` | `system` |
| `confirm.slide_delete` | bool | `true` |
| `hints.shortcuts` | bool | `true` |
| `ui.*` (not shown) | layout memory: `ui.source_width` (px), `ui.source_was_open`, `ui.source_full`, `ui.form_panel` | absent |

Each definition: key, label, description, type, allowed values, default.
Later features append definitions (011: `history.commit_on_save`).

## Settings file (`<config>/settings.json`)

A flat JSON object of key → value. Mode 0600, written atomically.
- **Invalid or unknown values:** an invalid value is reported once and read
  as the default. Unknown keys are kept as they are.
- **Reset:** writes `{}`, which keeps unknown keys.

## Clipboard item (editor memory)

`{kind: "text" | "element" | "slide", markdown, from_deck}`. The system
clipboard gets `markdown` as text/plain. The internal copy is used when the
system clipboard can't be read.
