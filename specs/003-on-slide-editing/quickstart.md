# Quickstart: validating on-slide editing

Prerequisites: Quarto ≥ 1.4, Chrome (or qd's downloaded browser),
`cargo build --release`. Use a scratch copy of a deck.

## 1. Type on the slide (US1, SC-001/002)

```bash
qd serve --open
```

Click a slide title, change it, click elsewhere. Then:

```bash
git diff --word-diff deck.qmd
```

Only the title text changed. Edit a word inside a paragraph containing bold
and a link: the diff shows only that word. Press ⌘Z: the file is
byte-identical to before (`git diff` empty).

## 2. Formatting toolbar (FR-003a–c)

Select a word, press ⌘B: the source gains `**word**`. Pick a colour: the word
is wrapped in `[…]{style="color:…"}`. Remove both: the diff is empty again.

## 3. Source editing in place (US2)

Click an equation: a source editor opens over it with the TeX. Change it,
⌘Enter: the equation re-renders. Type `## New` into it and save: refused with
a message; your text is still there.

## 4. Menus and structure (US4, US6)

Right-click a paragraph: the menu lists the actions in contracts/ui.md.
Duplicate, Move up, Delete, Insert after ▸ List — each matches its `qd block`
command:

```bash
qd block dup results/1 --dry-run
```

Right-click a slide in the list: Background…, Transition ▸ fade.

## 5. Together with an agent (US5, SC-005)

Start typing in a paragraph without saving. In a terminal:

```bash
qd title methods "Methods (agent)"
```

The preview shows an "updated" badge but your typing is untouched; save — it
succeeds. Start editing the same paragraph again, then:

```bash
qd edit results/1 --md "Agent's version."
```

Save: a conflict is shown with the agent's text, and yours is kept.

## 6. Coverage

```bash
cargo test --test onslide -- --nocapture
```

Reports the share of corpus blocks that are typeable and verifies every
on-slide edit leaves the file byte-identical outside the edited element.
