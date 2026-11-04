# Data Model: On-Slide Editing

No new persisted state in the deck. Additions are derived values and
browser-side session state.

## Block (existing, extended)

| Field | Type | Notes |
|---|---|---|
| id | string | `<slide>/<index>` or explicit `#id` (unchanged) |
| kind | enum | para, heading, list, quote, box, code, math, div, notes, raw, other (unchanged) |
| text | string | exact source span (unchanged) |
| **hash** | 16 hex | SHA-256 of `text`, first 16 hex; used by `--expect-text` (new) |

`qd show <slide>` and `/api/slide/<ref>` report `hash` for every block.

## Edit session (browser only)

| Field | Type | Notes |
|---|---|---|
| element | block id | the element being edited |
| mode | `typed` \| `source` | `typed` only if the round-trip gate passed |
| base_hash | 16 hex | block hash when the session began |
| base_source | string | source when the session began |
| draft | string | current markdown (serialized DOM or source text) |
| state | open → saved \| cancelled \| conflict | conflict keeps `draft` for re-apply |
| pending_render | path? | newest render queued while open (R4) |

Transitions: `open` → `saved` on blur/Tab/⌘Enter when the op succeeds;
`open` → `conflict` when the op returns `E_CONFLICT` (draft kept, current
source shown); `open` → `cancelled` on Escape. Leaving any state applies
`pending_render`.

## Action (browser only; one table, see contracts/ui.md)

| Field | Type | Notes |
|---|---|---|
| id | string | e.g. `el.delete`, `slide.hide` |
| label | string | menu text |
| shortcut | string? | shown in menus and toolbar tooltips |
| targets | set | element kinds / slide / slide-list / empty-area it applies to |
| enabled(target) | bool | disabled items shown greyed (FR-020) |
| command | qd op + args, or local | the CLI equivalent (FR-014) |

## Validation rules

- Saving `typed` drafts: serializer output; server trims to the differing
  middle (minimal patch) before writing.
- A `source` draft that adds/removes a slide heading or leaves an unbalanced
  fence is refused (`E_INVALID_ARG`, FR-008); the draft is kept.
- `block move` within one slide only; moving across slides = cut + paste.
