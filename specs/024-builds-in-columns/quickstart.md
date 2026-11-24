# Quickstart: Builds inside columns

## CLI (US2)

A deck `t.qmd`:

```markdown
## Results {#r}

:::: {.columns}
::: {.column}
- one
- two
:::
::: {.column}
![A plot](plot.png)
:::
::::
```

```bash
qd -d t.qmd show r                                   # r/0 has children r/0/0, r/0/1 with r/0/0/0, r/0/1/0
qd -d t.qmd build set r/0/0/0 --appear fade-in
qd -d t.qmd build set r/0/1/0 --appear fade-up
qd -d t.qmd build list r                             # r/0/0/0 then r/0/1/0
qd -d t.qmd build order r r/0/1/0 r/0/0/0            # fragment-index 1 on the picture, 2 on the list
qd -d t.qmd block rm r/0/0/0                         # E_INVALID_ARG: inside columns
```

Expected: only the list's and the picture's lines change; the list is
wrapped in `::: {.fragment ...}` inside its column.

## Editor (US1)

1. `qd -d t.qmd serve`, go to Results, click the list: only the list is outlined.
2. Animate view → Appear: Fade in. Click the picture → Fade up.
3. Drag the picture above the list in Builds.
4. Escape: the column is selected; again: the columns block; again: nothing.
5. Present: click 1 shows the picture, click 2 the list.

## Showcase (US3)

Open `examples/showcase/showcase.qmd` on `qd-showcase` (port 4880) and step
through the "Builds inside columns" slide.

## Checks

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```
