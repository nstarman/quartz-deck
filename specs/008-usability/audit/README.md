# Usability audit (008), 2026-11-06

The editor has the features. This audit looks at how it feels to use, surface
by surface, before any 008 spec or code. Each finding says where, what is
wrong, which principle it breaks, the evidence, a proposed change and a
priority. The findings become the 008 spec's stories.

**How it was done.**
- **Screenshots:** [`shoot.py`](shoot.py) serves [`deck/talk.qmd`](deck/talk.qmd)
  (a realistic ten-minute talk: title, bullets, an image, columns, math with a
  callout, a positioned box, a code cell, an empty slide) with fresh
  settings. It drives the release build with headless Playwright, in light,
  dark and high contrast at 1280×800, at 420 px wide, and as VS Code's panel
  (71 screenshots).
  Files are in [`shots/`](shots/). Rerun with
  `uv run specs/008-usability/audit/shoot.py`.
- **Measurements:** contrast ratios, control sizes and font sizes were taken
  from the live editor's computed styles (light and dark).
- **Tasks:** eight common tasks, with the clicks each takes today.
- **Not covered here:** screen readers (beyond focus and labels), Firefox and
  Safari, touch. The script doesn't capture the code-cell source pop-up (a
  click on a code cell didn't open it headlessly); the person's own
  screenshot of it (2026-11-06) stands in.

**Reference: Apple's Human Interface Guidelines**, read on 2026-11-06 for their
principles; our design stays our own (constitution I). The parts this audit
uses:
- [Accessibility](https://developer.apple.com/design/human-interface-guidelines/accessibility):
  macOS controls 28×28 pt by default, 20×20 pt minimum; text contrast at least
  4.5:1 (3:1 for 18 pt and larger, or bold), checked in light and dark; macOS
  body text 13 pt by default, 10 pt minimum; "consider spacing between
  controls as important as size".
- [Layout](https://developer.apple.com/design/human-interface-guidelines/layout):
  order content by importance; align and group related items; progressive
  disclosure; differentiate controls from content; adapt to window sizes,
  keeping functionality the same.
- [Drag and drop](https://developer.apple.com/design/human-interface-guidelines/drag-and-drop):
  show a translucent drag image; show an insertion point only where a drop is
  allowed; give clear, continuous feedback; offer menu alternatives; let
  people undo.
- [Feedback](https://developer.apple.com/design/human-interface-guidelines/feedback):
  integrate status into the interface; match the weight of feedback to its
  significance; say why a command can't be carried out.

## The ten that matter most

| # | Finding | Priority |
|---|---|---|
| 1 | Slides can only be added from one button at the end of the list, which then inserts after the *current* slide ([L1](#l1)) | P1 |
| 2 | Dragging a slide shows no handle, no drag image and no insertion marker ([L2](#l2)) | P1 |
| 3 | The slide list is text only; slides are recognised by a hover pop-up over the canvas ([L3](#l3)) | P1 |
| 4 | The element and formatting bars cover the slide's own title ([C1](#c1)) | P1 |
| 5 | Dark theme: the Transition menu is unreadable (1.18:1) and source colours fail contrast ([T1](#t1), [T2](#t2)) | P1 |
| 6 | No Present button: presenting is File ▸ Open preview ↗ ([H2](#h2)) | P1 |
| 7 | Nine plain browser `prompt()`/`confirm()`/`alert()` boxes, including typing an image path by hand ([D1](#d1)) | P1 |
| 8 | Several controls are below the 20 pt minimum: ▶ Run (17 px tall), box handles (8 px), Hidden checkbox (13 px) ([T3](#t3)) | P2 |
| 9 | The header tells you a version hash, not which deck this is ([H1](#h1)) | P2 |
| 10 | reveal.js's own menu button and progress bar sit inside the editing canvas ([C2](#c2)) | P2 |

## Findings by surface

### Window and header

<a id="h1"></a>**H1. The header names the app, not the deck; status is a hash.** P2
- *What:* the header reads "quartz-deck … rendered b74bad8"; the browser tab
  is "quartz-deck" for every deck. With two decks open you can't tell them
  apart, and "rendered b74bad8" means nothing to a person.
- *Principle:* feedback (status integrated, in the person's terms); layout
  (most important first).
- *Evidence:* [light-02](shots/light-02-content.png); `document.title` is
  "quartz-deck".
- *Change:* show the deck's title (and file name, smaller) in the header and
  the tab title. Status in words: "Saved", "Rendering…", "Up to date",
  "Changed on disk — reloading"; the version hash only in a tooltip.

<a id="h2"></a>**H2. No Present.** P1
- *What:* the one thing every slide app puts first, playing the deck, is
  File ▸ Open preview ↗ (two clicks, and "preview" doesn't say "present").
- *Principle:* order by importance; clarity of labels.
- *Change:* a ▶ Present button in the header (⌥⌘P), presenting from the
  current slide, and a "from the beginning" choice.

<a id="h3"></a>**H3. Two full-width bars for little content.** P3
- *What:* the menu bar (29 px), the header (47 px) and the slide bar (28 px)
  take 104 px above the slide at 800 px tall; the header holds only the name,
  status and Source. In VS Code's panel, a whole row holds a lone ☰
  ([vscode-01](shots/vscode-01-panel.png)).
- *Change:* one top bar: menus (or ☰), deck title, status, Present, Source.

<a id="h4"></a>**H4. Jargon in names.** P2
- *What:* "Render", "Open preview", "Export offline HTML", "Export with PDF",
  "+ Package", "Create environment", "Form", "On click", "Source" (both the
  pane and the element's markdown), "Edit source".
- *Change:* say what happens: "Present", "Export ▸ Web page (works offline) /
  PDF", "Add Python package…", "Properties"; keep "Source" for the pane and
  call the element's one "Edit markdown".

### Slide list

<a id="l1"></a>**L1. Adding a slide.** P1 (the person's)
- *What:* "+ Add slide" sits at the end of the list but inserts after the
  current slide, and is the only visible way. New slides are titled "New
  slide" in the file, so two additions make two identical titles.
- *Principle:* direct manipulation; feedback (where will it go?).
- *Evidence:* [light-02](shots/light-02-content.png); `#add-slide` handler.
- *Change:* a + between slides that shows on hover (and from the keyboard: ⏎
  on a slide in the list adds one after it); a + after the last slide; a
  layout choice when adding (with 009); new slides start untitled with a
  placeholder shown on the slide ("Click to add title"), not text in the file.

<a id="l2"></a>**L2. Dragging slides.** P1 (the person's)
- *What:* you drag by the row (number and title); while dragging, the slide
  fades and the row under the pointer gets a faint border. No handle, no drag
  image, no gap opening where it will land.
- *Principle:* drag and drop: translucent drag image, insertion point only
  where a drop is allowed, continuous feedback.
- *Evidence:* [light-11](shots/light-11-slide-list-drag.png).
- *Change:* a ⠿ handle on hover (the row stays draggable); a thumbnail drag
  image; an insertion line with the rows parting to make room; auto-scroll
  near the list's edges; Esc cancels; ⌥ while dropping duplicates. Keyboard:
  ⌥↑/⌥↓ on a selected slide.

<a id="l3"></a>**L3. No thumbnails.** P1
- *What:* the list shows a number and title; seeing a slide means hovering for
  a large pop-up that covers the canvas you're working on.
- *Principle:* recognition over recall; layout (don't cover content).
- *Evidence:* [light-09](shots/light-09-slide-list-hover.png).
- *Change:* small thumbnails in the list (the renders already exist), title
  under each; the hover pop-up goes.

<a id="l4"></a>**L4. The title slide isn't in the list.** P2
- *What:* the deck opens on the title slide, but the list starts at slide 1
  ("Highlights"); nothing is highlighted, and the title slide can't be chosen
  from the list.
- *Evidence:* [light-01](shots/light-01-start.png).
- *Change:* list the title slide first (its title and subtitle come from the
  front matter; editing them edits the YAML).

<a id="l5"></a>**L5. Multi-select.** P3
- *What:* one slide at a time for move, delete, copy.
- *Change:* ⇧-click and ⌘-click select ranges and sets; actions apply to all.

### Canvas (the slide)

<a id="c1"></a>**C1. Bars cover the title.** P1
- *What:* selecting the first element under the title puts the element bar
  (Edit · Source · On click · Delete · ···) over the title; editing puts the
  24-button formatting bar there.
- *Principle:* layout: controls must not hide content.
- *Evidence:* [light-05](shots/light-05-selected.png),
  [light-06](shots/light-06-editing-text.png).
- *Change:* place the bar above the element only where it doesn't cover
  another element; otherwise below it, or in a fixed toolbar strip above the
  canvas that changes with the selection (as a format inspector).

<a id="c2"></a>**C2. reveal.js's own controls in the canvas.** P2
- *What:* reveal's ☰ menu button (bottom left) and progress bar show while
  editing; the ☰ looks like our own folded menu.
- *Evidence:* every canvas shot.
- *Change:* hide reveal's menu button, controls and progress bar in the editor
  (they stay in the presented deck).

<a id="c3"></a>**C3. The canvas jumps.** P2
- *What:* the slide bar (Background · ∅ · Image… · Transition · Hidden · ···)
  appears only on content slides, so the canvas moves down 34 px when you
  leave the title slide.
- *Evidence:* [light-01](shots/light-01-start.png) vs
  [light-02](shots/light-02-content.png).
- *Change:* keep the bar's space always (disabled where it doesn't apply), or
  move slide properties into an inspector.

<a id="c4"></a>**C4. Slide bar controls.** P2
- *What:* a 28×22 colour swatch, an unlabeled "∅" (clears the colour), a
  native select, a 13×13 checkbox and "···", at three heights.
- *Change:* one control height (28 pt); "∅" becomes a "None" choice inside the
  colour menu; labels on everything or tooltips that say what each does.

<a id="c5"></a>**C5. Insert lacks the deck's own building blocks.** P2
- *What:* Insert offers paragraph, list, equation, image, text box; not code
  cell, callout, columns, speaker notes, table, quote.
- *Change:* add them (each is one markdown snippet), with a short preview.

<a id="c6"></a>**C6. Hover handles.** P3
- *What:* + and ⠿ beside a hovered element are 26×26 (above the 20 minimum,
  below the 28 default), and "+" doesn't say before or after.
- *Change:* 28 pt; "+" shows "Insert below" as a tooltip and a line where it
  will go.

### Text editing

<a id="e1"></a>**E1. The formatting bar.** P2
- *What:* 24 buttons of about 26 px: B I U S, code, link, bullets, numbers,
  outdent, indent, eight unlabeled colour dots, ∅, A−, A+, three alignment
  icons that look alike.
- *Principle:* progressive disclosure; group related items.
- *Evidence:* [light-06](shots/light-06-editing-text.png).
- *Change:* the common few up front (B I U S, link, list, colour); the rest in
  a "Aa" pop-over; colours named in tooltips; alignment as a three-way
  segmented control with clear icons.

<a id="e2"></a>**E2. Where the hint lives.** P3
- *What:* "Editing — ⌘Enter, Tab, Esc or click elsewhere saves" appears in the
  header, far from the text being typed.
- *Change:* none needed while editing (it saves); show a brief "Saved" near
  the element when it does.

<a id="e3"></a>**E3. Code cells and Run.** P2 (→ 012)
- *What:* the source pop-up on a code cell offers save and Cancel only; no Run
  (the person's screenshot).
- *Change:* 012 adds Run and ⇧Enter there; until then, at least the cell's
  right-click "Run cell" should also be in the pop-up.

### Positioned boxes

<a id="b1"></a>**B1. Handles are 8 px.** P2
- *What:* resize handles are about 8×8 px; the box shows no move cursor until
  hovered exactly.
- *Principle:* minimum control size (20 pt).
- *Evidence:* [light-19](shots/light-19-positioned-box.png).
- *Change:* 10 px visible squares with a 20 pt hit area; alignment guides
  while moving (009 covers snapping).

### Source pane

<a id="s1"></a>**S1. The run bar.** P2
- *What:* eight equally weighted buttons in two rows (Run all, Stop, Restart,
  Restart & run all, + Package, Create environment, Full width, Form), two
  heights (23 and 28 px), and the session state as 11 px grey "stopped".
- *Principle:* progressive disclosure; order by importance; feedback.
- *Evidence:* [light-12](shots/light-12-source-pane.png).
- *Change:* Run all ▶ and Stop ■ as icons with the session state beside them
  ("Python · idle"); Restart, Restart & run all, packages and environment in a
  "···" menu; Full width and Properties as view toggles in the pane's corner.

<a id="s2"></a>**S2. ▶ Run buttons are 17 px tall.** P2
- *Evidence:* measured 31×17 and 23×17 in the gutter.
- *Change:* at least 20 pt tall, aligned with the cell's first line.

### Dialogs and prompts

<a id="d1"></a>**D1. Plain browser boxes.** P1
- *What:* `prompt()` asks for the image path, link address, background colour,
  background image, custom class and Python packages; `confirm()` for delete
  slide and reset settings; `alert()` for About. They look foreign, can't
  validate, and the image path must be typed.
- *Principle:* consistency; feedback (say why something can't be done).
- *Change:* our own small sheets: an image picker (files beside the deck, plus
  drag and drop of an image file onto the slide), a link field with
  validation, the colour as a swatch menu, a package field that shows what it
  will add. Delete slide doesn't ask at all (undo restores it), as now when
  the setting is off.

<a id="d2"></a>**D2. Settings.** P3
- *What:* each select is as wide as its text, so the right edge is ragged;
  Theme, likely the most used, is fourth.
- *Evidence:* [light-13](shots/light-13-settings.png).
- *Change:* one width for the controls; Theme first.

### Themes and contrast

<a id="t1"></a>**T1. Dark: the Transition menu is unreadable.** P1
- *Measured:* 1.18:1 (light grey on white) in dark; minimum 4.5:1. In high
  contrast the value is white on white. The native select isn't themed.
- *Evidence:* [dark-06](shots/dark-06-editing-text.png),
  [hc-06](shots/hc-06-editing-text.png), [vscode-01](shots/vscode-01-panel.png).
- *Change:* theme every form control (`color-scheme` and explicit colours).

<a id="t2"></a>**T2. Source colours are fixed, not themed.** P1
- *Measured:* on the dark pane, fences `#7d3c98` 2.36:1, YAML `#8a6d3b`
  3.43:1, code `#1e8449` 3.53:1; on white, `#b7950b` 2.87:1. All need 4.5:1.
- *Change:* a light and a dark set of source colours, each checked.

<a id="t3"></a>**T3. Controls under the minimum size.** P2
- *Measured:* ▶ Run 17 px tall; box handles 8 px; Hidden checkbox 13 px;
  Transition select 19 px tall; menu bar items 24 px tall. Minimum 20 pt,
  default 28 pt.
- *Change:* covered by C4, S2, B1; menu bar items 28 px.

<a id="t4"></a>**T4. Muted text.** OK
- *Measured:* status and slide numbers 5.3:1 (light) and 5.67:1 (dark); body
  text 13–14 px. These pass.

### Narrow windows and VS Code

<a id="n1"></a>**N1. The panel's lone ☰ row.** P2
- *What:* in VS Code (and when folded), the ☰ takes a whole 28 px row above
  the header.
- *Evidence:* [vscode-01](shots/vscode-01-panel.png),
  [narrow-01](shots/narrow-01-start.png).
- *Change:* put ☰ at the start of the header row (H3).

<a id="n2"></a>**N2. A narrow window.** P2
- *What:* at 420 px the menus still fit (so nothing folds), the slide list
  turns into a horizontal strip cut off at the edge, the slide bar wraps onto
  two rows with "Background" against the left edge, and the slide sits in a
  tall frame that is mostly empty.
- *Evidence:* [narrow-01](shots/narrow-01-start.png).
- *Change:* below about 600 px, the list collapses to a slide picker in the
  top bar and slide properties move into the ··· menu; the canvas keeps the
  slide's aspect ratio without the empty band.

<a id="t5"></a>**T5. High contrast keeps the blue selection.** P3
- *What:* the selection and editing outlines on the slide are a fixed blue,
  not the theme's yellow accent.
- *Evidence:* [hc-06](shots/hc-06-editing-text.png).
- *Change:* the preview's outline colours follow the theme.

## Common tasks today

| Task | How, today | Clicks | Issue |
|---|---|---|---|
| Add a slide after slide 3 | select slide 3, "+ Add slide" (at the end of the list) | 2 | the button's place says "at the end" (L1) |
| Move slide 6 to second | drag by the row, or Slide ▸ Move up ×4 | 1 drag / 8 | no handle or marker (L2) |
| Add an image | Insert ▸ Image…, type a path | 2 + typing | no picker, no drop (D1) |
| Bold a phrase | click the text, select, ⌘B | 1 + select | fine |
| Change the background | slide bar swatch | 1–2 | ∅ unexplained (C4) |
| Present | File ▸ Open preview ↗ | 2 | hidden, misnamed (H2) |
| Run a cell | open Source, ▶ on the cell; or right-click ▸ Run cell | 2 | not in the code pop-up (E3) |
| Export | File ▸ Export offline HTML | 2 | jargon (H4) |

## What works

Worth keeping as it is: the menus (complete, with shortcuts, ARIA), the
right-click menus (clear labels, disabled items stay in place), "+ Add content"
on an empty slide, the shortcut reference with search, Settings' plain
descriptions, focus rings in all three themes, and the slide list's current
slide marking.

## Next

The P1 findings become 008's first user stories (slide list: add anywhere,
drag, thumbnails; canvas bars; contrast; Present; our own sheets), P2 the
next, P3 polish. Each keeps every action in the action list (007) and every
edit a `qd` command.
