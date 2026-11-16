# Plugins

New slide features come from Quarto and existing libraries first. A plugin here
is for what nothing provides, or to connect an existing library to Quarto and
reveal.js (constitution, principle VII).

Plugins come in two kinds (constitution 1.4.0):
- a **render plugin** is a Quarto extension, such as `cite-links` or
  `dot-image` (pictures in Graphviz boxes);
- a **source plugin** feeds `qd bib` with references and renders nothing.
  It is a program `qd-NAME` on `PATH` that prints CSL-JSON, built as a
  member crate of this workspace, such as `zotero/` (`qd-zotero`).

A render plugin is a folder:

```text
plugins/<name>/
├── _extensions/<name>/   the Quarto extension: what renders (Lua, CSS, JS, _extension.yml)
├── example.qmd           renders with plain `quarto render`, no qd
└── README.md             what it does, how to install it in a deck
```

Later, as plugins need them (not before), a plugin may also hold:
- `qd-plugin.json`: what the editor offers for it (inserts, element kinds, preview CSS);
- `python/`: a package for code cells.

A plugin follows every rule the core does:
- third-party code is listed with a license compatible with private use;
- it works offline;
- it runs code only on an explicit Run;
- it writes only the deck and its own files.

`crates/qd/tests/plugins.rs` renders every `example.qmd` with plain Quarto.
