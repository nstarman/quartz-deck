# CLI Contract additions: 002-offline-export

Conventions as in [001 contracts/cli.md](../../001-core-editor/contracts/cli.md)
(JSON on stdout, JSON errors on stderr, stable codes, exit 1/2).

| Command | Output |
|---|---|
| `qd check --offline [--html FILE] [--no-download]` | `CheckReport` (data-model.md) with `"kind": "offline_check"`; `html` is the checked file (the `.qmd` when qd rendered it). Without `--html`, renders the deck as `qd render` would (to a temp folder) and checks that. Exit 1 with `E_OFFLINE` when `ok` is false (report still on stdout). |
| `qd export [--out DIR] [--pdf] [--notes] [--per-slide] [--no-download]` | `ExportResult`. Exit 1 with `E_OFFLINE` if the check fails (previous export kept; staging path in `details`), `E_OFFLINE_INCOMPATIBLE` if a feature cannot work offline. |
| `qd pdf [--html FILE] [--out FILE] [--notes] [--per-slide] [--no-download]` | `PdfResult`. Default `--html`: the deck's export HTML; error `E_NOT_FOUND` with hint `qd export` if none. |
| `qd browser [--install \| --remove] [--no-download]` | `{"path","source","version"}`; `--install` downloads if none is found; `--remove` deletes qd's cached browser. |

New error codes: `E_NO_BROWSER` (none found and downloads forbidden),
`E_DOWNLOAD` (network/HTTP failure or checksum mismatch — nothing is used),
`E_OFFLINE` (check failed), `E_OFFLINE_INCOMPATIBLE`.

Problem codes (inside reports): `P_NETWORK`, `P_MISSING_FILE`, `P_PAGE_ERROR`,
`P_RAW_TEX`, `P_OFFLINE_INCOMPATIBLE`, `P_NOTES_VISIBLE`.

Environment: `QD_CHROME` (browser path), `QD_NO_DOWNLOAD=1`.
