# Research: Citations (014)

Checked on this machine with Quarto 1.11.9 (Pandoc 3.10), on 2026-11-13.
Spikes ran in the session scratchpad, and none of their code is kept.

## R1. When the extensions see citeproc's output

**Decision**: `cite-links` and `refs-slide` each run citeproc themselves,
from a filter at `at: post-quarto`, with `pandoc.utils.citeproc(doc)`. Then
they remove `bibliography`, `references` and `nocite` from the metadata, so
Quarto's own final citeproc pass has nothing to do. Whichever filter runs
first does this. The second sees the metadata gone and skips the step. The
step is the same eight lines in both extensions, since each must work alone.

**Measured**:
- A filter at `post-quarto`, `pre-render`, `post-render`, `pre-finalize` or
  `post-finalize` still sees raw `Cite` elements and an empty `#refs`. Quarto
  runs citeproc after every filter entry point.
- `filters: [citeproc, cite-links]` in the deck stops Quarto 1.11.9 with
  "FATAL QUARTO ERROR" (main.lua:591).
- When the filter calls `pandoc.utils.citeproc` itself, citations come out
  rendered ("Bonaca et al. (2020)", "(Koposov 2010)"), `#refs` is filled,
  and Quarto's later pass leaves the document alone.
- Setting `meta.csl` from a filter at `post-quarto` changes the style that
  Quarto's citeproc uses. An empty `talk.csl` made it fail with
  `CiteprocXMLError`, which shows the setting is read.

**Alternatives rejected**: ask decks to list `citeproc` first (it crashes
Quarto 1.11); post-process the HTML in JS (the rendered deck would then
depend on JS for its links, and the PDF and PPTX exports would lose them).

## R2. The default style, `talk.csl`

**Decision**: a small CSL 1.0.2 style written from scratch in
`plugins/cite-links/_extensions/cite-links/talk.csl`. It is set as
`meta.csl` before citeproc runs, unless the deck sets `csl`. Its rules:
- **citations**: author-date, narrative ("Name (2021)", "A & B (2021)",
  "A et al. (2021)" from 3 authors), with `disambiguate-add-year-suffix`;
- **bibliography**: authors, year, title, container, DOI or URL. This is the
  `full` style of the References slide.

It is written from scratch because standard CSL styles are CC BY-SA, which
isn't on the licence allow-list (Constraints). A test renders the example
deck and checks those three label forms and the a/b suffix.

## R3. The compact References labels match the citations

**Decision**: for `compact`, `refs-slide` builds a scratch document holding
one narrative citation per entry in `#refs`, runs `pandoc.utils.citeproc` on
it with the same metadata, and uses each citation's rendered text as that
entry's label. The labels are therefore citeproc's own, suffixes included,
by construction. `short` is formatted in Lua from
`pandoc.utils.references(doc)`: "Authors (year), *Venue*". `full` keeps
citeproc's entries.

## R4. Undo covers the companion file

**Decision**: yes. A `qd bib` write that changes the companion file goes
through the same `Store` path (an atomic write, and the conflict check
against the version it read). The deck's undo journal entry gains an
optional `also` list of `{path, before, after}`, kept in the cache like
today's entries. `qd undo` and `qd redo` restore every file in the entry.
They refuse (`E_NO_UNDO`) if any of those files changed since the edit.
When the companion file is the only file that changed (a `pull` into an
existing file), the entry's deck text is unchanged, and it still goes on the
deck's journal. So one `qd undo` always undoes the last `qd bib` command.

**Alternative rejected**: a second journal per companion file. Two undo
stacks for one command would be confusing, and undo would need to know
which stack to use.

## R5. Formats and conversion

**Decision**: qd works on CSL-JSON in memory (`serde_json::Value`). Pandoc
from Quarto does every format change:
- reading: `quarto pandoc -f bibtex|biblatex -t csljson FILE`, where `.bib`
  is read as BibLaTeX (a superset);
- writing `.bib`: `quarto pandoc -f csljson -t biblatex`;
- `.json` is read and written directly.

Embedded `references:` is the same CSL data as YAML, written with
`serde_yaml_ng` (already a dependency) through the existing front-matter
edit (`qd meta set`'s patch). So only the front matter changes, and the rest
of the deck's bytes are kept (Principle II).

**Measured**: a BibTeX → CSL-JSON → BibLaTeX round trip keeps keys
(`id`), authors, title, year and DOI. Pandoc writes `date = {2020}` where
the input had `year`. That is why a companion file that qd rewrites carries
the header comment (FR-021).

## R6. Matching and keys

**Decision**: one function, `bib::merge(existing, incoming) -> Merge`,
matches entries in this order:
1. DOI, compared lowercased with `https://doi.org/` stripped;
2. URL, normalised: scheme dropped, `www.` and a trailing `/` dropped, and
   an arXiv `abs/` or `pdf/` id with its version dropped;
3. title, lowercased, with punctuation and whitespace runs folded.

Key generation reuses today's `label` rules in `bib.rs`: the first
author's family name or literal name, ASCII-folded, lowercased, letters
only, then the four-digit year (`nd` without one), then `a`, `b`… on a
clash. `bib.rs`'s Zotero fetch and its `paper` lines move out (R8, R11).

## R7. Source plugins

**Decision**: `qd bib pull --from NAME ARGS…` runs `qd-NAME` with
`std::process::Command`, found on `PATH`, with ARGS after it. It reads
stdout as a JSON array. A non-zero exit gives `E_SOURCE` with the plugin's
stderr. A missing program gives `E_SOURCE` with the hint "install
`qd-NAME`, e.g. `cargo install --git … qd-NAME`". Output that isn't an
array of objects gives `E_SOURCE` too. Nothing is written before the output
is parsed. qd sets no timeout; the plugin is responsible for its own.

`qd-plugin.json` (`plugins/zotero/qd-plugin.json`) documents the source's
name, command and arguments. qd doesn't need it at run time (YAGNI: no
registry).

## R8. `qd-zotero`

**Decision**: a second workspace member, `plugins/zotero/` (crate
`qd-zotero`, one `main.rs`), using `ureq` and `serde_json`, which are already
in the lockfile. Its code moves from `bib.rs`: `fetch`, the key check,
`QD_ZOTERO_URL`, and the arXiv rule.
- **The request**: `GET /api/users/0/collections/KEY/items/top?include=data,csljson&limit=100&start=N`,
  so one request gives each item's CSL-JSON (printed) and its data. The
  data supplies the citekey: Zotero 7's own `citationKey` field, else Better
  BibTeX's `Citation Key:` line in `extra`. The key becomes the entry's
  `id`.
- **The arXiv rule**: for an item whose DOI starts with 10.48550 and that
  has a URL, the DOI is dropped from the printed entry, so the URL is the
  link.
- **Zotero not running**: the connection is refused at once. ureq gets a
  5-second connect timeout (SC-006). It exits 2 with today's `E_ZOTERO`
  message and hint on stderr.
- **The tests**: the mock server test moves from `crates/qd/tests` to
  `plugins/zotero/tests`. Whether `include=csljson` works on the local API
  is checked against the mock first. Then it is checked once against the
  person's running Zotero during the PPP conversion. If the local API
  ignores it, the fallback is `format=json` with the CSL mapping done in the
  plugin, which today's `label` and `link` code already half does.

## R9. Getting the extensions into a deck

**Decision**: `crates/qd/src/plugins.rs` holds each extension's files as
`include_str!` from `plugins/<name>/_extensions/<name>/`. `qd bib install`,
and every `qd bib` write, call `install(deck_dir)`. It compares the version
in the deck's `_extension.yml` with qd's:
- no copy, or the deck's version is older: write qd's files;
- the same version with different bytes: leave it, warn `W_EDITED`;
- the deck's version is newer: leave it.

The writes are atomic and are reported in the command's JSON (`installed`).
They aren't journaled for undo: they are tool files, not the person's work,
and a re-install restores them. `ponytail:` an edited copy of an *older*
version is overwritten, because qd doesn't keep old versions' bytes. Keep a
hash per released version if that ever matters.

## R10. The References slide overflowing

**Decision**: no new code. The rendered check (`qd check --offline`,
`offline.rs`) already flags content past the slide, and it skips elements
that scroll (`overflow` not `visible`). That covers `.scrollable`. A test
renders a long References slide without and with `.scrollable` and expects
the warning only for the first.

## R11. `qd bib migrate`

**Decision**:
- **Parse**: shortcodes are found with the regex `bib.rs` already uses,
  and their arguments with the same `key="value"` rule. Unknown arguments
  give `E_INVALID_ARG` naming the slide, before any write.
- **References**: from `--from zotero KEY`, else the Makefile's `refs`
  line (`qd bib --zotero KEY`), through R7. Without either, every entry is
  built from its shortcode: `id` from R6, `author` as a literal `who`,
  `issued` from `year` without the suffix, `title`, and `URL` or `DOI` from
  `url`.
- **Matching**: R6 (URL, then title). An unmatched shortcode's entry is
  built from the shortcode and listed in the report.
- **The rewrite**: one patch per shortcode (Principle II):
  - an inline shortcode becomes `@key`;
  - a corner one becomes `[[@key]]{.corner}`;
  - the marker list becomes `::: {#refs refs-style="compact" refs-columns="3"}`
    and `:::`;
  - the slide's `.bibliography` class stays.
- **The check**: renders before and after with qd's render pipeline into
  qd's cache. From each `section`, it collects `(href, visible text, title
  attribute)`:
  - before: for `a.paper-ref`;
  - after: for `a.cite-link`, and for the `#refs` entries' links.

  Then it compares slide by slide.
- **The result**:
  - **Without `--accept`**: any difference restores every touched file
    from the backup and exits `E_MIGRATE_DIFF`. The JSON report is kept in
    qd's cache, and its path is printed.
  - **With `--accept`**: the result is kept, and the same report is
    returned.
- **The backup**: before any write, every file the conversion will touch is
  copied: the deck, its Makefile, `_extensions/paper/`, and the companion
  file if it exists. They go to `--backup DIR`, or for a git deck to
  `<cache>/migrate/<hash>/<timestamp>/`. A non-git folder without
  `--backup` is refused (`E_INVALID_ARG`).

**PPP talk facts (measured)**: 35 shortcodes in the text, 24 of them
corner, and 15 in the marker list. Comparing them to that list: 11 in-text
labels differ (hand-written "Myers, Starkman & Necib (2025)", "Euclid
Collaboration: Starkman et al."), and 9 in-text URLs are arXiv links where
the list has the DOI (matched by title in R6). These are the differences
the person reviews before `--accept` (Clarifications).

## R12. The constitution

**Decision**: amendment 1.4.0 (MINOR, VII expanded) lands first, as its own
PR, through `/speckit-constitution`:
- plugins are **render plugins** (Quarto extensions) or **source plugins**
  (programs that feed `qd bib` and render nothing);
- a plugin that needs a binary builds it in the workspace (`plugins/<name>/`
  as a member crate);
- "one `qd` binary" becomes "one `qd` binary, plus plugin binaries";
- the plugin rules are unchanged.

## Licences

There are no new third-party dependencies. `qd-zotero` uses `ureq`
(MIT/Apache-2.0) and `serde_json` (MIT/Apache-2.0), both already used by
`qd`. `talk.csl`, the Lua filters, the CSS and the icon (today's `paper`
mask SVG) are our own work.
