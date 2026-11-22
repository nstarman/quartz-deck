# Feature Specification: Diagram source follows the cursor

**Feature Branch**: `021-dot-source-follow`

**Created**: 2026-11-20

**Status**: Draft

**Input**: User description: "Plan for an edition where, depending on the location of the cursor, I would like it to either or both show a preview on hover over in the graph viz of the element and highlight in the slide the object it's selecting. So for example, the data, labeled data, right? If you have your cursor in it, then it gives a little preview in source and highlights in slide."

## Context

A `{dot}` diagram cell (spec 015) is text: one line per box
(`data [label=Data, pos="0,-0.5!", shape=box, …]`), one line per arrow
(`data -> tool;`), and a few lines for the whole diagram
(`layout=neato; splines=ortho;`). In the source pane the presenter sees only
that text. To find which box a line describes, or what a change to its
attributes looks like, they have to look across at the slide and match names
by eye, then wait for the slide to redraw.

The source pane already follows the slide in a coarse way (spec 005): with
the text cursor in a cell, the slide view goes to that slide and selects
the whole diagram. This feature goes one level finer, to the box or arrow
the cursor is in.

Decisions made with the person (2026-11-20):

- The preview shows **that box (or arrow) alone**, drawn as Graphviz draws
  it. It does not show a thumbnail of the whole diagram.
- **The text cursor and the mouse** both set it. The text cursor sets it
  while typing or moving with the keys. The mouse, resting over another
  statement, shows that one for as long as it stays there.
- On the slide the element is **highlighted only**. The slide's selection,
  and so what the bars and the inspector act on, stays as it is.

## User Scenarios & Testing *(mandatory)*

### User Story 1 — The slide shows which box a line is (Priority: P1)

The presenter puts the text cursor on the line `data [label=Data, …]` in
the source pane. On the slide, the box labelled "Data" in that diagram gets
an outline. They move down to `tool [label=Tool, …]`, and the outline moves
to "Tool". On `data -> tool;` the arrow between them is outlined. When they
move to a line outside the diagram, the outline goes.

**Why this priority**: matching a line to a box is the most frequent
question, and the slide is already showing the diagram. It is useful with
no preview at all.

**Independent Test**: open a deck with a diagram in the source pane, put
the cursor on each node and edge line in turn, and check that exactly the
matching box or arrow on the slide is outlined, and that nothing is
outlined once the cursor leaves the cell.

**Acceptance Scenarios**:

1. **Given** the cursor in a node statement, **When** it rests there, **Then** that node's box on the slide is outlined and no other element is.
2. **Given** the cursor in an edge statement `a -> b`, **When** it rests there, **Then** the arrow from `a` to `b` is outlined. **Given** a chain `a -> b -> c` with the cursor on `b -> c`, **Then** that link is outlined; with the cursor elsewhere in the statement, every arrow of the chain is.
3. **Given** a box is outlined, **When** the cursor moves to a whole-diagram line (`layout=neato;`, `node [shape=box];`) or out of the cell, **Then** the outline goes.
4. **Given** something else is selected on the slide (an arrow, a text box), **When** the cursor moves through diagram lines, **Then** that selection and its bars stay as they were.
5. **Given** the mouse rests over another node line while the text cursor is in the first, **When** it rests there, **Then** that node is outlined instead. **When** the mouse leaves, **Then** the outline goes back to the text cursor's node.

---

### User Story 2 — A preview of the box beside the line (Priority: P2)

With the cursor on `data [label=Data, shape=box, fillcolor="#eef", …]`, a
small picture of the "Data" box appears beside the line in the source
pane, drawn with its shape, label, colours and picture. The presenter
changes `fillcolor` and the picture changes as they type, before the slide
has redrawn. On an arrow line, the preview shows that arrow with its two
end boxes.

**Why this priority**: it answers "what will this look like" without
looking away or waiting, but needs Story 1's mapping from line to element
first.

**Independent Test**: put the cursor on a node line, check that the
preview matches that box on the slide, then edit one attribute and check
that the preview follows within the time in SC-002, while the slide hasn't
changed yet.

**Acceptance Scenarios**:

1. **Given** the cursor in a node statement, **When** it rests there, **Then** a preview of that one box appears beside the cursor's line without covering the line itself.
2. **Given** the preview is showing, **When** the presenter changes an attribute of that statement, **Then** the preview redraws with the change.
3. **Given** the statement can't be drawn mid-typing (an unclosed quote or bracket), **When** the presenter keeps typing, **Then** the last good preview stays, marked as out of date, until the text can be drawn again.
4. **Given** the diagram sets defaults for every box (`node [shape=box, fontname=…]`), **When** a node's preview is drawn, **Then** it uses those defaults, as the slide does.
5. **Given** the preview is showing, **When** the presenter presses Esc or the cursor leaves the statement, **Then** the preview closes.

---

### User Story 3 — The same in the cell's own editor on the slide (Priority: P3)

Editing a diagram's source over the slide (the cell editor that opens on
the slide, spec 015) works the same: the cursor's box is outlined in the
diagram underneath, and the preview shows beside the line.

**Why this priority**: the source pane is where long diagrams are edited;
the editor on the slide is for quick changes.

**Independent Test**: open a diagram cell's source on the slide, move the
cursor through node lines, and check the outline and the preview as in
Stories 1 and 2.

**Acceptance Scenarios**:

1. **Given** the cell editor is open on the slide, **When** the cursor rests on a node line, **Then** that node is outlined in the diagram it covers, and the preview shows beside the line.

### Edge Cases

- A node named in several statements (declared once, then used in edges):
  only the declaration counts as that node's line. An edge line names its
  arrow, not its end boxes.
- A node that is only ever named in an edge (no statement of its own):
  outlined when the cursor is on its name inside that edge statement.
- Names with quotes or spaces (`"my box" [label=…]`) and HTML labels
  (`<<b>Data</b>>`) are matched like any other.
- Subgraphs and clusters: with the cursor on a `subgraph cluster_x {` line,
  the cluster's frame is outlined; inside, each statement works as usual.
- Two diagrams on one slide: only the one whose cell the cursor is in is
  used.
- The slide isn't drawn yet, or is redrawing: no outline until it is, then
  the outline appears for wherever the cursor is by then.
- Selecting several lines: the outline covers every box and arrow they
  touch; no preview is shown.
- An element the slide draws but the source no longer has (just deleted):
  never outlined.
- Speaker view, presenting and exported files: nothing shows. This exists
  only while editing.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The editor MUST find which node, arrow, cluster or
  whole-diagram statement of a `{dot}` cell the text cursor is in, in the
  source pane and in the cell editor on the slide.
- **FR-002**: With the cursor in a node, arrow or cluster statement, the
  editor MUST outline that element in the cell's diagram on the slide, and
  only that element. Arrows in a chain follow the link under the cursor
  (acceptance scenario 1.2).
- **FR-003**: The outline MUST NOT change the slide's selection, the bars,
  the inspector or the undo history, and MUST NOT be written to the deck.
- **FR-004**: With the cursor in a node or arrow statement, the editor MUST
  show a preview of that element alone beside the line. It is drawn by the
  same Graphviz engine the slide uses, with the diagram's defaults and the
  statement's attributes as they are typed.
- **FR-005**: The preview MUST NOT cover the cursor's line, MUST stay within
  the window, and MUST close on Esc, when the cursor leaves the statement,
  or when the pane loses focus.
- **FR-006**: The mouse resting over a statement (as a hover) MUST show
  that statement's outline and preview while it rests there, and give back
  to the text cursor's statement when it leaves.
- **FR-007**: A statement that can't be drawn mid-typing MUST keep the last
  good preview, marked as out of date, and MUST NOT show an error over the
  text.
- **FR-008**: The outline and the preview MUST follow the cursor without
  waiting for the deck to render again, and MUST NOT slow typing.
- **FR-009**: Whole-diagram statements (`graph`/`node`/`edge` defaults,
  `layout=…`), comments and blank lines MUST show no outline and no
  preview.
- **FR-010**: A setting MUST turn the preview off (the outline stays). It is
  on by default.
- **FR-011**: Nothing from this feature MUST appear when presenting, in the
  speaker view, or in rendered or exported output.

### Key Entities

- **Statement under the cursor**: the node, edge (and link within it),
  cluster or whole-diagram line of a `{dot}` cell that holds the cursor or
  the resting mouse; it names one element of the drawn diagram.
- **Outline**: a temporary mark on that element in the slide's diagram.
- **Preview**: a small drawing of that element alone, beside the source
  line.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: In a diagram of 20 boxes and 25 arrows, putting the cursor on
  any node or arrow line outlines the matching element on the slide, and
  only it, every time.
- **SC-002**: The outline and the preview follow a cursor move or a typed
  change within 100 ms of the cursor coming to rest, on the 20-box diagram.
- **SC-003**: Typing in a diagram cell is no slower with this feature on
  than off. Each keystroke appears within one frame, measured over 200
  keystrokes.
- **SC-004**: A node's preview and the same box on the slide have the same
  shape, label, colours and picture in each of the deck's diagrams.
- **SC-005**: On the person's own talk diagram, they can say which line is
  which box without looking at the names, and adjust a box's look from the
  preview alone. The person tries this by hand.

## Assumptions

- The editor already carries the Graphviz engine that Quarto uses (spec
  020), so it can draw one element by itself in milliseconds.
- The drawn diagram on the slide names each box and arrow after the
  statement it comes from, the way Graphviz's output does. Spec 015's
  on-slide editing already relies on this.
- Source pane hover means the pointer resting over the text for a moment,
  as tooltips do. It does not mean while the mouse moves past.
- The preview is a picture of the element. Clicking it does nothing in
  this spec.
- Other code cells (Python, Mermaid) are out of scope.
