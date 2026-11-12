# CLI contract: `qd export` (011 additions)

```text
qd export [--out PATH] [--pdf [--notes] [--per-slide]] [--no-download]   # unchanged (002/010)
qd export --to FORMAT [--out PATH] [--dry-run]
qd export --list
```

| Flag | Meaning |
|---|---|
| `--to FORMAT` | any name from `qd export --list`. `revealjs` is the same as no `--to`, and `revealjs-pdf` the same as `--pdf` |
| `--out PATH` | output file for `--to` (default `export/<deck>.<ext>` next to the deck); for plain export it stays the destination folder |
| `--list` | print the format list (data-model.md) and exit; conflicts with `--to` |
| `--dry-run` (global) | with `--to`: check the tool and print the result with losses, without rendering |

`--pdf`, `--notes` and `--per-slide` conflict with `--to` (other than
`revealjs`/`revealjs-pdf`).

## Exit and errors

| Case | Code | Hint |
|---|---|---|
| unknown format | `E_INVALID_ARG` | `see qd export --list` |
| TeX missing for `pdf`/`beamer` | `E_NO_TEX` | `install TeX: quarto install tinytex` |
| Quarto not found | `E_NO_QUARTO` (existing) | existing |
| a cell fails | `E_RENDER` (existing), with `slide` | existing |
| output can't be moved into place | `E_IO`, message names the temp file kept | `close the file and try again` |

Every case leaves the deck file unchanged, and no file at `--out` unless
the export succeeded.

## `qd schema`

`export` gains `"--to"`, `"--list"` and the result shapes `export-format`
and `export-formats`.
