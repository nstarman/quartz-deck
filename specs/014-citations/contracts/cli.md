# CLI contract: `qd bib` (014)

All commands take the global `--deck`, `--expect`, `--dry-run` and `--human`
flags. Output is JSON on stdout. Errors go to stderr as `{"ok":false,
"code","message","hint"}` with a non-zero exit (Principle III). Every write
also returns `version` and, when Save as git commit is on, `commit`, as
other edits do.

## `qd bib`

Lists the deck's references. It is read-only.

```json
{"ok": true, "kind": "bib", "where": "companion|embedded|both|none",
 "files": [{"path": "_talk/references.bib", "format": "bib|json", "inside": true}],
 "embedded": 0, "entries": 15,
 "missing": [{"key": "foo2020", "slides": ["intro/2"]}],
 "extensions": [{"name": "cite-links", "deck": "1.0.0", "qd": "1.0.0", "state": "same"}]}
```

## `qd bib pull FILE`

FILE is a `.bib` (BibTeX or BibLaTeX) or a `.json` (CSL-JSON). It may be
outside the deck's folder; reading from anywhere is allowed.

```json
{"ok": true, "op": "bib-pull", "version": "…", "file": "_talk/references.bib", "created": false,
 "added": ["bonaca2020"], "changed": [{"id": "koposov2010", "fields": ["DOI"]}],
 "merged": [], "gone": [], "warnings": ["…"], "installed": ["cite-links"], "dry_run"?: true}
```

## `qd bib pull --from NAME [ARGS…]`

Runs `qd-NAME ARGS…` from `PATH`. The output and the merge are the same as
`pull FILE`, with `"source": "zotero"` added.

| Error | When |
|---|---|
| `E_SOURCE` | `qd-NAME` isn't on `PATH` (hint: how to install it); it exited non-zero (message: its stderr); its output isn't a JSON array of objects |

## `qd bib embed`

Moves the companion file's entries into `references:` and removes
`bibliography:`. The companion file is left on disk, and the result says
so (`"left": "_talk/references.bib"`). Deleting it is the person's choice.

## `qd bib extract [--to FILE] [--format bib|json]`

Writes the embedded entries to FILE (default: the default place in data-model.md, as `.bib`),
sets `bibliography:` and removes `references:`. If FILE exists, the entries
are merged into it.

## `qd bib convert --format bib|json`

Rewrites the companion file in the other format under the same stem
(`references.bib` ↔ `references.json`), updates `bibliography:`, and removes
the old file. It is recorded in the undo entry's `also`.

## `qd bib install`

Writes or updates `_extensions/cite-links` and `_extensions/refs-slide`
(R9). It isn't an edit of the deck, so it has no `version`, and `--expect`
is ignored.

```json
{"ok": true, "kind": "bib-install", "installed": ["cite-links"], "kept": [{"name": "refs-slide", "state": "edited", "code": "W_EDITED"}]}
```

## `qd bib migrate [--from zotero KEY] [--backup DIR] [--accept]`

Converts a `paper` deck (R11).

```json
{"ok": true, "op": "bib-migrate", "version": "…", "accepted": false,
 "differences": 0, "report": "<cache>/migrate/…/report.json", "backup": "…",
 "entries": {"from_source": 15, "from_shortcodes": 0}, "citations": {"inline": 11, "corner": 24}}
```

| Error | When |
|---|---|
| `E_INVALID_ARG` | not a git folder and no `--backup`; a shortcode with an unknown argument (names the slide); no `paper` shortcodes in the deck |
| `E_MIGRATE_DIFF` | the renders differ and there is no `--accept`. The message gives the count, `report` gives the path, and every file has been restored |
| `E_RENDER` | a render failed. Everything is restored |
| `E_SOURCE`, `E_ZOTERO` | as for pull. Nothing has been written yet |

## Errors shared by the writes

| Code | When |
|---|---|
| `E_READ_ONLY` | a write would touch a bibliography file outside the deck's folder. Hint: "copy it into the deck's folder, or pull from it: `qd bib pull PATH`" |
| `E_AMBIGUOUS` | several companion files inside the deck's folder |
| `E_CONFLICT` | `--expect` is stale, or the deck or companion file changed during the command |
| `E_NO_QUARTO` | a format change needs `quarto pandoc` and Quarto isn't found |

## Removed (last PR)

`qd bib --zotero KEY` and the marker list it wrote. `qd bib migrate` reads
the Makefile's old `qd bib --zotero KEY` line, so old decks still convert.

## `qd-zotero COLLECTION` (source plugin)

- Prints a CSL-JSON array on stdout and exits 0.
- **A bad key**: exits 2 with `E_INVALID_ARG …` on stderr, without
  connecting.
- **Zotero not reachable**: exits 2 with `E_ZOTERO …` and the hint.
- **The address**: `QD_ZOTERO_URL` replaces `http://localhost:23119`.
