# CLI contract: Run on the slide (013)

Extends 004's `qd run` (`specs/004-source-and-run/contracts/cli.md`).

## `qd run`

- Cells run with the engine Quarto uses for the deck (research R2):
  - a Jupyter kernel: named by `jupyter:`; else, for a Python deck with an
    environment, qd's `qd-python` (as 004); else the first installed kernel
    for the deck's language;
  - an R session for knitr decks;
  - later, a Julia session for `engine: julia`.
- The result gains `"engine": {"kind": "jupyter|knitr|julia", "kernel"?, "label"}`.
- `cell_code` accepts any executable cell (`{python}`, `{r}`, `{julia}`, …),
  not only Python. A cell in a language the deck's engine can't run (say an
  `{r}` cell in a Jupyter deck) is `E_ENGINE`, as in Quarto.

## Errors

| Case | Code | Hint |
|---|---|---|
| the deck names a kernel that isn't installed | `E_ENGINE` | Quarto's message, with the kernels found |
| knitr deck, no R | `E_ENGINE` | install R (https://cran.r-project.org) |
| knitr deck, R without knitr/rmarkdown | `E_ENGINE` | `install.packages(c("knitr", "rmarkdown"))` |
| a non-Python Jupyter deck, and uv missing | `E_ENV_TOOL` | install uv (as 004) |
| `{ojs}` cell from the CLI | `E_ENGINE` | OJS cells run in the editor's page (`qd serve`), not from the CLI |

`E_ENGINE`'s message is "cannot run {lang} cells: {what is missing}".
