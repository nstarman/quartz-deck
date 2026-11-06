# Research: App shell (007)

Decisions for the plan. Each was checked against the code as of `main`
af2bf00.

## R1. One action list, as data both sides read

**Decision**: the actions live in one JSON file, `src/ui/actions.json`: id,
label, menu path, keys per platform (`mac`, `other`, and `vscode` where the
panel needs a different key or none), the places the key works (`where`:
anywhere, slide, element, text, source), and the matching `qd` command. The
server serves it to the editor as a one-line module, `/ui/actions.js` =
`export default <the JSON>;`. JSON module imports (`with { type: 'json' }`)
need Chromium 123, and VS Code 1.90's Electron ships 122. The editor maps ids
to its own handler functions. The binary embeds the same file
(`include_str!`) for `qd actions`. One keydown dispatcher in the editor,
built from the list, replaces the scattered key checks.

**Rationale**: the menus, the right-click menus, tooltips, the reference,
key handling and `qd actions` cannot drift if they all read one file. JSON
because Rust and the browser both read it with no build step (constitution
VII). Handlers stay in JS next to the code they call.

**Alternatives**: a JS module as the source (Rust can't read it); a Rust
table served as JSON (the UI would wait on a fetch before it could handle a
key, and the labels would live far from the UI); keeping `actions.js`'s
inline `shortcut:` strings (display only; no dispatch, no CLI).

**Check that nothing drifts**: a browser test asserts every action has a
handler, and that every listed key runs its action. A Rust test scans
`src/ui/*.js` for key checks (`e.key ===`, `key ===`) outside the
dispatcher. A short allowlist covers keys that are part of text editing
itself (arrows in menus, Enter/Tab inside a list being typed), each with its
reason.

## R2. Settings: a per-user file, served by qd

**Decision**: settings are stored in `settings.json` in qd's per-user config
folder: `$QD_CONFIG_DIR`, else `~/Library/Application Support/quartz-deck` on
macOS, else `$XDG_CONFIG_HOME/quartz-deck` or `~/.config/quartz-deck`. This
mirrors `store::cache_root`. The file is mode 0600 and written atomically.
The server serves the file at `GET /api/settings`, with each setting's
definition. It changes them with `POST /api/settings`, which takes a partial
object, checks it against the definitions and requires the token. Other open
tabs learn of a change from an SSE `settings` event. The definitions (type,
allowed values, default, label) live in Rust (`src/settings.rs`), so the CLI
and the server check values the same way. The UI's remembered layout (pane
width, source pane open, full width, form panel) moves from `localStorage`
into the same file under `ui.*` keys, which the window does not show.

**Rationale**: `localStorage` belongs to the page's origin, and `qd serve`
picks a new port each run. So today's preferences are forgotten between
sessions, and VS Code's panel never sees them (spec US2). A file belongs to
the person on this machine, and every qd server reads the same one. Over
Remote-SSH it lives where qd runs, which is the assumption in the spec.

**Alternatives**: keep `localStorage` with a fixed port (that breaks two
servers at once); store settings in the deck or in its cache (they would
differ per deck, against FR-008); a cookie (still bound to the origin).

## R3. CLI for settings and actions

**Decision**: `qd actions` prints the action list as JSON (FR-015). `qd
settings` prints values and definitions; `qd settings set KEY VALUE` and `qd
settings reset` change them, with the same checks as the server. A running
server notices the file changed (it already watches files) and emits the
SSE event, so a change from the CLI also applies live.

**Rationale**: constitution III. Agents and later features (011's "save as
git commit") need to read and set settings without the editor.

## R4. Menus: the existing menu code, plus a menu bar

**Decision**: `src/ui/menu.js` already renders keyboard-driven menus with
submenus for right-click. Add a menu bar on top of it that follows the WAI-ARIA
menubar pattern: `role=menubar`/`menu`/`menuitem`/`menuitemcheckbox`; F10 or
Alt alone focuses it; arrows move; Enter/Space runs; Escape closes and
returns focus; type-ahead. Disabled items get `aria-disabled` and stay in
place. The right-click menus become filtered views of the same list.
Below a fitting width, and when framed in VS Code, the bar folds into one ☰
button that opens the same menus as a drop-down.

**Rationale**: reuse (ponytail). The browser has no native menu bar for
pages, and the editor is one page with no build step.

## R5. Escape keeps an edit in progress

**Decision**: per the clarification, Escape in an on-slide text edit or a
source popup ends it the way a click elsewhere does, saving what was typed.
Undo takes it back as one step. The Cancel button in the source popup still
discards. The status line changes from "Esc cancels" to "Esc or click
elsewhere saves".

## R6. Copy and paste of slides

**Decision**:
- **Copy:** copying a selected slide (in the slide list) puts its full markdown
  source on the system clipboard and in the editor's own copy, as elements are
  copied today.
- **Paste:** pasting, with a slide or the slide list focused, goes to a new op,
  `paste` (CLI `qd paste --after SLIDE --md TEXT|-|--file PATH`). It splits the
  text into slides and inserts them after SLIDE, with fresh ids wherever an id
  would clash. The renaming reuses `ops::dup`'s `renew_id`. It is one patch, so
  one undo step. With an element selected, pasting stays `block add` (as today).
- **Text from other applications:** text that didn't come from qd's own copy
  and isn't slide markdown becomes a paragraph. Its `<` and `&` are escaped, so
  text from elsewhere is never taken as raw HTML (spec edge case).

**Rationale**: dup already knows how to give a copied slide fresh ids. A
separate op keeps `qd add` (title + body) simple.

## R7. VS Code: keys the panel must leave alone

**Decision**:
- **The list:** a checked-in list, `vscode/default-keys.json`, holds VS Code's
  default bindings that apply while a webview has focus (macOS). It comes from
  VS Code's public default keybindings reference. It includes ⌘P, ⇧⌘P, ⌘B,
  ⌘J, ⌘W, ⌘N, ⌘O, ⌘S, ⌘, and ⌘K (a chord prefix), plus ⌘1–9, ⌃` and ⇧⌘E/F/G/D/X.
- **Exceptions:** the editing keys VS Code hands to a focused webview (⌘Z,
  ⇧⌘Z, ⌘X, ⌘C, ⌘V, ⌘A) stay the editor's, as spec 006 relies on.
- **Clashing actions:** each one gets a `vscode` key in `actions.json`, or
  `null` (menu-only).
- **Test:** a node test fails if any action's effective key in the panel is on
  the list.
- **Real-app check:** whether VS Code still grabs a key the webview handles is
  checked in a real VS Code run, recorded in a checklist.

**Rationale**: VS Code's bindings must win (spec US5). A static list is
testable without VS Code. For example, ⌘K (link) and ⌘B (bold) both clash, so
in the panel they become ⌥⌘K and ⌥⌘B or menu-only.

## R8. Theme and the other first settings

**Decision**:
- **`theme`:** system (default), light, dark or hc. The CSS already has
  `:root[data-theme]` from 006. "system" removes the attribute, so the
  existing `prefers-color-scheme` rules apply. In VS Code the panel's theme
  message still wins.
- **`source.side`:** left (default) or right. A class on `main` flips the order.
- **`source.open`:** last (default), open or closed.
- **`source.save`:** pause (default; today's "saves as you pause") or explicit
  (on ⌘S or leaving the pane). This is the spec's "preview update while
  typing", since the preview re-renders when the source is saved.
- **`confirm.slide_delete`:** on/off.
- **`hints.shortcuts`:** on/off.

## R9. Undo covers every edit

**Decision**: every editor edit already goes through an op, so the store
journals it, except source-pane typing, which saves through `replace`. A
browser test runs one edit from each surface (menu, shortcut, toolbar, drag,
on-slide typing, source pane) and checks that one undo reverts exactly that
edit. Anything that fails gets fixed in the task that finds it.

## Licences

No new dependencies. The editor stays plain ES modules, and the Rust side
uses only crates already in the tree (serde_json, notify).
