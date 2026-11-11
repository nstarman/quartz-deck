# Data model: Usability (008)

Nothing new is stored in the deck. Two editor-side things and one file
location are new.

## Thumbnail (editor only)

| Field | Meaning |
|---|---|
| slide | the slide's id (or `title-slide`) |
| version | the render version it was copied from |
| source hash | the slide's source hash when copied; a changed hash rebuilds it |
| node | a shadow root holding a copy of the slide's `<section>`, scaled to the list's width |

Rules: built only for items in view; rebuilt when a render lands and the
slide's source hash changed; the deck's stylesheets are shared by all
thumbnails (one constructed stylesheet per render).

## Sheet (editor only)

| Field | Meaning |
|---|---|
| title | what is being asked ("Link", "Background colour", …) |
| fields | each `{name, kind: text\|swatches\|files\|check, value, validate}` |
| anchor | the control or element it opens beside |
| result | a promise of the values, or `null` on Cancel/Escape |

Rules: `validate(value)` returns a message or nothing; OK stays disabled while
any field has a message; Escape cancels; Enter confirms.

## Image beside the deck

| Field | Meaning |
|---|---|
| path | relative to the deck (`images/chart.png`, or wherever it already is) |
| kind | png, jpg, jpeg, gif, svg, webp (by extension and magic bytes) |
| size | bytes (an upload is refused above 20 MB) |
| modified | for "newest first" in the picker |

Rules: uploads go to `images/` beside the deck, made if missing; the name is
the dropped file's base name reduced to `[A-Za-z0-9._-]`; a taken name gets
`-1`, `-2`, …; nothing is ever overwritten. Listing walks at most 3 levels
and skips hidden folders, qd's cache and the export folder.

## Slide selection (editor only, P3)

A set of slide ids with an anchor for ⇧-ranges. Actions on the list apply to
the set; one op each (`qd move a b c --after x`, `qd rm a b c`), one undo
step.
