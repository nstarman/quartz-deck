# Quickstart run: 014 citations (T040)

**Date**: 2026-11-15
**Build**: branch `014-polish` (debug), `qd serve` in the Claude browser pane
**Setup**: scratch decks in a scratch folder, citing made-up references (the
cite-links example's `references.bib`: test DOIs `10.5555/…`, example.org
links); qd's config and cache in scratch folders; Quarto 1.11.9; Zotero 7
running.

## §1 References in a scratch deck (US2) — pass

- [x] `qd bib` on a deck with no references: `"where": "none"`, and the four
  cited keys listed under `missing`.
- [x] `pull paper.bib` created `references.bib` beside the deck, set
  `bibliography:`, added all 7 entries, installed cite-links and
  refs-slide, and named both in `filters:`.
- [x] `embed` moved the entries into `references:` and left the file on
  disk. `extract --format json` wrote `references.json`. `convert --format
  bib` gave `references.bib` and removed the `.json`.
- [x] `qd undo` reverted the convert: `references.json` was back and
  `bibliography:` named it.
- [x] Pulling the same file again changed nothing. A copy with one DOI
  edited kept the key and listed `changed: DOI`.
- [x] With `bibliography: ../paper/refs.bib` (outside the deck's folder):
  `convert`, `extract --to ../paper/x.bib`, and a pull into it all failed
  with `E_READ_ONLY`, and the outside file was untouched. `embed` read it
  and wrote only the deck. Pulling it into another deck's own file worked.

## §2 Citations on slides (US1) — pass

- [x] A plain `quarto render` gave:
  - "Bell & Cho (2020)" with an icon, linked to its DOI, with "Bell & Cho
    2020: Two authors" on hover;
  - the parenthetical citation's key linked;
  - the corner form as the icon alone, top right;
  - the arXiv preprint linked to its arXiv page;
  - the entry without DOI or URL in plain text, with the render's warning.
- [x] In the editor (`qd serve`) the slide looked right: icon links in the
  text, the corner icon at the top right. Quarto added its references list
  at the end of the deck, which has no References slide.

## §3 Zotero (US3) — pass

- [x] `qd bib pull --from zotero KEY` against a real collection on this
  machine: all 15 items added, all with their Better BibTeX keys, in about
  3 s.
- [x] With the Zotero address pointing at a closed port (`QD_ZOTERO_URL`, so
  the running Zotero didn't need quitting), the pull failed at once with
  `E_SOURCE` carrying `qd-zotero`'s `E_ZOTERO` message and the hint to open
  Zotero.

## §4 The References slide (US4) — pass

- [x] The defaults, `refs-style="short" refs-columns="2"`, and
  `refs-style="full" refs-columns="1" refs-size="smallest"` each rendered
  with their classes. Every entry was linked except the one with no DOI or
  URL, in the short ("Dunn, Eko & Fry (2021), *Example Journal*") and full
  forms as expected.
- [x] Quarto gave the slide `smaller scrollable` each time. So a long list
  scrolls, and `qd check --offline` doesn't flag it (the
  `long_refs_slide_scrolls_without_an_overflow_warning` test). The
  quickstart's old expectation of a warning is corrected.

## §5 Converting a `paper` deck (US5) — pass

- [x] On a git copy of the migrate fixture (`crates/qd/tests/fixtures/migrate`),
  `qd bib migrate`: `differences: []`, no shortcodes left, `_extensions/paper`
  replaced by cite-links and refs-slide, the citations as `@arden2019` and
  `[[@bell2020]]{.corner}`, the list as a refs-slide `#refs` div with
  `nocite: '@*'`. (The skill's `lessons.qmd` fixture is already converted,
  so the quickstart now names this fixture.)
- [x] A real talk was converted on 2026-11-14:
  - a dated backup first, and the person's Zotero open;
  - the first run failed with `E_MIGRATE_DIFF` and put everything back;
  - the person read the report (label changes, arXiv links becoming DOIs,
    `https` to `http`, one extra entry) and said to accept it;
  - the second run, with `--accept`, kept it.

## §6 `paper` is gone — pass

- [x] Outside `specs/` and `design/`, the only matches are `qd bib migrate`
  itself (its code, tests and fixture), the docs and help that point old
  decks to it, and one comment in `store.rs`.

## Found

- The quickstart's §4 and §5 were out of date: the overflow warning, and the
  fixture to convert. Both are fixed in this PR.
