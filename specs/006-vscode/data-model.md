# Data Model: VS Code Integration

## Buffer overlay

| Item | Where | Rule |
|---|---|---|
| overlay text | `<cache>/buffer.qmd` (qd's user-only cache dir) | exists only while a VS Code panel is attached; `Store` reads/writes it instead of the `.qmd` |
| attached flag | `session.json` `"buffer": true` | set by `POST /api/buffer/open`, cleared on close / server stop |
| version | `version_of(overlay text)` | the version every op, `--expect` and `qd focus` refer to while attached |

Lifecycle: `open(text)` → overlay = buffer text → edits flow both ways →
`close` (panel closed or `qd stop`) → overlay removed; the `.qmd` is whatever
VS Code last saved.

## Buffer edit (qd → VS Code)

`{base, version, edits: [{start, end, text}]}` — byte offsets into the overlay
text at `base`, applied in order; `version` is the overlay after them. The
extension applies them only if its last-known overlay version is `base`
(otherwise it re-syncs from the full text).

## Buffer update (VS Code → qd)

`{base, text}` — the whole buffer; written iff the overlay is still at `base`,
else 409 `{version, text}` and the extension merges (three-way: base, buffer,
overlay).

## Messages (webview ↔ editor iframe)

| Direction | Type | Payload | Effect |
|---|---|---|---|
| extension → editor | `follow` | `{line}` | show the slide containing that source line (not a focus change by the person) |
| extension → editor | `theme` | `{kind: "light"\|"dark"\|"hc"}` | switch theme |
| editor → extension | `reveal` | `{lines: [first, last]}` | reveal that range in the text editor, no focus steal |
| editor → extension | `ready` | `{}` | the editor loaded (restore cursor follow) |

Focus (spec 005) gains `where: "text"` for the VS Code text editor.
