# Data Model: 001-core-editor

All types live in memory only; the `.qmd` text is the persistent form.
`Span` = byte range `[start, end)` into the file.

## Deck

| Field | Type | Notes |
|---|---|---|
| path | path | canonical absolute path |
| version | string(16) | SHA-256 prefix of file bytes (R7) |
| front_matter | Span? | `---\n…\n---` YAML at file start |
| slide_level | u8 | from front matter, default 2 |
| width, height | u32 | from front matter, default 1050×700 |
| slides | [Slide] | in file order; spans tile the body |

**Invariant**: concatenating front matter + all slide spans == file bytes.

## Slide

| Field | Type | Notes |
|---|---|---|
| id | string | explicit `{#id}`, auto-id, or `@N` (R3) |
| id_explicit | bool | |
| index | u32 | 1-based position (title slide from YAML is 0 if present) |
| kind | enum | `heading` (level n) \| `rule` (`---`) \| `section` (level 1) |
| title | string? | heading text, attributes stripped |
| attrs | Attrs | heading attribute block |
| span | Span | heading/rule line through end of slide |
| blocks | [Block] | top-level blocks after the heading |
| opaque | bool | segmentation unsure → structural edits refused |

## Block (Element)

| Field | Type | Notes |
|---|---|---|
| id | string | explicit `#id` or `<slide>/<i>` |
| kind | enum | `para`, `list`, `code`, `cell`, `div`, `image`, `table`, `quote`, `html`, `math`, `pause`, `notes`, `box` |
| attrs | Attrs? | for divs, code, images with `{…}` |
| span | Span | |
| text | string | source text (for show/edit) |

`box` = `div` or `image` with class `absolute`; exposes `geom`.

## Attrs

`{ id: string?, classes: [string], kv: [(string, string)] }` — order
preserved; printing back an unmodified Attrs yields the original text.

## Geom (box)

`{ top, left, width, height, bottom, right }` each `Length?`, where
`Length = { value: f64, unit: "px"|"%"|"em"|… }`; unitless parses as px.

## Edit / Patch

| Field | Type | Notes |
|---|---|---|
| op | string | command name, e.g. `slide.move` |
| replacements | [(Span, string)] | non-overlapping, applied right-to-left |
| version_before / version_after | string | |
| diff | string | unified diff (dry-run and JSON result) |
| assigned_ids | [string] | IDs written as a side effect (R3) |

## Undo journal entry

`{ ts, op, version_before, version_after, before_text }`, appended JSONL in
the cache dir. State transition: `undo` valid only when file version ==
last entry's `version_after`; restoring pops the entry.

## Editor session

`{ deck_path, url, port, render_state: idle|rendering|ok|error, last_error? }`
Registered in `<cache>/session.json` so `qd url` can find a running server.
