# Quickstart: slide groups (018)

A run-through that proves the feature end to end on a made-up deck. Run it
from a scratch folder, never on a real talk.

## Setup

```bash
qd new groups.qmd --title "Groups"
qd add --title "Intro" && qd add --title "Methods" && qd add --title "M1" \
  && qd add --title "M2" && qd add --title "Results" && qd add --title "R1"
```

## 1. Make groups from the CLI

```bash
qd indent m1        # Methods becomes `#`, M1 its sub-slide; M2, Results and R1 follow (R3)
qd outdent results  # Results becomes `#`; R1 follows it
qd slides | jq '.slides[] | {id, head, in}'
```

Expected: `methods` and `results` have `head: true`; `m1`, `m2` are `in:
"methods"`, `r1` is `in: "results"`; `intro` has neither. `git diff`
shows only `##` → `#` on two lines.

## 2. Render matches the list

```bash
quarto render groups.qmd
```

Expected: the HTML has two vertical stacks, Methods (with M1, M2) and
Results (with R1), and Intro outside them (SC-004).

## 3. Move a group, outdent, undo

```bash
qd move results r1 --before methods
qd outdent m2       # M2 becomes a head of its own
qd undo && qd undo
```

Expected: after the move, Results and R1 come before Methods; after
`outdent`, M2 is `#`; two undos give back the file of step 1 byte for byte.

Time the editor version of steps 1 and 3 in section 5: turning a run of
slides into a group, and dragging a collapsed group of ten slides, each
takes under 10 seconds (SC-002).

## 4. Navigation mode

```bash
qd navigation vertical && qd meta
qd navigation linear && qd meta
```

Expected: `navigation-mode: vertical` appears, then is gone again.

## 5. In the editor (built-in browser pane or headless Playwright)

`qd serve groups.qmd`, then:
- the list shows M1, M2 indented under Methods; the triangle collapses it
  to one row with "+2"; reloading keeps it collapsed; the `.qmd` is
  unchanged (SC-001);
- focus Intro's row and press Tab: refused, with "nothing above to go
  under" (only the title slide is above it); Escape returns focus to the
  canvas;
- drag the collapsed Methods row below Results: one move, one undo;
- drag R1 to the end of the list, 16 px to the left: it lands there as a
  head; dropped the same way between M1 and M2, the gap doesn't open
  (FR-017).
