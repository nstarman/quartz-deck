# Quickstart / Validation: 002-offline-export

Prerequisites: everything from 001; Google Chrome (or let qd download Chrome
for Testing on first use); network for the first export only. For Plotly
fixtures: `pip install plotly` (tests skip otherwise).

## 1. Detect (US1, SC-001)

```bash
qd new /tmp/qd-off/deck.qmd --title Offline && cd /tmp/qd-off
qd add --title Math --md '$$\int_0^1 x\,dx$$'
qd check --offline            # E_OFFLINE: P_NETWORK (cdn mathjax) + P_RAW_TEX on "math"
```

## 2. Export (US2, SC-002)

```bash
qd export                     # {"ok":true,"html":".../export/deck.html","check":{"ok":true,...}}
qd check --offline --html export/deck.html   # ok, no problems
```

Turn off Wi-Fi, open `export/deck.html`: math renders. `git status` shows no
changes to `deck.qmd` or project files.

## 3. PDF (US3, SC-003)

```bash
qd add --title Steps --md $'::: {.fragment}\none\n:::\n\n::: {.fragment}\ntwo\n:::'
qd export --pdf               # pdf.pages == number of fragment steps
qd pdf --notes                # notes on separate pages
```

## 4. Editor (US4)

`qd serve --open` → Export → problem list; click a problem → preview shows that slide.

## 5. Browser management

```bash
qd browser                    # which Chrome is used
QD_CHROME=/nonexistent QD_NO_DOWNLOAD=1 qd check --offline   # E_NO_BROWSER with hint
```

Bumping pinned versions (MathJax, Chrome for Testing): update the version and
SHA-256 constants in `src/assets.rs` / `src/browser.rs`, run the export tests.
