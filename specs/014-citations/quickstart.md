# Quickstart: Citations (014)

A run-through that proves the feature end to end. Commands and JSON are in
[contracts/cli.md](contracts/cli.md), and the markup is in
[contracts/extensions.md](contracts/extensions.md).

## Prerequisites

- `qd` built from this branch: `cargo install --path crates/qd`.
- `qd-zotero`: `cargo install --path plugins/zotero` (Story 3 onward).
- Quarto ≥ 1.4 (measured on 1.11.9).
- For the Zotero steps: Zotero 7 running, with "Allow other applications on
  this computer to communicate with Zotero" ticked.

## 1. References in a scratch deck (US2)

In a scratch folder with `talk.qmd` (any deck) and a `paper.bib` exported
from any manager:

```bash
qd bib --deck talk.qmd
```

```bash
qd bib pull paper.bib --deck talk.qmd
```

```bash
qd bib embed --deck talk.qmd
```

```bash
qd bib extract --format json --deck talk.qmd
```

```bash
qd bib convert --format bib --deck talk.qmd
```

```bash
qd undo --deck talk.qmd
```

**Expected**:
- The first command reports `"where": "none"`.
- The pull creates `references.bib` beside the deck (or `_talk/references.bib`
  if `_talk/` exists) and sets `bibliography:`. It reports `added`, and
  installs both extensions.
- Embed moves the entries into `references:`.
- Extract writes `references.json`.
- Convert gives `references.bib`.
- Undo reverts the convert: the `.json` file is back, and `bibliography:`
  names it again.

Pull the same file again, then a copy with one DOI edited. The keys are
unchanged, and `changed` lists `DOI`. A `bibliography:` pointing to
`../paper/refs.bib` makes `extract` and `convert` fail with `E_READ_ONLY`,
while `pull ../paper/refs.bib` works.

## 2. Citations on slides (US1)

Add to a slide: `See @KEY and [@KEY2].` and, alone in a paragraph,
`[[@KEY]]{.corner}`.

```bash
quarto render talk.qmd
```

**Expected**, with plain Quarto and no qd:
- "Name et al. (year)" with an icon, linking to the DOI, with the title on
  hover;
- the corner icon top right, with the slide's layout unchanged;
- a key without DOI or URL renders plain, and the render warns.

## 3. Zotero (US3)

```bash
qd bib pull --from zotero COLLECTIONKEY --deck talk.qmd
```

**Expected**: the collection's items are merged, with Better BibTeX keys
when present. With Zotero quit, the same command fails within 5 s with
`E_SOURCE`, its message `E_ZOTERO`, and the hint to open Zotero.

## 4. The References slide (US4)

Add:

```markdown
## References

::: {#refs refs-style="short" refs-columns="2"}
:::
```

Render with `quarto render`. Then try each style and size, and `.scrollable`
on the slide.

```bash
qd check --offline --deck talk.qmd
```

**Expected**: each style looks as described, and every entry is linked. The
check warns when a long list overflows a slide that isn't scrollable.

## 5. Converting a `paper` deck (US5)

On a copy of `tests/skills/quarto-revealjs/fixture/lessons.qmd` in a git
folder:

```bash
qd bib migrate --deck lessons.qmd
```

**Expected**: no shortcodes left, `_extensions/paper` gone, the two new
extensions installed, and `differences: 0`.

**The PPP talk** (the builder runs it; the person opens Zotero and reads the
report):

```bash
cp -R "$HOME/Documents/Academia/Presentations/2026-PPP Talk" "$HOME/Documents/Academia/Presentations/2026-PPP Talk.backup-2026-11-13"
```

```bash
qd bib migrate --from zotero S376YTIE --backup "$HOME/Documents/Academia/Presentations/2026-PPP Talk.backup-2026-11-13" --deck "$HOME/Documents/Academia/Presentations/2026-PPP Talk/presentation.qmd"
```

**Expected**: it fails with `E_MIGRATE_DIFF` and the folder is restored.
The report lists about 11 label and 9 link differences (research R11).
After the person accepts the report, run the same command with `--accept`.
`make refs` in the talk then runs `qd bib pull --from zotero S376YTIE`.

## 6. `paper` is gone

```bash
grep -rn "{{< paper\|_extensions/paper\|bibliography:start\|--zotero" --include='*.qmd' --include='*.md' --include='Makefile' --include='*.rs' . | grep -v '^./specs/'
```

**Expected**: no matches outside `specs/` and the migrate code's own
shortcode parser and tests.
