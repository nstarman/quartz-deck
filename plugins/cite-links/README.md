# cite-links

A Quarto filter extension: each citation links to its paper, not to a list
at the end, with a paper icon and "Label: Title" on hover. `talk.csl` gives
the labels a talk wants: "Name (2021)", "A & B (2021)", "A et al. (2021)",
with a/b on a clash.

```markdown
See @bonaca2020 and [@koposov2010; see @dunn2021, p. 3].

[[@dunn2021]]{.corner}
```

- `@key` and `[@key]` are Pandoc's own citations. Each key links to its DOI,
  else its URL. An arXiv DOI (10.48550) links the entry's arXiv page.
- `[[@key]]{.corner}` is the icon alone, top right of the slide. Its
  paragraph takes no room. It marks where a result slide's figure comes
  from.
- A key whose entry has neither DOI nor URL renders as plain text, and the
  render warns.

Options, in the front matter:

```yaml
filters: [cite-links]
cite-links:
  icon: false        # no icon in the text (the corner form keeps its icon)
  link: references   # link to the References list instead of the paper
csl: my-style.csl    # your own style instead of talk.csl
```

`qd bib` installs it into a deck's `_extensions/` and names it in
`filters:`. Without qd: copy `_extensions/cite-links/` into the deck's
folder. Then `quarto render` is enough.

**How it works:** Quarto runs citeproc after every filter, so the filter
runs citeproc itself (`pandoc.utils.citeproc`), once, and clears the
references so Quarto's own pass has nothing left to do (spec 014, research
R1).

**Licence:** our own work, like the rest of quartz-deck. `talk.csl` was
written from scratch from the CSL 1.0.2 specification and is not derived
from any other style. The icon is our own SVG.

`example.qmd` cites made-up references (test DOIs `10.5555/…` and
example.org links) in every form above.
