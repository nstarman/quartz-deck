# refs-slide

A Quarto filter extension: the References slide in the style, columns and
size the presenter picks on its `#refs` div. Nothing resizes the slide
while it is shown.

```markdown
## References

::: {#refs refs-style="compact" refs-columns="3" refs-size="smaller"}
:::
```

| Attribute | Values | Default |
|---|---|---|
| `refs-style` | `compact`: "icon Name et al. (2021)" links, the same labels as the citations, suffixes included; `short`: "Authors (year), *Venue*", linked; `full`: the deck's citation style, as citeproc writes it | `compact` |
| `refs-columns` | `1`, `2`, `3` | `3` |
| `refs-size` | `normal`, `smaller`, `smallest` | `smaller` |

Quarto already makes the slide holding `#refs` `smaller scrollable`, so a
long list scrolls, and the sizes are relative to that already smaller
text.

Each entry links to its DOI, else its URL. An arXiv DOI (10.48550) links
the entry's arXiv page. An entry with neither is listed without a link.

Use it with `cite-links`, listed first, so the labels follow `talk.csl`:

```yaml
filters: [cite-links, refs-slide]
```

`qd bib` installs both and names them in that order. Without qd: copy
`_extensions/refs-slide/` into the deck's folder. On its own, refs-slide
uses the deck's citation style, or Pandoc's default.

**How it works:**
- It shares cite-links' first step, run citeproc once (research R1). The
  same code is in both, so each works alone.
- The compact labels come from a second citeproc run that cites every
  listed key in the narrative form (research R3).

**Licence:** our own work, like the rest of quartz-deck. The icon is our own
SVG, the same as cite-links'.

`example.qmd` cites made-up references (test DOIs `10.5555/…` and
example.org links).
