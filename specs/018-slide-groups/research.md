# Research: Slide groups

Facts checked by rendering small made-up decks with Quarto 1.11 on
2026-11-17, and by reading qd's own code.

## R1. How Quarto groups slides

**Decision**: a group is what Quarto already renders as one reveal.js
vertical stack. Its head is a section slide at the deck's **top section
level**: the smallest heading level among headings above `slide-level`. With
the default `slide-level: 2` that is `#`. Its sub-slides are every slide
after it up to the next heading at that level.

**Checked**:
- `# Part A` then `## A1`, `## A2` renders as
  `<section><section id="part-a" class="title-slide slide level1 center">`
  followed by the two `level2` sections inside the same outer `<section>`.
  This happens in the default `navigation-mode: linear` too.
- `## Intro` before the first `#` renders as its own top-level `<section>`,
  outside any stack.
- A `---` slide after `## A1` joins Part A's stack.
- With `slide-level: 3`, `#` and `##` both make section slides, and the
  stacks are the `#` sections: `## S` sits inside `# P`'s stack with the
  `###` slides.
- With `slide-level: 3` and no `#` at all, the stacks are the `##`
  sections. So "top section level" is the smallest level present, not
  always 1.
- An empty `#` heading (or `# {#named}`) renders an empty section slide, so
  a head always shows as a slide; there is no invisible group head.

**Alternatives considered**: a qd attribute marking groups (rejected with
the person: not native, and the render would not match the list).

## R2. Changing a slide's level

**Decision**: indent and outdent patch only the heading marks of the slides
involved, inside each slide's `head` span:
- ATX (`## Title {#id}`): replace the leading run of `#`. Title, attributes
  and spacing stay byte for byte.
- Setext (`Title` underlined): swap `=` and `-` when the new level is 1 or
  2, keeping the underline's length. A setext heading going to level 3 or
  more is rewritten as ATX (setext has no level 3).
- A `---` slide that becomes a head: its rule line becomes the top-level
  marks alone (`#`), giving an empty heading, which Quarto renders as an
  empty section slide (R1). The editor says so.
- An implicit slide (content before any heading) can't become a head:
  refused with "give the slide a title first".

Auto ids don't change: Quarto derives them from the title text, not the
level. Explicit ids are inside the attributes, which aren't touched.

**Why not `Ctx::rewrite`**: it rebuilds the whole heading line, which turns
setext into ATX and re-renders attributes. A marks-only patch is smaller
and keeps the person's formatting (constitution II).

## R3. Indent and outdent rules

**Decision** (P = the slide just above X in deck order):

| X is | Indent | Outdent |
|---|---|---|
| a head | X → slide level; its sub-slides join P's group. If P is a top-level slide, P also becomes a head. | refused: already at the top |
| a sub-slide | refused: one level only | X → top level; the sub-slides after it become its sub-slides |
| a top-level slide (before the first head) | P → head (top level); X stays at slide level | refused |
| the first slide, or after only the title slide | refused: nothing above | — |

With `slide-level: 1` there are no groups: both are refused with that
reason. With deeper slide levels, indent sets the slide level and outdent
sets the top section level; section slides at the levels in between are
sub-slides and keep their level until outdented.

## R4. Moving a group, and landing indented

**Decision**:
- `qd move` already moves several slides in deck order as one patch, so a
  group move is `qd move <head> <sub-slides…> --after <x>`. The editor
  expands a collapsed head into its members in one place, the list's
  `selected(id)`, which drag, copy, cut and Delete already go through.
  No new command for "move the group".
- A drop that also changes depth must be one undo step, so `qd move` gains
  `--indent` and `--outdent`: after placing the slides, apply R3 to the
  first moved slide in the same patch (its own heading, plus P's when P
  becomes a head).
- The editor, not the command, keeps a moved group (or an open head moved
  alone, or a slide dropped outdented) to places where the next slide is a head or the end of the deck
  (FR-017): a `#` dropped elsewhere would take the slides after it into
  its group. The drop marker skips the other places. On the command line, `qd move` does what the file then means.

- `qd dup` today takes one slide. It takes several, in deck order, as one
  patch (like `move` and `rm`), so a collapsed group's Duplicate is one
  command and one undo step. Paste and duplicate of a group go after the
  end of the target's group, chosen by the editor (FR-017).

**Alternatives considered**: a `--group` flag on `move`/`rm` (rejected:
the members are in `qd slides`, and two ways to say the same thing is one
too many); two ops per drop (rejected: two undo steps).

## R5. Collapse state

**Decision**: `view.json` in the deck's own qd cache folder (where its
undo journal already lives), holding `{"collapsed": [head ids]}`. The
editor reads it with `GET /api/view` and writes it with `POST /api/view`
(the run token, like `POST /api/settings`; a body over 64 KB or not a JSON
object is refused). Ids of heads that are gone are dropped on the next
write.

**Why**: it is view state for this machine and must never reach the
`.qmd` (constitution II). The browser's `localStorage` was the first
choice, but `qd serve` takes a new port each run and `localStorage` is kept
per origin (port included), so it would be empty after every restart
(found while building PR 2). qd's settings file is per user and shared by
every deck, so it is the wrong place for per-deck view state. The cache
folder is per deck, on this machine, and the browser and VS Code's panel
read the same file.

## R6. Navigation mode

**Decision**: `qd navigation linear|vertical|grid`.
- If the deck already has `navigation-mode` (top level or under
  `format: revealjs:`), that line's value is replaced, or the line removed
  for `linear`.
- Otherwise, if the front matter has a `format: revealjs:` mapping, the
  line is added at the end of that mapping with its indentation; otherwise
  it is added as a top-level key, with the existing `meta set` patch.

**Checked**: a top-level `navigation-mode: vertical` with
`format: revealjs` renders `navigationMode: 'default'` (reveal.js's name
for vertical stacks), so Quarto reads the top-level key. A key under
`format: revealjs:` overrides a top-level one, which is why an existing
line is edited where it is.

**Editor**: Slide menu ▸ "Presentation navigation" ▸ Linear / Vertical /
Grid, checked by the deck's current mode, like the Transition submenu. The
deck JSON gains `navigation_mode`.

## R7. Tab in the slide list

**Decision**: Tab indents and ⇧Tab outdents the focused slide, as the
person chose, and ⌘] / ⌘[ do the same. When the action isn't possible, Tab
does nothing in the list and the editor says why, so the key never does
two different things. Keyboard users leave the list with Escape, which
returns focus to the canvas (the list already handles Escape for its
drag).

**Trade-off**: taking Tab inside a composite widget is normal (Keynote's
list, outliners), but it means Tab no longer walks out of the list. This
is noted for the person's review.

## R8. Indent while dragging

**Decision**: during a list drag, the landing depth is set by the
pointer's horizontal offset from where the drag began: more than one
indent width (16 px) to the right lands indented, more than 16 px to the
left lands outdented, otherwise the slide keeps its depth. The drop
marker is drawn at the landing depth. In the narrow layout (a horizontal
strip) there is no depth by drag; Tab and the menu still work.
