# Quickstart: Arrange elements inside columns

```markdown
## R {#r}

:::: {.columns}
::: {.column}
- one

Two.
:::
::: {.column}
![A plot](p.png)
:::
::::

Below.
```

```bash
qd -d t.qmd block move r/0/0/1 --before r/0/0/0    # Two. above the list
qd -d t.qmd block move r/0/1/0 --after r/0/0/1      # the picture into the left column
qd -d t.qmd block move r/0/0/2 --into r/0/1         # and back into the (now empty) right one
qd -d t.qmd block move r/0/0/0 --after r/1          # the list out, below "Below."
qd -d t.qmd block dup r/0/0/0                       # copy in the column, fresh ids
qd -d t.qmd block place r/0/1/0 --at 600,400 --width 300   # out of the column as a box
```

Editor: on the slide, ⌥↑ inside a column; drag the picture to the other column and below the columns; empty a column and use "+ Add content".

```bash
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```
