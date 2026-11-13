# Code audit: is quartz-deck set up as a monorepo with plugins?

2026-11-13, on `main` at `b0d1fcc` (after spec 013). This is a read-only audit;
nothing in the code was changed.

**The direction** (from the Keynote review): new slide features such as
shapes, arrows, galleries and nicer tables come through Quarto and existing
libraries (Python, JavaScript, HTML, CSS) wherever possible. Our own plugins
are for new features, or to connect an existing library to Quarto and
reveal.js. A deck must stay plain Quarto, renderable with `quarto render` and
no qd.

**Short answer:** the deck side is ready. Quarto's own extension mechanism
already works in qd's preview and exports, and plugin markup already parses,
round-trips and can be placed. The repository side is not ready:
- it is one crate, not a workspace;
- the editor's menus, inserts and preview styling are compiled-in lists that
  no plugin can add to;
- there is no `plugins/` home, so the one real plugin we have lives inside a
  skill;
- the constitution forbids a plugin system "until a concrete need exists".

None of this needs a rewrite. It needs a constitution amendment, a mechanical
move into a workspace, a `plugins/` folder, and a small data-driven
contribution point in the editor.

---

## 1. What the repository is today

| Part | Where | Size | Built and tested by |
|---|---|---|---|
| `qd` (CLI, server, deck model, render) | `src/*.rs`, one crate `quartz-deck` | 25 files, ~14,500 lines; `cli.rs` 2,053, `ops.rs` 1,490, `server.rs` 1,293, `deck.rs` 1,018 | `cargo test` (20 test binaries) |
| Editor (browser UI) | `src/ui/*.js`, `.css`, `.html`, `actions.json` | 18 files, ~5,300 lines; `editor.js` 2,127 | compiled into `qd` with `include_str!`; browser tests in `tests/onslide.rs` |
| Kernel bridges | `src/kernel_bridge.{py,R,jl}` | 140 / 83 / 86 | compiled in; `tests/engine.rs`, `tests/run.rs` |
| Built-in layouts | `src/layouts/*.md` (12) | | compiled in (`layouts.rs`) |
| Lua filter for exports | `src/formats.lua` | | compiled in (`formats.rs`) |
| VS Code extension | `vscode/` (own `package.json`, no npm dependencies) | 5 JS files | `node --test vscode/test/*.test.js` |
| Skills | `skills/quartz-deck`, `skills/quarto-revealjs` (with `assets/_extensions/paper`, a Quarto shortcode extension) | | `tests/skills`, CI job `test quarto-revealjs` |
| Specs, constitution | `specs/`, `.specify/` | | |
| CI | `.github/workflows/ci.yml` (one job), `test-quarto-revealjs.yml` | | |

The constitution's VII says "One Rust workspace, one `qd` binary". In fact
`Cargo.toml` is a single `[package]`, not a workspace.

## 2. What already works for plugins

1. **Quarto extensions apply in qd.** qd renders a temporary copy of the deck
   *next to the deck* (`render::render`), so a deck's own `_extensions/`
   folder, or its Quarto project's, is found by Quarto exactly as with
   `quarto render`. This covers shortcodes, Lua filters, custom formats,
   reveal.js plugins, CSS and JS. Users can install them with `quarto add`;
   qd needs nothing for that.
2. **Plugin markup parses and round-trips.** An unknown div becomes
   `BlockKind::Div` and a shortcode inside a paragraph stays text
   (`deck.rs::classify`). Both survive edits byte for byte and can be edited
   as source in the pop-up.
3. **Plugin content can be placed freely.** A div with the class `.absolute`
   is a `Box`. So `::: {.arrow .absolute ...}` is movable and resizable, can
   be stacked, and snaps, with today's code.
4. **Code-based content runs through the engines.** Python (matplotlib,
   pandas, any library in the deck's locked environment), R, Julia and
   Observable JS already produce figures and HTML on the slide (004 and 013).
   A "nicer pandas tables" plugin can be a Python package added with
   `qd env add`.
5. **Layouts are data.** The 12 built-in layouts are markdown files, and
   users already save their own (`_layouts/`, `qd layout save`).
6. **Offline export** already bundles what a page loads, `data-app` webapps
   (figpress) included (010). A plugin's JS and CSS will be checked the same
   way.

## 3. Gaps, most important first

### G1. The constitution forbids a plugin system (principle VII)

> "No speculative abstractions, plugin systems, or configuration until a
> concrete need exists."

The need is now concrete: shapes, arrows, galleries and table displays,
through libraries. This is a MINOR amendment (expanded guidance). It should
say:
- plugins are Quarto extensions first, so decks stay plain Quarto;
- the editor may read plugin manifests;
- each plugin's dependencies pass the same licence rule;
- what a plugin may not do (run code outside the Run rules, reach the
  network when offline, write anywhere but the deck).

### G2. Not a workspace

- One crate holds the deck model, CLI, server, render pipeline and editor
  assets. That's fine for one binary, but there's nowhere for a second Rust
  crate to go, such as a shared deck-model library a plugin's tests or a
  tool could use.
- `vscode/`, `skills/` and any future `plugins/` sit beside the crate with
  no shared layout or test entry point.
- **Fix (mechanical, no behaviour change):** make the root `Cargo.toml` a
  `[workspace]` and move the crate to `crates/qd/`. Paths in
  `include_str!`, `tests/` and CI change, and nothing else. Split crates
  only when a second consumer actually exists (VII still applies).

### G3. The editor has no contribution points

Everything a plugin would add to the editor is a hard-coded list:

| What | Where | Effect |
|---|---|---|
| Insert menu entries | `INSERTS` in `src/ui/actions.js` | a plugin can't offer "Insert ▸ Arrow" |
| Actions, menus, keys | `src/ui/actions.json`, compiled into `qd` (also read by `cli.rs` for `qd actions`) | no plugin actions |
| Editor modules | `UI_MODULES` table in `server.rs` (`include_str!`) | a plugin can't ship editor JS |
| Preview styling | `PREVIEW_CSS` string in `editor.js` | no plugin hover or edit affordances |
| Element panels | the Properties panel and element bar know the core kinds only | no fields for a plugin element (arrow direction, gallery images) |
| What can't be placed, and why | `UNPLACEABLE` in `actions.js` | |

**Fix:** one data-driven contribution point. qd reads plugin manifests
(G4) and serves them at `GET /api/plugins`. The editor merges:
- **inserts**: a label plus a markdown template;
- **element kinds**: matched by class, with simple panel fields that edit
  attributes or the source;
- **preview CSS**.

No plugin JS runs in the editor at first. All edits still go through `qd`
ops (`qd block add --md`, `qd set`, `qd edit`), so agents and the CLI get
plugin features for free.

### G4. No plugin home, packaging or discovery

- The only real plugin, the `paper` shortcode, lives in
  `skills/quarto-revealjs/assets/_extensions/paper`, a skill's asset folder.
- **Fix:** add a `plugins/` folder. Each plugin is a directory:

  ```text
  plugins/<name>/
  ├── _extensions/<name>/      # the Quarto extension: what renders (Lua, CSS, JS, _extension.yml)
  ├── qd-plugin.json           # optional: editor contributions (inserts, element kinds, preview CSS)
  ├── python/                  # optional: a Python package for code cells (pyproject.toml)
  ├── example.qmd              # renders with plain `quarto render`
  └── tests/                   # render test (and editor test if it has a manifest)
  ```

- **Discovery:** qd looks in the deck's `_extensions/` (and the Quarto
  project's) for a `qd-plugin.json` next to each extension. So a plugin
  installed with `quarto add` lights up in the editor, and a deck stays
  self-contained.
- `qd plugin add <name>` copies a bundled plugin into the deck's
  `_extensions/`. It's the same as `quarto add`, but offline.

### G5. Licences and offline checks don't reach plugins

- `tests/licenses.rs` checks Cargo dependencies only. A plugin bringing a JS
  library (an arrows library, say) or a Python package needs the same rule:
  each plugin lists its third-party code and licence in its manifest, and a
  test checks it against the allowed list (FR-041).
- The offline check should run on each plugin's `example.qmd`, so a plugin
  that loads from a CDN fails in CI.

### G6. CI has one job for everything

- `ci.yml` runs every Rust test in one job, about 15 minutes with R and
  Julia. Plugins would add Quarto renders.
- **Fix:** a matrix job, or one job that renders every `plugins/*/example.qmd`
  with `quarto render` (no qd) and with qd, and runs their tests. Rust stays
  as it is.

### G7. Large files (lower priority)

- `editor.js` (2,127 lines), `cli.rs` (2,053) and `ops.rs` (1,490) are each
  several features in one file. G3's contribution point lands in
  `editor.js` and `actions.js`.
- Splitting `editor.js`'s preview hooks (hover, selection, `PREVIEW_CSS`,
  frame hooks) into their own module first would keep that change small.
  `cli.rs`'s per-command handlers could move next to their modules later.
  Neither blocks plugins.

## 4. Suggested order

1. **Constitution amendment** (G1): VII allows plugins as Quarto extensions
   with an optional editor manifest, and states their rules: licences,
   offline, code runs only on Run, no writes outside the deck.
2. **Workspace move** (G2): `crates/qd/`, root `[workspace]`, paths in tests
   and CI. No behaviour change, all tests green, one PR.
3. **`plugins/` and the first plugin** (G4): move `paper` out of the skill
   into `plugins/paper/`, with `example.qmd` and a render test. The skill
   points at it.
4. **Plugin licence and offline tests, and the CI job** (G5, G6).
5. **Editor contribution point** (G3), as a spec: manifest schema,
   `/api/plugins`, inserts, element kinds by class, preview CSS. Prove it
   with one real plugin, shapes and arrows built on an existing JS or SVG
   library.

Steps 1–4 are small and mechanical. Step 5 is a feature (spec 014?), and it
is where "shapes and arrows through libraries" lands.

## 5. Questions for you

- **Plugin repo:** in this monorepo (`plugins/`), or a separate repository
  that qd bundles? This audit assumes in-repo.
- **Plugin JS in the editor:** no plugin code in the editor at first, only
  data (templates, fields, CSS)? That keeps the token and Run rules simple.
  Later plugins might need editor JS (a gallery picker, say).
- **figpress:** is figpress-apps a plugin here (`plugins/figpress`), or does
  it stay its own project that qd supports through `data-app`?
- **Python packages:** do plugin Python packages live under
  `plugins/<name>/python/` and install into a deck's environment with
  `qd env add` (a path or a published package)?
