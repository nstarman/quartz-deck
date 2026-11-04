# Round-trip coverage (T010 baseline, 2026-11-04)

From `cargo test --test onslide gate_ -- --nocapture` (Quarto 1.11.9, Chrome).
"Typeable" = the rendered element serializes back to exactly its source, so it
can be typed into; everything else opens as source (US2).

| Corpus deck | Typeable | Not typeable (reason) |
|---|---|---|
| prose | 4 / 5: quotes+dashes, soft breaks, escapes, entities | footnote/citation paragraph (rendered spans) |
| lists | 7 / 8: tight, nested 4-space, ordered, start, loose, `+`, incremental | task list (checkbox attributes) |
| inline | 4 / 7: emphasis/strike/code, underline/mark/colour/size spans, sup/sub, hard break | links line (autolink + titled link), `__`/`_` emphasis (serialized as `**`/`*`), inline math |
| blocks | 7 / 13: quote, aligned div, text box, sub-heading + its paragraph, notes-slide paragraph, fragment div | display math, code, callout, columns, raw HTML, table |
| **total** | **22 / 33 (67 %)** | |

Notes block (`::: notes`) is not in the preview and is excluded. Every
typeable block passed the splice check (no-op, an insertion at every mapped
position, and an append leave all other source bytes identical).

Known easy wins for later: plain autolinks, `_`/`__` emphasis (keep the
source's marker as the bullet hint does).

## Re-run after P2 and its audit (2026-11-04)

Same result: **22 / 33** typeable, splice check clean for every typeable
block. T035 is met by this check together with the CLI tests: the gate test
inserts at *every* mapped position of every typeable block (stronger than one
edited word) and asserts the source changes only there, and
`edit_changes_only_the_element_bytes` / `source_that_breaks_slide_structure_is_refused`
show `qd edit` (used by typed and source saves alike) writes nothing outside
the element and refuses structure changes.
