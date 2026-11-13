# Data model: Citations (014)

There is no database. Everything lives in the deck, its companion file or
qd's cache.

## Reference entry

A CSL-JSON object (`serde_json::Value`), as Pandoc and Zotero write it.

| Field | Use |
|---|---|
| `id` | **The key.** It is cited as `@id`. Required; it is unique within the deck's references. |
| `type` | The CSL type (`article-journal`, `software`, …). |
| `author` | `[{family, given}]` or `[{literal}]`. The first author gives the generated key. |
| `issued` | `{"date-parts": [[2020]]}`. The year gives the generated key, or `nd` without one. |
| `title` | Matching (third) and hover text. |
| `DOI` | The link (`https://doi.org/…`). Matching (first). An arXiv DOI (10.48550) yields to `URL`. |
| `URL` | The link when there is no DOI, or for an arXiv DOI. Matching (second). |
| others | Kept as given. A pull overwrites the fields its source provides, and keeps the rest. |

**Validation**:
- `id` matches Pandoc's citation-key rule: it starts with a letter, digit or
  `_`, and continues with letters, digits and `:.#$%&-+?<>~/`. Generated
  keys use `[a-z]+[0-9]{4}[a-z]?`.
- Two entries with the same `id`: the later entry is renamed with a
  suffix, and a warning is given.

## Deck references

Where a deck's entries live. It is derived from the front matter, never
stored separately.

| State | Front matter | Writable by qd |
|---|---|---|
| none | neither key | yes, and the first pull creates the companion file |
| companion, inside | `bibliography: _talk/references.bib` (or `.json`), the path resolved inside the deck's folder | yes |
| companion, outside | `bibliography:` resolving outside the deck's folder | **no** (`E_READ_ONLY`): only read, cited, and pulled from |
| embedded | `references: [...]` (CSL YAML) | yes |
| both | both keys | yes. `qd bib` reports both, and embed or extract merges them |

`bibliography:` may be a list. qd writes only to the single file inside the
deck's folder. If there are several inside, `qd bib` refuses to write with
`E_AMBIGUOUS` and names them.

**Default place** (FR-013): `_talk/references.bib` when `_talk/` exists,
else `references.bib` beside the deck.

**The companion file's header** (BibTeX `%` comment, or none for JSON): "%
Written by qd bib. Comments and @string macros are not kept when qd rewrites
this file." JSON has no comments, so `qd bib` notes the same thing in its
output the first time it writes a JSON file.

## Merge result

`bib::merge(existing, incoming) -> Merge`:

| Field | Meaning |
|---|---|
| `added` | `[id]`, new entries, with the key their source gave or a generated one |
| `changed` | `[{id, fields: [name]}]`, matched entries whose fields changed |
| `merged` | `[{id, from: [id]}]`, duplicates in the incoming data folded into one entry |
| `gone` | `[id]`, deck entries the source didn't return (kept, and a warning given) |
| `entries` | the resulting list, in the existing order with new entries appended |

## Undo journal entry (store.rs, extended)

Today's `Entry` gains `also: Vec<OtherFile>` (`#[serde(default)]`, so older
journals still load).

| `OtherFile` field | Meaning |
|---|---|
| `path` | the absolute path of the companion file |
| `before` | its text before the edit (`None` if the edit created it) |
| `after` | its text after the edit (`None` if the edit removed it) |

Undo and redo check every file's current content against `after` (or
`before`) and refuse with `E_NO_UNDO` on a mismatch. They then write all of
them, the deck last.

## Plugin extension (compiled into qd)

| Field | Meaning |
|---|---|
| `name` | `cite-links` or `refs-slide` |
| `version` | from its `_extension.yml` |
| `files` | `[(relative path, bytes)]`, through `include_str!` |

Install states, compared with the deck's `_extensions/<name>/_extension.yml`:
`absent` or `older` → write qd's files; `same` with identical bytes → nothing;
`same` with different bytes → `W_EDITED`, kept; `newer` → kept.

## Source plugin manifest (`plugins/<name>/qd-plugin.json`)

```json
{ "source": { "name": "zotero", "command": "qd-zotero", "args": "COLLECTION" } }
```

It is documentation and a test input (a test checks that `command` is the
binary the crate builds). qd doesn't read it at run time.

## Paper shortcode (for migrate)

`{{< paper url=… who=… year=… title=… [corner="true"] >}}`. Any other
argument is an error. Its fields map to an entry when nothing matches:
- `URL` (or `DOI`, when the URL is `https://doi.org/…`);
- `author: [{literal: who}]`;
- `issued`: the year without its a/b suffix;
- `title`.

## Migrate report

```json
{ "deck": "…/presentation.qmd", "backup": "…", "accepted": false,
  "entries": {"from_source": 15, "from_shortcodes": 5, "unmatched": [{"slide": "…", "url": "…", "who": "…"}]},
  "citations": {"inline": 11, "corner": 24},
  "differences": [
    {"slide": "results/2", "kind": "text|href|title|missing|extra",
     "before": {"href": "…", "text": "Myers, Starkman & Necib (2025)", "title": "…"},
     "after":  {"href": "…", "text": "Myers et al. (2026a)", "title": "…"}}
  ] }
```

The report is saved in qd's cache, and its path is in the command's JSON as
`report`. A slide is identified by its qd slide id, else by its 1-based
index.
