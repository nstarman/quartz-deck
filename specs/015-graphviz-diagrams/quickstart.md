# Quickstart: Diagrams you edit on the slide (015)

A run-through proving the feature end to end. The commands are in
[contracts/cli.md](contracts/cli.md) and the gestures in
[contracts/editor.md](contracts/editor.md).

## Prerequisites

- `qd` built from this branch.
- Quarto ≥ 1.4 (measured on 1.11.9).
- A scratch deck `talk.qmd` with one empty slide `## D {#d}`.

## 1. Commands (US1, US2)

```bash
qd dot new --on d --deck talk.qmd
```

```bash
qd dot show d/0 --deck talk.qmd
```

```bash
qd dot add d/0 --label Quarto --at 4,0 --look tool --from tool --deck talk.qmd
```

```bash
qd dot move d/0 n1 --by 0.5,0 --deck talk.qmd
```

```bash
qd dot rename d/0 data "This talk" --deck talk.qmd
```

```bash
qd dot rm d/0 'tool->n1' --deck talk.qmd
```

```bash
qd undo --deck talk.qmd
```

**Expected:**
- `new` inserts a pinned two-box diagram, `data` and `tool`; `add` prints
  `n1`.
- `show` lists it, `editable: true`.
- Each edit's `diff` touches one line.
- `undo` restores the edge.
- `quarto render talk.qmd` shows the boxes where pinned, with right-angle
  arrows.

## 2. Editing on the slide (US1, US2)

```bash
qd serve --deck talk.qmd
```

In the editor:
1. Drag a box: straight lines while dragging, routed arrows after the
   drop.
2. Drag from a box's right handle onto another box: an arrow.
3. Drag from a handle into empty space: a new connected box.
4. Double-click a box and rename it.
5. Select an arrow, then Delete.
6. Pick the "artifact" look for a box in Properties.

**Expected:** each step changes only its line of the cell, and `qd undo`
steps back through them.

## 3. Pinning a laid-out diagram (US1)

Paste a `{dot}` cell with no `layout` and no `pos` and drag one of its boxes.

**Expected:** the editor offers "Pin layout". After pinning, the slide
looks the same, and the drag works.

## 4. Pictures in boxes (US3)

```bash
qd dot set d/0 tool image=face.png --deck talk.qmd
```

```bash
quarto render talk.qmd
```

```bash
qd export --deck talk.qmd
```

**Expected:**
- the label gains a spacer;
- `_extensions/dot-image` is installed and named in `filters:`;
- the render shows the picture at the box's left, clear of the label;
- the one-file export shows it offline.

## 5. Live routing (US4)

Turn on Settings ▸ "Route arrows while dragging", then drag a
box.

**Expected:** routed arrows follow while dragging. The saved text is the
same as with the setting off.
