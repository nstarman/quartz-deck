# CLI Contract: `qd`

The LLM-facing interface. `qd schema` emits this contract as JSON; tests
keep the schema, this file's commands and the Skill in sync.

## Conventions

- `qd [--deck PATH] <command> [args]`. The deck is `--deck`, else
  `$QD_DECK`, else the only `*.qmd` in the current directory.
- stdout: one JSON object per invocation (default). `--human` for text.
- stderr on error: `{"error": {"code": "E_…", "message": "…", "hint": "…"}}`,
  exit code 1 (usage errors `E_USAGE`: 2). The file is never modified on error.
- Markdown input: `--md TEXT` (`--md -` reads stdin) or `--file PATH`;
  values may start with `-` (e.g. lists).
- Edit commands accept `--dry-run` (return diff, write nothing) and
  `--expect <version>` (fail with `E_CONFLICT` if the file changed).
- Every edit result:
  ```json
  {"ok": true, "op": "add", "version": "9f2c…", "diff": "@@ …",
   "assigned_ids": ["results"], "target": "results"}
  ```
- Slide refs: explicit id, auto-id, `@N` (1-based position), `first`,
  `last`. Element refs: `<slide>/<i>` or `#<element-id>`.

## Read commands

| Command | Output |
|---|---|
| `qd slides` | `{"version", "slides": [{"id", "explicit", "index", "kind", "title", "summary", "blocks": n, "boxes": n}]}` |
| `qd show <slide>` | `{"version", "slide": {…, "attrs", "source", "blocks": [{"id", "kind", "text", "attrs", "geom?"}]}}` |
| `qd meta` | `{"version", "front_matter": {…}}` (parsed YAML) |
| `qd check` | `{"ok", "problems": [{"severity", "code", "slide?", "message"}]}`; includes Quarto render errors when Quarto is present |
| `qd schema` | every command, its args (from clap) and output shape; error codes; ref syntax |
| `qd skill [--install DIR]` | prints / writes the agent Skill |
| `qd url [<slide>]` | `{"url": "http://127.0.0.1:PORT/#/<id>"}` from the running server, else `E_NO_SERVER` |

## Edit commands

| Command | Effect |
|---|---|
| `qd add [--after <slide> \| --before <slide>] [--title T] [--md MD \| --file F] [--attr k=v]… [--class c]…` | insert slide (default: end); id = slug of title (unique), else `s-xxxx` |
| `qd dup <slide> [--after <slide>]` | duplicate slide with fresh IDs |
| `qd move <slide> (--after \| --before) <slide>` | reorder |
| `qd rm <slide>` | delete slide |
| `qd title <slide> <text>` | set title (`""` → untitled `## {…}`) |
| `qd body <slide> (--md MD \| --file F)` | replace all non-box blocks |
| `qd set <slide> --attr k=v… [--unset k]… [--class c]… [--unclass c]…` | slide attributes (background, transition, classes) |
| `qd edit <element> (--md MD \| --file F)` | replace one block's text |
| `qd box add <slide> (--text MD \| --image PATH) --at LEFT,TOP --size W,H` | new positioned box |
| `qd box move <box> --at LEFT,TOP` / `--by DX,DY` | reposition (px) |
| `qd box resize <box> --size W,H` | resize (px) |
| `qd box rm <box>` | delete box |
| `qd meta set <key> <yaml-value>` / `qd meta unset <key>` | top-level front matter key (single-line value) |
| `qd undo` | revert last qd edit (only if file unchanged since) |
| `qd redo` | re-apply last undone edit (only if file unchanged since the undo) |
| `qd new <path> [--title T]` | create minimal reveal.js deck |

## Render / serve

| Command | Output |
|---|---|
| `qd render [--to FORMAT]` | `{"ok", "output": "path/to/deck.html", "log"}` |
| `qd serve [--port N] [--open]` | prints `{"url", "pid"}` then runs until killed; `--detach` returns immediately |

## Error codes

| Code | Meaning |
|---|---|
| `E_NO_DECK` | no deck given/found |
| `E_PARSE` | file could not be segmented at all |
| `E_NOT_FOUND` | slide/element ref matches nothing |
| `E_AMBIGUOUS` | ref matches several (duplicate IDs) |
| `E_UNSAFE_EDIT` | target is opaque/ambiguous; edit refused |
| `E_CONFLICT` | file changed since read / `--expect` mismatch |
| `E_INVALID_ARG` | bad value (attr syntax, geometry, YAML) |
| `E_NO_UNDO` | journal empty or file changed since last qd edit |
| `E_NO_QUARTO` | Quarto not on PATH (hint: install URL) |
| `E_RENDER` | Quarto render failed (log in `details`) |
| `E_NO_SERVER` | no running `qd serve` for this deck |
| `E_USAGE` | bad command line (exit 2) |
| `E_IO` | filesystem error |
