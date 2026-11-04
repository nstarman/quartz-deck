# Spec roadmap

The order in which features are specified and built. Each line becomes a
`specs/NNN-name/` folder when its spec is started; until then it is a plan
and may be reordered.

| # | Spec | Status | Summary |
|---|------|--------|---------|
| 001 | core-editor | done (merged) | Lossless `.qmd` model, `qd` CLI and skill, live preview, browser editor with form panel, drag/resize boxes, safe concurrent edits |
| 002 | offline-export | done (PR #2) | `qd check --offline`, `qd export` (one self-contained HTML), `qd pdf`, editor Export |
| 003 | on-slide-editing | spec draft | Edit on the slide itself: type into titles/text, in-place source editor for math/code, property toolbars, add/delete/reorder elements, **right-click context menus** with the full set of actions; the form panel becomes a hidden Source view; full formatting toolbar |
| 004 | app-shell | planned | Behave like a desktop app: an app menu bar (File, Edit, Insert, Slide, View, Help) with every action and its shortcut; a Settings window (editor theme, panel layout, autosave/commit behaviour, preview zoom, default export options, keyboard shortcuts) stored per user; a keyboard-shortcut reference; consistent standard paradigms (selection, clipboard, undo, Escape/Enter) across the app |
| 005 | shared-focus | planned | The editor reports what the person is looking at and has selected (`qd focus`), so "make this slide…" in chat refers to the right slide/element; agents can point the editor at a slide or element |

Small editor improvements that don't need a spec (e.g. the side-scrolling
slide strip and hover preview) go straight to a PR.
