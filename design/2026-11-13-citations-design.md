# Citations: Quarto's own, with links, a styled References slide, and pluggable sources

2026-11-13. From a brainstorm with the person who owns quartz-deck, after
`paper` became the first plugin (#64) and they asked whether Quarto had a
native way. Next step after approval: spec 014 through the spec-kit flow
(specify → plan → tasks).

## Intent

Citations in a deck should be Quarto's own, so a deck cites the way Quarto
users expect, and the same markup and reference files work in a paper:
- `[@key]` and `@key` in the text;
- references resolved by citeproc;
- a references list.

On top of that, keep what made `paper` useful in a talk:
- a citation links straight to the paper, not to a list at the end;
- the citation has an icon, and the paper's title on hover;
- a corner form: a small icon pinned to the slide's top right that takes no
  space. It marks where a result slide's figure comes from.

The References slide's look is the presenter's choice, from a few styles, so
it fits the slide or scrolls. References come from the person's citation
manager: Zotero today, others later.

This follows constitution VII (1.3.0): Quarto and existing libraries first,
plugins only for what's missing.

## Goals

1. **Native citations.** Decks cite with Pandoc citation syntax and render
   with plain `quarto render` (citeproc). No shortcode.
2. **References: a companion file by default, or embedded.**
   - Default: a companion file in the deck's support folder (`_talk/` for
     decks set up by the quarto-revealjs skill, else beside the deck),
     named by `bibliography:` in the front matter.
   - Option: the references embedded in the `.qmd` front matter as
     `references:` (CSL YAML), so the deck is one portable file. Quarto
     reads both natively.
3. **Two file formats.** BibTeX (`.bib`) and CSL-JSON (`.json`). A deck can
   cite from a paper's `.bib` (keys included).
   - **Outside the deck's folder, qd only reads.** A `bibliography:` file
     outside the deck's folder is read-only to qd: `pull` can import from it,
     while `pull` into it, `convert` and `extract` refuse, with a hint to
     copy it in. *(Decided by the person.)*
   - **Inside the deck's folder, qd owns the companion file** and may rewrite
     it whole through Pandoc. Comments and `@string` macros don't survive
     that, and the file's header comment says so.
4. **`qd bib` converts** between companion file and embedded, and between
   `.bib` and CSL-JSON, without changing a citation on any slide.
5. **Pull from sources.**
   - `qd bib pull <file>`: any exported file. This covers BibDesk, JabRef,
     Better BibTeX's auto-export, Paperpile and Mendeley exports.
   - `qd bib pull --from <source> <args>`: source plugins (Zotero first).
6. **Stable keys.** Refreshing from a source never breaks a citation on a
   slide (see Keys).
7. **Citation links** (render plugin `cite-links`):
   - in-text citations link to the paper (DOI or URL), with an icon and the
     title on hover;
   - the corner form `[[@key]]{.corner}`;
   - the text of a citation stays whatever the deck's CSL style makes it.
8. **References slide styles** (render plugin `refs-slide`): the presenter
   picks the style and size per deck; nothing measures or resizes the slide
   while it's shown. The options:
   - **style:** `compact` ("icon Name et al. (2020)", today's look), `short`
     (authors, year, venue), or `full` (the deck's CSL style);
   - **columns:** 1, 2 or 3;
   - **size:** normal, smaller or smallest;
   - **scroll:** reveal.js's `.scrollable`.

   `qd check` warns when the slide overflows (it already detects overflow).
9. **Convert now.** `paper` is removed in this work, not retired gradually.
   The decks using it are converted in the same change, with a backup first:
   - `Presentations/2026-PPP Talk/presentation.qmd`: 49 citations, 24 of them
     corner, its References from `qd bib --zotero S376YTIE`, not a git
     repository;
   - the skill's test fixture `tests/skills/quarto-revealjs/fixture/lessons.qmd`;
   - the skill's scaffold (`assets/`: `_extensions/paper`, `Makefile`'s
     `refs` target, `presentation.qmd`);
   - `plugins/paper`.

## Non-goals

- Automatic fitting of the References slide (it never resizes itself).
- A built-in reference editor: entries are edited in the citation manager or
  the file.
- Citation managers other than Zotero, beyond the file import. The source
  protocol makes them possible; none ships here.
- Syncing the nstarman-skills-small marketplace copy of the skill. Its owner
  copies from here, as for `paper` today; this work lists what changed.

## Anti-goals (a failure even if it works)

- A deck that renders only with qd. Every deck must render with plain
  `quarto render` once the two extensions are in its `_extensions/`.
- Breaking a citation when the references are refreshed or converted.
- A citation-manager connection at render or present time. Sources run only
  on `qd bib pull`.
- Python needed just to cite: a deck without code cells needs no Python.

## Constraints

- **Constitution 1.3.0, VII, as amended by this work:**
  - plugins come in two kinds: **render plugins** (Quarto extensions) and
    **source plugins** (feed `qd bib`, render nothing);
  - one `qd` binary, **plus the small binaries of plugins that need one, each
    built in the workspace**;
  - every plugin follows the core's rules on licences, offline use, Run, and
    writing only the deck and its own files.

  The amendment lands first, as its own PR.
- Edits go through `qd` ops (Principle III). `qd bib` is an edit: `--expect`,
  `--dry-run`, `qd undo`, a conflict check, and minimal patches (Principle
  II). The companion file is written atomically.
- Conversion uses the Pandoc bundled with Quarto (`quarto pandoc`): it reads
  and writes BibTeX, BibLaTeX and CSL-JSON. qd writes no format converter.
- Lua filters run after citeproc (`filters: [citeproc, cite-links]`, or the
  extension's filter at `at: post-quarto`; the plan picks one).
- Offline: the extensions bundle their CSS and icon. The offline check passes
  on their example decks.
- Licences: the extensions and `qd-zotero` are our code. Any third-party
  code a plugin bundles is listed in its manifest, and checked by step 4 of
  the plugin audit.

## Approaches considered

**How the Quarto side is split:**
- **A: one extension** (links, corner, References styles). Rejected: a deck
  wanting only the styled list would get citation rewriting too.
- **B: two extensions, `cite-links` and `refs-slide` (chosen).** One job
  each, each usable alone (also in a paper), each with its own example and
  test. `qd bib` installs both by default.

**Where citation-manager code lives:**
- **S2: compiled into qd behind an interface.** Rejected: a new manager
  would need a qd release, and these aren't plugins.
- **S1 + S3 (chosen).** File import (S3) is in core: most managers keep an
  exported file current, so they need nothing installed. Live sources (S1)
  are source plugins: a command that prints CSL-JSON. The Zotero plugin is
  written in **Rust** (`qd-zotero`), so citing needs no Python, and it reuses
  today's tested Zotero code and mock-server test.

## Design

### Pieces and where they live

| Piece | Kind | Where |
|---|---|---|
| `cite-links` | render plugin | `plugins/cite-links/` (`_extensions/cite-links/`: Lua filter, CSS, icon, and `talk.csl`; `example.qmd`; README) |
| `refs-slide` | render plugin | `plugins/refs-slide/` (Lua filter, CSS; `example.qmd`; README) |
| `qd-zotero` | source plugin | `plugins/zotero/`: a workspace crate building the binary `qd-zotero`, plus `qd-plugin.json` and a README |
| `qd bib` | core | `crates/qd/src/bib.rs` (rewritten) and `cli.rs` |
| `paper` | removed | `plugins/paper/` and the skill's copy deleted once nothing uses them |

**How decks get the plugins** *(my calls)*:
- **The extensions:** `qd` carries both, compiled in from `plugins/` with
  `include_str!`, and writes them into a deck's `_extensions/` when `qd bib`
  first needs them, or with `qd bib install`. It replaces a deck's copy only
  when its own version (`_extension.yml`) is newer, and warns instead if the
  deck's copy was edited. A deck outside this repository, such as the PPP
  talk, therefore needs only `qd`.
- **The source plugin:** `qd-zotero` is a second binary in the same
  workspace, installed like `qd` (`cargo install --git … qd-zotero`), and
  found on `PATH` as `qd-zotero`.

### Keys

When a pull meets an entry, matched by DOI, else URL, else normalised title:
- **already in the deck's references:** the entry keeps its key. The fields
  the source provides overwrite the file's, while fields only the file has
  are kept. Every changed field is listed in the result. The manager is the
  source of truth when pulling. *(My call.)*
- **new:** its key comes from the source when the source gives one (a
  Better BibTeX citekey, a `.bib` file's own key); otherwise qd makes one,
  `lastname` + year, lowercased ASCII, with `a`, `b`… on a clash
  (`bonaca2020`, `bonaca2020a`).

Entries no source returns any more are kept, and listed in a warning. The
person removes them; a pull never deletes. Today's label rules (et al.,
"&", programmers as authors, `n.d.`) feed the generated key's surname and
year.

### `qd bib`, the commands

| Command | Does |
|---|---|
| `qd bib` | lists the deck's references: where they live (file path or embedded), format, count, and keys cited on slides that have no entry |
| `qd bib pull <file>` | merges a `.bib`, BibLaTeX or CSL-JSON file (Keys) |
| `qd bib pull --from <source> [args…]` | runs the source plugin, reads its CSL-JSON, merges it (Keys) |
| `qd bib embed` | companion file → `references:` in the front matter; removes `bibliography:` |
| `qd bib extract [--to FILE] [--format bib\|json]` | embedded → companion file (default `_talk/references.bib`, or beside the deck); sets `bibliography:` |
| `qd bib convert --format bib\|json` | changes the companion file's format; renames it and updates `bibliography:` |
| `qd bib migrate` | converts `{{< paper >}}` decks (below) |

When the deck has no references yet, the first `pull` creates the companion
file in the default place, as `.bib`. Every write is a qd edit: `--expect`,
`--dry-run`, undo, and JSON output with the counts and warnings. The plan
decides whether undo covers the companion file or only the deck; the
decision must be stated.

### Source plugin protocol (S1)

`plugins/<name>/qd-plugin.json`:

```json
{ "source": { "name": "zotero", "command": "qd-zotero", "args": "COLLECTION" } }
```

- `qd bib pull --from zotero S376YTIE` runs `qd-<name>` from `PATH` (the
  manifest names the command, for documentation and for a plugin whose binary
  is named differently) with the given arguments.
- The command prints a CSL-JSON array on stdout, and errors on stderr, with
  a non-zero exit.
- A source may set an entry's `id` to its own key, which qd keeps for new
  entries.
- qd makes no network call itself. A source plugin's network use happens
  only during that command.

**`qd-zotero`:** reads the collection from Zotero 7's local API
(`localhost:23119`), with `format=csljson` and paged, and keeps today's
key check and errors:
- `E_ZOTERO`, with today's hint, when Zotero isn't running;
- the Better BibTeX citekey as `id` when that add-on is installed;
- arXiv's 10.48550 DOIs: the item's URL is preferred, as today.

Its test uses `QD_ZOTERO_URL` and a mock server, moved from `bib.rs`.

### `cite-links`

A Lua filter that runs after citeproc. For each citation:
- look up each cited key's DOI, else URL, in the document's references (the
  arXiv rule as above);
- **the citation style:** `talk.csl`, a small CSL style written from
  scratch for talks, reproduces today's look. `@key` renders "Name (2021)",
  "A & B (2021)" or "A et al. (2021)", and citeproc adds the a/b suffixes on
  a clash. The extension sets it as the default `csl:` unless the deck sets
  its own. It's written from scratch because standard CSL styles are CC
  BY-SA, which isn't on the licence allow-list. *(My call, following "keep
  today's look".)*
- wrap the citation's rendered text in a link to it, with the `cite-link`
  class, an icon span, and the title on hover (`Name et al. 2020: Title`);
- for a citation with several keys, link each key's part separately;
- a key with no DOI or URL stays plain text, and the filter warns.

**Corner form:** `[[@key]]{.corner}` renders as the icon alone, top right,
in a paragraph that takes no room (today's `paper-corner` CSS).

**Options** (front matter, `cite-links:`):
- `icon: true|false`;
- `link: paper|references`, so a deck can keep citeproc's own link to the
  list.

### `refs-slide`

A Lua filter that runs after citeproc, on the slide holding `::: {#refs}`.
It reads the slide's attributes:
- `refs-style`: `compact`, `short` or `full`;
- `refs-columns`: 1 to 3;
- `refs-size`: `normal`, `smaller` or `smallest`;
- reveal's own `.scrollable`.

The styles:
- **`full`:** citeproc's entries as they are, each entry's DOI or URL linked
  (`link-bibliography`).
- **`short`:** "Authors (year), *Venue*", linked.
- **`compact`:** "icon Name et al. (year)" links, sorted like today. The
  labels are citeproc's own, so the a/b suffixes match the citations in the
  text.

Defaults: `compact`, 3 columns, `smaller`. The skill's scaffold sets them on
its References slide.

### `qd bib migrate` (the conversion now)

For a deck using `{{< paper >}}`:
1. **References:** if the deck has a `bibliography:start/end` list from
   `qd bib --zotero KEY`, and a Zotero key is known (from `--from zotero
   KEY`, or the Makefile's `refs` line), pull the collection. Otherwise
   build entries from the shortcodes themselves (url, who, year, title).
2. **Citations:** each `{{< paper url=… >}}` becomes `@key` (narrative, so it
   still reads "Name et al. (2021)"), matched by URL. A `corner="true"` one
   becomes `[[@key]]{.corner}`. *(Decided by the person: keep today's look.)*
3. **The References slide:** the marker list becomes `::: {#refs}\n:::`,
   with `refs-slide` attributes reproducing today's look (`compact`, 3
   columns).
4. **Extensions:** installs `cite-links` and `refs-slide` into the deck's
   `_extensions/`, removes `_extensions/paper`, and updates the Makefile's
   `refs` target to `qd bib pull --from zotero …`.
5. **A check that nothing changed for the audience:** renders the deck
   before and after, and compares, per slide, the set of link targets and
   the visible citation text. **Any difference fails the migration.** The
   deck is restored from the backup, and every difference is reported.

`migrate` refuses a deck folder that isn't under git unless `--backup DIR`
is given. It is a permanent command, because the marketplace copy of the
skill keeps making `paper` decks until it's synced *(my call)*.

**The PPP talk** is converted in step 6, on this machine, by the builder
*(decided by the person)*:
1. copy the folder to a dated backup;
2. with Zotero running (the person opens it), run `qd bib migrate --from
   zotero S376YTIE`;
3. hand the before/after report to the person to check.

Shortcodes that no longer match an entry in today's collection by URL get
an entry built from the shortcode itself, and are listed in the report.

### Editor

Nothing new is required. Citations are text in paragraphs, and the
References slide is a div with attributes, editable as source. The editor
manifest (audit step 5) can later offer "Insert citation" and the
References slide's styles as panel fields.

## Order of work (each its own PR)

1. Constitution 1.4.0: VII's two plugin kinds, and plugin binaries in the
   workspace.
2. `qd bib` core: formats, conversion, embed and extract, file import, keys.
   The old `--zotero` path stays until step 6.
3. `qd-zotero` and the source protocol (`pull --from`).
4. `cite-links` (plugin, example deck, test).
5. `refs-slide` (plugin, example deck, test, overflow warning).
6. `migrate`, the conversion of the PPP talk (backup first) and the
   repository's fixtures and scaffold, and the removal of `paper`,
   `--zotero` and the old marker list. The skill's docs change too.

## Left to the builder

- Filter ordering against citeproc (`filters:` list or `at:`).
- The exact `short` formatting and the icon artwork. Today's mask SVG is
  ours and can be reused.
- Whether undo covers the companion file. It must be stated in the plan.
- Lua tests: through rendering the example decks, as in `plugins.rs`.
