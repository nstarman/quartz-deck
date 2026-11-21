# Quickstart run: slide groups (018)

Run on 2026-11-20 against main at ea6b725 (all four user stories merged),
in a scratch folder with a made-up deck, using a debug build of `qd`.

## 1. Make groups from the CLI

`qd new groups.qmd`, then six `qd add`. `qd indent m1` made Methods a head
(`changed: ["methods"]`); `qd outdent results` made Results a head
(`changed: ["results"]`). `qd slides`: methods and results are heads, m1 and
m2 are in methods, r1 is in results, first-slide and intro are top-level.
As expected.

## 2. Render matches the list

`quarto render groups.qmd`: two vertical stacks, Methods (M1, M2) and Results
(R1); the title slide, First slide and Intro stand alone. As expected. The
same check runs as a test (`quarto_stacks_match_the_groups`, in linear and
vertical mode).

## 3. Move a group, outdent, undo

`qd move results r1 --before methods`: Results and R1 come before Methods.
`qd outdent m2`: `# M2`. Two `qd undo`: the file is byte-identical to step 1.
As expected.

## 4. Navigation mode

`qd navigation vertical` added `navigation-mode: vertical` at the top level
(the deck has `format: revealjs` as a string); `qd navigation linear` removed
it and the file was byte-identical to step 1 again. As expected.

## 5. In the editor, timed (SC-002, under 10 s)

Headless Chrome on a made-up deck with Part A and nine slides after it,
measured from the key or drag to the list showing the result (the machine's
load average was about 170):

- Tab on A1: Part A became a head with A1–A9 under it in **0.1 s**.
- Folding Part A (ten slides) and dragging it below Part B: the group moved
  whole, in order, in **0.3 s**.

Both well under 10 s.
