# Data Model: 002-offline-export

All in memory or in output JSON; nothing is stored in the deck.

## ExportOptions

| Field | Type | Default | Notes |
|---|---|---|---|
| out | path | `<deck dir>/export/` | destination folder (replaced on success) |
| pdf | bool | false | also print a PDF |
| notes | bool | false | speaker notes on separate PDF pages |
| per_slide | bool | false | one PDF page per slide instead of per fragment step |
| allow_download | bool | true | may fetch libraries/browser (`--no-download`) |

## ExportResult (JSON)

`{ ok, out, html, files: [rel path], pdf?: PdfResult, check: CheckReport, assets: [Asset] }`

- `files`: non-inlinable files copied beside the HTML (relative paths).
- `assets`: `{ name, version, sha256, source: cache|download|python }`.

## CheckReport

`{ ok, html, slides, steps, problems: [Problem] }` — `ok` is false iff any
problem has severity `error`.

## Problem

| Field | Type | Notes |
|---|---|---|
| code | enum | `P_NETWORK`, `P_MISSING_FILE`, `P_PAGE_ERROR`, `P_RAW_TEX`, `P_OFFLINE_INCOMPATIBLE`, `P_NOTES_VISIBLE` |
| severity | `error` \| `warning` | `P_NOTES_VISIBLE` is a warning; the rest are errors |
| slide | string? | preview anchor (as in `qd slides`), `null` for whole-deck problems (e.g. requests during load) |
| message | string | |
| hint | string | how to fix |
| url | string? | for `P_NETWORK`, `P_MISSING_FILE` |

## PdfResult

`{ path, pages, per_slide, notes }`

## Browser

`{ path, source: env|installed|download, version? }` — from `qd browser`.

## Asset (pinned)

`{ name: "mathjax"|"plotly"|"chrome", version, url?, sha256 }` — MathJax and the
browser are pinned in code; plotly.js is taken from the user's Python plotly
and reported with its version and hash.
