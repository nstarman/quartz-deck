# paper

A Quarto shortcode for citing a paper on a slide: an icon and "Name et al.
(year)" as a link, with the title on hover.

```markdown
{{< paper url="https://doi.org/…" who="Name et al." year="2021" title="…" >}}
{{< paper url="…" who="…" year="…" corner="true" >}}   <!-- the icon alone, top right -->
```

`qd bib --zotero KEY` writes a References slide of these lines from a Zotero
collection (between `<!-- bibliography:start -->` and `<!-- bibliography:end -->`).
On a slide with the class `.bibliography`, the list is set in three columns.

**Install in a deck:** copy `_extensions/paper/` into the deck's (or its Quarto
project's) `_extensions/`.

**Copies:** the quarto-revealjs skill ships the same files in
`skills/quarto-revealjs/assets/_extensions/paper/`, so a deck set up from the
skill alone has it. This folder is the source: edit here, then copy over.
`crates/qd/tests/plugins.rs` fails when the two differ.

`example.qmd` renders with plain `quarto render`.
