# Feature Specification: Citations

**Feature Branch**: `014-citations`

**Created**: 2026-11-13

**Status**: Draft

**Input**: User description: "Citations (replaces the `paper` plugin), following the approved design in design/2026-11-13-citations-design.md: native Quarto/Pandoc citations with links to the DOI/URL and a corner icon form (`cite-links` extension with a from-scratch `talk.csl` reproducing today's labels); a References slide with person-chosen styles (compact/short/full, columns, size, scrollable) via a `refs-slide` extension; `qd bib` core commands (companion bib file in the deck's support folder by default, or embedded `references:` in the qmd; .bib and CSL-JSON with conversion between them and between file and embedded; file import; read-only for files outside the deck folder); source plugins that print CSL-JSON (`qd-<name>` on PATH), the first being `qd-zotero` in Rust; `qd bib migrate` converting `{{< paper >}}` decks to `@key` citations with a before/after check that fails on any visible difference and restores from backup; converting the PPP talk and removing `paper` from the repo and skill."

## Context

Decks cite papers today with the `paper` shortcode (`{{< paper url=… >}}`),
and their References slide is a list that `qd bib --zotero KEY` writes
between two markers. That works only in quartz-deck decks. A paper written in
Quarto cites with `@key` and a bibliography file, and the two can't share
references.

This feature moves decks to Quarto's own citations, so the same markup and
reference files work in a talk and in a paper. It keeps what made `paper`
good in a talk: a citation links straight to the paper, with an icon and the
title on hover, and a corner icon marks where a figure comes from. The
References slide gets a few styles the presenter picks from. References come
from a citation manager (Zotero first, others through plugins) or any
exported file. Every deck that uses `paper` is converted in this work, and
`paper` is removed.

The approved design, with the approaches considered and the reasons, is
`design/2026-11-13-citations-design.md`. This spec states what the person
gets. The design's "decided by the person" items are requirements here.

## Clarifications

### Session 2026-11-13

- Q: In the PPP talk, 11 in-text `paper` citations carry hand-written labels
  that differ from the References list (e.g. "Myers, Starkman & Necib
  (2025)" where Zotero gives "Myers et al. (2026a)"), and 9 link to arXiv
  where the list links the DOI. Native citations print citeproc's label and
  one link per paper. What should migrate do? → A: Review, then accept.
  `migrate` fails and reports every difference; after the person reads the
  report, `migrate --accept` converts anyway. Labels become citeproc's, and
  each citation links to its entry's DOI or URL.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — Cite a paper and have the citation link to it (Priority: P1)

The presenter writes `@bonaca2020` in a slide. In the rendered deck it reads
"Bonaca et al. (2020)", the way it reads today, with a small paper icon. A
click opens the paper (its DOI, else its URL). Hovering shows the paper's
title. On a results slide, `[[@bonaca2020]]{.corner}` puts only the icon in
the slide's top right corner, taking no space, as a link to the paper the
figure comes from.

**Why this priority**: this is what the audience sees, and the reason
`paper` existed. Without it, moving to native citations loses the talk's
look.

**Independent Test**: an example deck with a few references, cited in the
narrative, parenthetical and corner forms, renders with plain
`quarto render` (no qd) and shows the links, icons, titles and labels.

**Acceptance Scenarios**:

1. **Given** a deck whose references include an entry with a DOI, **When**
   `@key` is rendered, **Then** it reads "Name et al. (year)" (or "Name
   (year)", "A & B (year)" by author count), links to the DOI, has the icon,
   and shows "Name et al. year: Title" on hover.
2. **Given** an arXiv entry whose DOI is arXiv's own (10.48550), **When** it
   is cited, **Then** the link goes to the entry's URL, as today.
3. **Given** a citation with two keys (`[@a; @b]`), **When** rendered,
   **Then** each key's part links to its own paper.
4. **Given** `[[@key]]{.corner}`, **When** rendered, **Then** only the icon
   shows, pinned top right, and the slide's other content doesn't move.
5. **Given** a cited key whose entry has neither DOI nor URL, **When**
   rendered, **Then** the citation shows as plain text and the render warns
   naming the key.
6. **Given** two entries by the same first author in the same year, **When**
   both are cited, **Then** their labels get "a" and "b" suffixes.
7. **Given** the deck sets its own citation style, or turns the icon or the
   paper link off, **When** rendered, **Then** the deck's choice wins.

---

### User Story 2 — Keep the deck's references in one place, in either format (Priority: P1)

The presenter (or an agent) adds references to a deck from any exported file:
BibTeX or BibLaTeX from BibDesk, JabRef or Better BibTeX's auto-export, or
CSL-JSON. By default they go in a companion file in the deck's support
folder. For a deck that must travel as one file, the presenter embeds them
in the deck's front matter, and can extract them back. They can switch the
companion file between BibTeX and CSL-JSON. Refreshing never breaks a
citation that's already on a slide. A deck can also cite from a paper's
own `.bib` file elsewhere on disk, which qd only reads.

**Why this priority**: every other story needs references in the deck, and
this works with any citation manager that can export a file.

**Independent Test**: on a deck with no references, pull a `.bib` file, cite
two keys, embed, extract, convert to CSL-JSON and back, and pull an updated
file. Every step renders the same citations, and every step can be undone.

**Acceptance Scenarios**:

1. **Given** a deck with no references, **When** a file is pulled, **Then**
   a companion `.bib` is created in the deck's support folder (or beside the
   deck when it has none), the deck names it, and the result lists how many
   entries were added.
2. **Given** a deck with a companion file, **When** it is embedded, **Then**
   the references move into the front matter, the deck no longer names a
   file, and every citation renders as before. Extracting reverses it.
3. **Given** a companion `.bib`, **When** it is converted to CSL-JSON (or
   back), **Then** the file is renamed, the deck names the new file, and
   every citation renders as before.
4. **Given** an entry already in the deck, **When** a pull meets the same
   paper (matched by DOI, else URL, else title), **Then** the entry keeps its
   key, the source's fields replace the deck's, fields only the deck has are
   kept, and the result lists every changed field.
5. **Given** a new entry without a key from its source, **When** it is
   added, **Then** it gets a key from the first author's surname and the
   year, lowercased ASCII, with a, b… on a clash.
6. **Given** entries the source no longer has, **When** a pull runs,
   **Then** they stay, and a warning lists them.
7. **Given** the deck names a bibliography file outside its own folder,
   **When** a pull into it, a convert or an extract is asked for, **Then**
   it is refused with a hint to copy the file in. Reading and citing from it
   still work, and pulling from it into the deck's own file works.
8. **Given** any of these writes, **When** run with `--dry-run`, **Then**
   nothing changes and the result says what would. **When** run with a stale
   `--expect`, **Then** it is refused as a conflict.
9. **Given** a deck, **When** `qd bib` is run with no subcommand, **Then**
   it lists where the references live, their format and count, and any keys
   cited on slides that have no entry.

---

### User Story 3 — Pull a deck's references from Zotero (Priority: P2)

The presenter keeps a Zotero collection for the talk. With Zotero open, one
command brings the collection into the deck's references, with the same key
rules as a file pull. Other citation managers can be added later as plugins
that print references in a standard format, without a new qd release.

**Why this priority**: it's the person's daily source and what the deck's
`make refs` does today, but a file export (Story 2) already covers it by
hand.

**Independent Test**: against a mock of Zotero's local service, pulling a
collection adds its entries with the right keys, and with Zotero not
running the command fails with a hint to open it.

**Acceptance Scenarios**:

1. **Given** Zotero is running and the collection exists, **When** the
   presenter runs `qd bib pull --from zotero KEY`, **Then** the collection's
   items are merged into the deck's references.
2. **Given** Better BibTeX is installed in Zotero, **When** items are
   pulled, **Then** new entries keep their Better BibTeX citekeys.
3. **Given** Zotero isn't running, **When** the pull runs, **Then** it fails
   with a clear error and a hint to open Zotero, and the deck is unchanged.
4. **Given** a collection key that isn't 8 letters and digits, **When** the
   pull runs, **Then** it is refused before any connection is made.
5. **Given** a source name with no installed plugin, **When** the pull runs,
   **Then** it fails naming the program it looked for.
6. **Given** a source plugin that exits with an error or prints something
   that isn't a reference list, **When** the pull runs, **Then** it fails
   with the plugin's message and the deck is unchanged.

---

### User Story 4 — Choose how the References slide looks (Priority: P2)

The presenter puts a References slide in the deck and picks its style:
compact ("icon Name et al. (2020)" links, today's look), short (authors,
year, venue) or full (the deck's citation style). They also pick 1 to 3
columns, a text size, and whether the slide scrolls. Nothing resizes the
slide on its own; `qd check` warns when it overflows.

**Why this priority**: the References slide matters once per talk, and the
default (compact, 3 columns, smaller) already matches today.

**Independent Test**: an example deck with one References slide per style
renders with plain `quarto render`, each in its chosen style, columns and
size, with every entry linked.

**Acceptance Scenarios**:

1. **Given** a References slide with no options, **When** rendered,
   **Then** it shows compact entries in 3 columns at the smaller size,
   sorted like today, each linking to its paper.
2. **Given** `short` or `full`, **When** rendered, **Then** the entries are
   in that form, each linked to its DOI or URL.
3. **Given** compact entries for two papers by the same first author and
   year, **When** rendered, **Then** the a/b suffixes match those in the
   slides' citations.
4. **Given** the scroll option, **When** the deck is presented, **Then** a
   list longer than the slide can be scrolled.
5. **Given** a list that overflows the slide without scrolling, **When**
   `qd check` runs, **Then** it warns about that slide.

---

### User Story 5 — Convert every `paper` deck now, and remove `paper` (Priority: P3)

Every deck that uses the `paper` shortcode is converted in this work: the
PPP talk, the skill's test fixture and scaffold, and the plugin's own
example. The audience sees no difference. One command does it, with a backup
first and a before/after check that fails the conversion, and puts the deck
back, on any visible difference. Where a difference is expected (a label
written by hand, an arXiv link where the entry has a DOI), the person reads
the report and runs the conversion again with `--accept`. Then `paper`, the old `--zotero` option and
the old marker list are removed.

**Why this priority**: it needs Stories 1–4 to exist. It's what lets
`paper` go.

**Independent Test**: on a copy of a `paper` deck, the conversion produces a
deck with no shortcodes whose rendered slides have the same link targets and
citation text per slide. On a deck with an injected difference, the
conversion fails and the deck is restored byte for byte.

**Acceptance Scenarios**:

1. **Given** a deck using `{{< paper >}}`, **When** `qd bib migrate` runs,
   **Then** each shortcode becomes `@key` matched by URL, else by title, each corner one
   becomes `[[@key]]{.corner}`, the marker list becomes a References slide
   in today's look, the new extensions are installed, `paper`'s copy is
   removed, and the Makefile's `refs` target pulls from Zotero.
2. **Given** a Zotero collection is known (from the command, or the
   Makefile's `refs` line), **When** the conversion runs, **Then** the
   references come from it. **Given** none is known, **Then** they are
   built from the shortcodes' own details.
3. **Given** a shortcode whose URL matches no entry in the collection,
   **When** the conversion runs, **Then** an entry is built from the
   shortcode and the report lists it.
4. **Given** the rendered deck before and after differ on any slide in link
   targets or visible citation text, **When** the check runs, **Then** the
   conversion fails, the deck folder is restored from the backup, and the
   report lists every difference.
5. **Given** that report, **When** the person runs the conversion again with
   `--accept`, **Then** it converts, with citeproc's labels and each
   citation linking to its entry, and the report still lists every
   difference.
6. **Given** a deck folder not under git, **When** the conversion runs
   without a backup folder, **Then** it is refused.
7. **Given** the PPP talk, **When** it is converted (dated backup first,
   Zotero opened by the person), **Then** the person gets the report to
   check, and decides whether to accept it.
8. **Given** the conversion is done everywhere, **When** the repository is
   searched, **Then** no `paper` shortcode, extension, `--zotero` option or
   marker list remains, and the skill's docs and scaffold use the new
   citations.

---

### Edge Cases

- The same key is cited before its entry exists: the render shows Quarto's
  own missing-citation mark, and `qd bib` lists the key.
- A deck has both `bibliography:` and embedded `references:`: `qd bib`
  reports both, embed and extract merge them by the key rules, and Quarto
  renders both.
- A pull file has two entries for the same paper: they merge into one, and
  the result says so.
- Two different papers would get the same generated key: suffixes a, b…
  keep them apart.
- A companion file the person edited by hand, with comments or `@string`
  macros: a rewrite drops those, and the file's header comment says so up
  front.
- The deck's copy of a plugin extension was edited: qd doesn't replace it,
  and warns.
- The deck's copy of an extension is newer than qd's: qd leaves it.
- A deck renders without qd: it works, as long as the two extensions are in
  its `_extensions/`.
- Offline: rendering and presenting reach no network. Only a source pull
  does, during that command.
- The conversion meets a `paper` deck with no References slide: it converts
  the citations and adds no slide.
- A `paper` shortcode with arguments the conversion doesn't know: the
  conversion stops before writing and names the slide.

## Requirements *(mandatory)*

### Functional Requirements

**Citations on slides (cite-links)**

- **FR-001**: Decks MUST cite with Pandoc's citation syntax (`@key`,
  `[@key]`, `[@a; @b]`) and MUST render with plain `quarto render` when the
  plugin extensions are in the deck's `_extensions/`.
- **FR-002**: Each cited key MUST link to its paper: the DOI, else the URL;
  an arXiv DOI (10.48550) MUST prefer the URL. A citation of several keys
  MUST link each key's part to its own paper.
- **FR-003**: Each linked citation MUST show an icon and, on hover, "label:
  title" (for example "Bonaca et al. 2020: Title").
- **FR-004**: `[[@key]]{.corner}` MUST render the icon alone, top right,
  without taking space on the slide.
- **FR-005**: The default citation style MUST reproduce today's labels:
  "Name (year)", "A & B (year)", "A et al. (year)", with a/b suffixes on a
  clash. It MUST be our own work (no copied CC BY-SA style). A deck's own
  `csl:` MUST win.
- **FR-006**: A cited key with no DOI or URL MUST render as plain text and
  the render MUST warn naming the key.
- **FR-007**: A deck MUST be able to turn off the icon, and to link
  citations to the References list instead of the paper.

**References slide (refs-slide)**

- **FR-008**: A References slide (`::: {#refs}`) MUST offer the styles
  `compact`, `short` and `full`, 1–3 columns, the sizes normal, smaller and
  smallest, and reveal.js's scrolling, chosen on the slide.
- **FR-009**: The defaults MUST be compact, 3 columns, smaller. Every entry
  MUST link to its DOI or URL. Compact labels MUST match the citations'
  labels, suffixes included.
- **FR-010**: Nothing MUST resize the References slide while it's shown.
  `qd check` MUST warn when a References slide overflows and doesn't scroll.

**References in the deck (qd bib)**

- **FR-011**: `qd bib` MUST list where the references live (file path or
  embedded), the format, the entry count, and keys cited on slides that
  have no entry.
- **FR-012**: `qd bib pull FILE` MUST merge BibTeX, BibLaTeX or CSL-JSON
  files into the deck's references.
- **FR-013**: The first pull into a deck with no references MUST create a
  companion `.bib` in the deck's support folder (`_talk/` when present,
  else beside the deck) and name it in the front matter.
- **FR-014**: `qd bib embed` MUST move the companion file's entries into the
  front matter as `references:` and remove `bibliography:`;
  `qd bib extract [--to FILE] [--format bib|json]` MUST do the reverse.
- **FR-015**: `qd bib convert --format bib|json` MUST change the companion
  file's format, rename it, and update the front matter.
- **FR-016**: Embed, extract and convert MUST leave every rendered citation
  unchanged.
- **FR-017**: Entries MUST be matched by DOI, else URL, else normalised
  title. A matched entry MUST keep its key; the source's fields MUST replace
  the deck's, fields only the deck has MUST be kept, and the result MUST
  list every changed field.
- **FR-018**: A new entry MUST keep the key its source gives. Otherwise its
  key MUST be the first author's surname plus year, lowercased ASCII, with
  a, b… on a clash, following today's label rules for the surname and year.
- **FR-019**: A pull MUST NOT delete entries. Entries the source no longer
  has MUST be listed in a warning.
- **FR-020**: A bibliography file outside the deck's folder MUST be
  read-only to qd: pulling into it, converting it and extracting to it MUST
  be refused with a hint to copy it in. Citing from it and pulling from it
  MUST work.
- **FR-021**: qd MAY rewrite a companion file inside the deck's folder whole.
  The file MUST start with a comment saying qd writes it and that comments
  and macros won't survive.
- **FR-022**: Every `qd bib` write MUST be a qd edit: `--dry-run`,
  `--expect` with conflict refusal, undo, an atomic write of each file, and
  JSON output with counts and warnings. The plan MUST state whether undo
  covers the companion file.
- **FR-023**: Format conversion MUST use the Pandoc that comes with Quarto;
  qd MUST NOT carry its own converter.

**Source plugins and Zotero**

- **FR-024**: `qd bib pull --from NAME ARGS…` MUST run the program
  `qd-NAME` found on `PATH` with the given arguments, read a CSL-JSON array
  from its output, and merge it by FR-017–FR-019. A missing program, a
  non-zero exit or invalid output MUST fail with the plugin's message and
  leave the deck unchanged.
- **FR-025**: A source plugin MUST be described by a `qd-plugin.json` naming
  its source and command, in its folder under `plugins/`.
- **FR-026**: `qd-zotero COLLECTION` MUST read the collection from Zotero 7's
  local service, all pages, and print it as CSL-JSON. It MUST refuse a
  collection key that isn't 8 letters and digits before connecting, fail
  with today's error code and hint when Zotero isn't running, use Better
  BibTeX citekeys as keys when present, and prefer the URL for arXiv DOIs.
- **FR-027**: `qd-zotero` MUST be built in this repository's workspace and
  installable on its own. Citing MUST NOT need Python.
- **FR-028**: qd itself MUST NOT reach the network for citations. Only a
  source plugin may, and only while its pull runs.

**Getting the extensions into a deck**

- **FR-029**: qd MUST carry both extensions and install them into a deck's
  `_extensions/` when `qd bib` first needs them, or on `qd bib install`.
- **FR-030**: qd MUST replace a deck's copy only when its own version is
  newer, and MUST warn instead of replacing a copy that was edited.
- **FR-031**: Each extension MUST bundle its CSS and icon; its example deck
  MUST pass the offline check.

**Converting `paper` decks**

- **FR-032**: `qd bib migrate [--from zotero KEY] [--backup DIR]` MUST
  convert a deck using `{{< paper >}}`:
  - references from the Zotero collection (given, or read from the
    Makefile's `refs` line), else built from the shortcodes;
  - each shortcode to `@key` matched by URL, else by normalised title,
    corner ones to
    `[[@key]]{.corner}`;
  - the marker list to a References slide in today's look;
  - the extensions installed, `_extensions/paper` removed, and the
    Makefile's `refs` target changed to pull from Zotero.
- **FR-033**: A shortcode matching no collection entry MUST get an entry
  built from its own details, listed in the report.
- **FR-034**: The conversion MUST render the deck before and after and
  compare, per slide, the link targets and the visible citation text. Any
  difference MUST fail the conversion, restore the deck folder from the
  backup, and report every difference, unless `--accept` is given, in which
  case the conversion MUST keep its result and still report every
  difference.
- **FR-035**: The conversion MUST refuse a deck folder not under git unless
  `--backup DIR` is given, and MUST make the backup before any write.
- **FR-036**: The conversion MUST stop before writing, naming the slide, on
  a shortcode it can't convert.
- **FR-037**: `qd bib migrate` MUST stay as a permanent command.
- **FR-038**: In this work, the PPP talk, the skill's fixture and scaffold
  and the plugin examples MUST be converted, and then `paper` (plugin and
  skill copy), `qd bib --zotero` and the marker list MUST be removed. The
  skill's docs MUST describe the new citations.

**Constitution**

- **FR-039**: Before any plugin code lands, the constitution MUST be
  amended (1.4.0) so VII allows render plugins and source plugins, and
  plugin binaries built in the workspace, under the core's rules on
  licences, offline use, Run and writes.

### Key Entities

- **Reference entry**: one paper. A key, type, authors, year, title, venue,
  DOI and/or URL, and any other fields its source gives.
- **Deck references**: the deck's entries, in a companion file (BibTeX or
  CSL-JSON, inside or outside the deck's folder) or embedded in the front
  matter. Read-only when the file is outside the deck's folder.
- **Citation**: a place on a slide citing one or more keys, in the narrative,
  parenthetical or corner form.
- **References slide**: the slide holding the list, with its style, columns,
  size and scrolling.
- **Source plugin**: a program `qd-NAME` that prints a deck's references from
  a citation manager, described by its manifest.
- **Pull result**: entries added, changed (with their fields), merged,
  no longer in the source; warnings.
- **Conversion report**: per slide, the before and after link targets and
  citation text, the differences, and the entries built from shortcodes.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: The PPP talk (49 citations, 24 corner) is converted, and every
  per-slide difference in link targets and citation text is in the report
  the person reviews before accepting; none is unreported.
- **SC-002**: After this work, no deck, fixture, scaffold or plugin in the
  repository uses `paper`, and every converted deck renders with plain
  `quarto render`.
- **SC-003**: A presenter adds a citation that links to its paper in one
  edit (typing `@key`), with no shortcode and no extra step.
- **SC-004**: Embedding, extracting and converting a deck's references, in
  any order, leave 100% of its rendered citations unchanged.
- **SC-005**: Refreshing a deck's references from its source changes no key
  that a slide cites, in every test case.
- **SC-006**: Refreshing from Zotero is one command, and with Zotero closed
  it fails in under 5 seconds with a hint to open it.
- **SC-007**: Each of the three References styles, at each column count and
  size, renders in the example deck with every entry linked.
- **SC-008**: A deck that doesn't use code cells can cite and pull with no
  Python installed.

## Assumptions

- The person uses Zotero 7 with its local service on; Better BibTeX is
  optional.
- Quarto (with its bundled Pandoc) is installed wherever decks render, as
  today.
- The skill's marketplace copy (nstarman-skills-small) is synced by its
  owner; this work lists what changed and doesn't edit it.
- The PPP talk isn't a git repository, so its conversion uses a dated backup
  folder beside it.
- Citation managers other than Zotero work through file export in this
  work; their source plugins come later.
- No editor UI is added. Citations are text, and the References slide is a
  div with attributes, both editable as source. "Insert citation" can come
  with the editor manifest later.
- The order of work is six PRs: the constitution amendment, `qd bib` core,
  `qd-zotero` and the source protocol, `cite-links`, `refs-slide`, then the
  conversion and removal of `paper`.
