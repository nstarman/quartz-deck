# Quickstart / Validation: 001-core-editor

## Prerequisites

- Rust stable (`cargo --version`), Quarto ≥ 1.4 (`quarto check`).
- `cargo install --path .` (puts `qd` on PATH).

## 1. CLI edits (US1)

```bash
qd new /tmp/qd-demo/deck.qmd --title "Demo"
cd /tmp/qd-demo
qd add --title "Intro" --md "Hello."                     # target: "intro"
qd add --after intro --title "Results" --md $'- one\n- two'
qd slides                       # 3 slides, explicit IDs, JSON
qd set results --attr background-color=aquamarine --dry-run   # diff only
qd set results --attr background-color=aquamarine
qd move results --before intro
qd check                        # {"ok": true, ...}
qd --human slides               # order: first-slide, results, intro
```

Expected: every command prints JSON with `"ok": true`; `qd show nope`
exits 1 with `E_NOT_FOUND` on stderr.

## 2. Round trip (SC-002)

```bash
cargo test --test roundtrip     # every tests/corpus/*.qmd byte-identical
```

## 3. Live preview (US2)

```bash
qd serve --detach               # prints {"url": "http://127.0.0.1:PORT/", "port", "pid"}
qd url results                  # URL opening on that slide
qd title results "Findings"     # preview updates in < 3 s, stays on slide
printf -- '---\ntitle: [x\n---\n' > deck.qmd   # banner shows the Quarto error, last render kept
qd stop
```

## 4. GUI (US3, US4)

Open the URL in a browser (or the agent's browser tool):
- drag a slide in the list → `qd slides` shows the new order;
- click a paragraph, edit, confirm → `git diff` shows only that paragraph;
- `qd box add results --text "Note" --at 100,100 --size 300,80`, drag it
  in the GUI (or edit Left/Top/Width/Height in its panel), then
  `qd show results` reports the new `geom`;
- `quarto render deck.qmd` → box at the same position (±1 px).

## 5. Concurrency (SC-007)

```bash
V=$(qd slides | jq -r .version); echo >> deck.qmd
qd title intro "X" --expect $V  # E_CONFLICT, file unchanged
```

## 6. Agent loop (SC-001)

`qd skill --install ~/.claude/skills/quartz-deck`, then ask Claude Code to
perform the 5-step task from SC-001 in `/tmp/qd-demo`; verify with
`qd check` and the preview.
