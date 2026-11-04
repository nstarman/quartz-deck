# Corpus for on-slide editing (spec 003)

Rendered by `tests/onslide.rs`; every block is put through the round-trip gate.

- `prose.qmd`: smart quotes, dashes, ellipsis, apostrophes, soft line breaks, escapes, footnotes, citations, cross-refs, entities.
- `lists.qmd`: tight, nested (4-space), ordered, `start`, loose, `+`/`*` bullets, incremental, task lists.
- `inline.qmd`: strong/emph/strike/code, links (plain, auto, titled), underline/mark/colour/size spans, sup/sub, hard breaks, `__`/`_` emphasis, inline math.
- `blocks.qmd`: display math, code, block quote, callout, columns, aligned div, positioned box, sub-heading, raw HTML, table, notes, fragment div.
