# Quickstart: Arrows on the slide (016)

A run-through proving the feature end to end. The commands are in
[contracts/cli.md](contracts/cli.md) and the gestures in
[contracts/editor.md](contracts/editor.md).

## Prerequisites

- `qd` built from this branch; Quarto ≥ 1.3 (measured on 1.11.9).
- Network for the first arrow (quarto-arrows is downloaded once).
- A scratch deck `talk.qmd` at 1600×900 with a slide `## A {#a}` holding a
  paragraph and an image.

## 1. Commands (US1–US4)

```bash
qd arrow add a --from 200,600 --to 700,300 --deck talk.qmd
```

```bash
qd arrow move a/2 --to 760,280 --deck talk.qmd
```

```bash
qd arrow bend a/2 --through 450,350 --deck talk.qmd
```

```bash
qd arrow set a/2 color=crimson width=4 dash=true label="the peak" fragment=true --deck talk.qmd
```

```bash
qd arrow show a/2 --deck talk.qmd
```

```bash
qd undo --deck talk.qmd
```

**Expected:**
- `add` adds one line before any notes and installs
  `_extensions/emilhvitfeldt/arrows/`.
- Each later command changes only its options in that line.
- `undo` steps back; undoing the `add` removes the line (the extension's
  files stay, unused).
- `quarto render talk.qmd` draws the arrow with its ends where given.

## 2. On the slide (US1)

```bash
qd serve --deck talk.qmd
```

1. Insert ▸ Arrow, then drag from the paragraph to the image. The ends snap
   to their edges with a guide line; hold ⌘ to place freely.
2. Click the arrow; drag its end; drag its line; nudge it with the arrow
   keys; press Escape mid-drag.
3. Delete it, then undo.

**Expected:** each step changes only its line, and the rendered arrow
lands where it was dropped.

## 3. Curve and style (US2)

Drag the middle handle sideways; set colour, width, dash and head in
Properties.

**Expected:** the arrow curves through the handle; only those options change.

## 4. Label and appearing on click (US3, US4)

Double-click the arrow, type a label; turn on "Appears on click"; present.

**Expected:** the label is drawn along the arrow; presenting, the arrow
draws in on the click.

## 5. Offline the first time

With `QD_NO_DOWNLOAD=1` and an empty cache, `qd arrow add` fails with
`E_DOWNLOAD` and the deck is unchanged.
