# Research: On-Slide Editing

Decisions for spec 003. Each: Decision / Rationale / Alternatives.

## R1. How typed text and formatting become a minimal `.qmd` edit

**Decision**: Edit in the rendered element (contenteditable inside the
same-origin preview iframe). On save, the editor serializes the element's DOM
back to markdown with its own small serializer (supported subset below) and
sends the markdown through the existing `edit` op. Two guards make the result
safe regardless of serializer quality:

1. **Round-trip gate (per element, before editing starts)**: serialize the
   *unedited* element; if the result is not byte-identical to the element's
   source, the element is not typeable and opens the in-place source editor
   instead (FR-006). This decides typeable vs. source automatically and
   guarantees FR-003c (untouched constructs the serializer can't represent
   never reach a save).
2. **Minimal patch (server side, in `ops::edit`)**: the replacement is
   trimmed to the differing middle (common prefix and suffix of old and new
   source removed), so only changed bytes are written even inside the element
   (FR-002: "unchanged inline markup inside the element" stays byte-identical).

Serializer subset: text runs (escaping `* _ \` [ ] < > #` and leading `- + 1.`
where they would gain meaning), `**strong**`, `*em*`, `~~strike~~`,
`` `code` ``, `[text](url)`, `[text]{.underline}`, `[text]{.mark}`,
`[text]{style="color:…"}` / `font-size`, hard line breaks, bullet and
numbered lists (nested by indentation), block quotes, and a paragraph wrapped
in `::: {style="text-align:…"}` for alignment. Typography produced by Pandoc
(curly quotes, en/em dashes, ellipsis) is mapped back by aligning against the
original source characters, so ordinary prose passes the gate.

**Rationale**: The DOM is already there (no HTML parser dependency, no second
renderer). The gate makes correctness local and checkable: an element is
either round-trippable or it isn't, and the server-side trim makes
byte-identity independent of the browser. Rendering stays with Quarto
(spec assumption): after save the real render replaces the edited DOM.

**Alternatives**: (a) A rich-text framework (ProseMirror/TipTap) with a
markdown schema — large dependency, its own document model to keep in sync
with the deck model, still needs a source-faithful serializer. (b) Converting
in Rust from posted HTML — needs an HTML parser crate and duplicates the
browser's DOM; the CLI gains nothing from it. (c) Inline source spans
annotated into the preview copy (`[run]{data-qd-i=…}`) for exact mapping —
precise, but changes the preview DOM (layout-drift risk, spec 001 checklist)
and is far more machinery than the gate needs.

## R2. Formatting commands

**Decision**: `document.execCommand` for bold, italic, underline,
strikethrough, link, lists and indent/outdent (still implemented by Chrome,
Safari and Firefox; the serializer accepts both `<b>/<strong>`, `<i>/<em>`,
`<u>`, `<s>/<strike>/<del>`); own `Range`-wrapping for spans the commands
don't express cleanly (inline code, colour, highlight, size) so they produce
exactly one `<span class/style>` per run. Alignment applies to the whole
paragraph/list. The toolbar is a plain DOM bar positioned above the element,
with shortcuts ⌘B ⌘I ⌘U ⌘⇧X ⌘E ⌘K.

**Rationale**: Native editing gives caret, selection, IME, spellcheck and
undo-within-the-element for free; the handful of custom wraps is small.

**Alternatives**: a framework (see R1a); fully custom editing (re-implements
caret and IME behaviour).

## R3. Conflicts: element-level, not file-level

**Decision**: Browser saves send `expect_text` = SHA-256 (16 hex) of the
element's source as read when editing began, instead of the whole-file
version. The op refuses with `E_CONFLICT` only if that element's current
source differs (or the element is gone). CLI: `qd edit <el> --expect-text
<hash>`; `qd show` reports each block's `hash`. Whole-file `--expect` keeps
working as before.

**Rationale**: US5 needs an agent's edit to *another* element to leave the
person's save valid, while a change to the *same* element is refused. A
file-level version refuses both.

**Alternatives**: retry-on-conflict in the browser (silently overwrites the
agent if the same element changed — violates FR-015); per-slide hash (an agent
fixing a typo elsewhere on the slide would still block the save).

## R4. Re-renders during an edit

**Decision**: While an edit session is open, `rendered` events are queued,
not applied; the newest is applied when the session ends (save, cancel or
conflict). After a save the edited DOM stays in place (no flash of old text)
until the new render loads, normally 1–2 s (SC-004). Changes from elsewhere
show a small "updated — will refresh when you finish" badge.

**Rationale**: Today every render reloads the iframe (`load(d.html)`), which
would discard the caret and unsaved text (FR-016).

**Alternatives**: patch the changed slide's DOM in place — would need
per-slide rendering or DOM diffing; out of proportion.

## R5. Structural ops for single elements

**Decision**: New `qd block` subcommands mirroring `qd box`:
`block add <slide> (--after|--before <el> | --start | --end) --md`,
`block move <el> (--before|--after <el>)`, `block rm <el>`, `block dup <el>`.
Each is a span replacement within one slide; moved/duplicated text is copied
byte-for-byte (FR-013). Element refs are the existing `<slide>/<index>` or
`#id`.

**Rationale**: FR-014 requires a CLI equivalent for every on-slide action;
`qd edit` cannot delete or move. Agents gain the same precise operations.

**Alternatives**: overloading `qd edit` with empty text = delete (the current
error message says to use `qd body`), or doing structure via `qd body` (writes
the whole slide body; loses minimality).

## R6. Context menus

**Decision**: One menu component in the editor page, opened from
`contextmenu` events on the preview iframe's document and on the slide list;
positioned inside the window; ARIA `role="menu"`/`menuitem`, arrow keys,
Enter, Escape, Shift+F10 / ContextMenu key; disabled items shown greyed.
Each item calls an op (via `/api/op`) or a local action (edit, edit source,
copy). A single action table (id, label, shortcut, enabled(target), run)
feeds the menu, the toolbar and the keyboard handler, so 004's menu bar can
reuse it.

**Rationale**: Standard behaviour (spec US6) with one source of truth for
actions; avoids three diverging lists.

**Alternatives**: separate hand-written menus per target (drifts).

## R7. Clipboard for elements

**Decision**: Copy/Cut put the element's markdown on the clipboard as
`text/plain` (so pasting into any text editor gives the exact source) plus
`text/html` of the rendered element; Paste of plain text at an element
inserts it with `block add` after that element. Within a text edit,
paste is plain text (formatting from other apps dropped) unless it is qd's
own copy.

**Rationale**: FR-022: exact reproduction; plain text is the portable form.

## R8. Source panel, fragments, title slide, notes

- Side panel hidden by default; "Source" toggle in the header; open/closed
  remembered per browser (`localStorage`) until Settings exist (spec 004).
- While an edit session or the editor is in edit mode, all fragments on the
  current slide get `visible` so hidden content can be reached; restored on
  exit. Presenting is unaffected.
- Title slide: title, subtitle, author, date are typeable and saved with
  `qd meta set` (existing op), plain text only.
- Notes: slide toolbar/context menu "Speaker notes…" opens the `::: notes`
  block in the source editor (creates it with `block add` if missing).

## R9. Testing the browser side

**Decision**: Drive the real editor in headless Chrome through
`chromiumoxide` (already a dependency from spec 002) in Rust integration
tests: open `qd serve`, run scripted edits (focus element, insert text, apply
toolbar commands, save) and assert on the `.qmd` bytes. A serializer
round-trip test runs the gate over every block of a corpus of ≥ 20 decks
(SC-002/SC-003) and reports the typeable share.

**Rationale**: No JS test toolchain to add; tests exercise the real DOM and
the real ops.

**Alternatives**: a JS unit-test runner (Node + jsdom) — new toolchain, and
jsdom's contenteditable differs from browsers.
