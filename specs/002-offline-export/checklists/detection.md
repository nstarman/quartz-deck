# Offline failure detection (T028, SC-001)

Each failure mode from the quarto-revealjs guide (nstarman/skills#25) has a
fixture in `tests/fixtures/offline/` and a test asserting the problem code.
Run 2026-11-04, Quarto 1.11.9, Chrome 156, macOS.

| Failure mode | Fixture | Expected | Test | Result |
|---|---|---|---|---|
| Default deck loads MathJax from a CDN | math.qmd | `P_NETWORK` (cdn.jsdelivr.net/…/mathjax) + `P_RAW_TEX` on `math`, none on `plain` | offline.rs `default_deck_with_math_fails_offline` | ✅ |
| KaTeX never embedded | katex.qmd | `P_RAW_TEX` on `math` + `P_NETWORK` (katex) | offline.rs `detects_katex` | ✅ |
| Plotly figure carries web MathJax / plotly.js | plotly.qmd | `P_NETWORK` (plot.ly or mathjax) | offline.rs `detects_plotly_web_scripts` | CI (needs Python plotly; skipped locally) |
| Missing background video | missing.qmd | `P_MISSING_FILE` on `gone` | offline.rs `missing_file_page_error_and_visible_notes` | ✅ |
| Missing background iframe | missing-iframe.qmd | `P_MISSING_FILE` on `app` | offline.rs `detects_missing_background_iframe` | ✅ |
| Script error on the page | throws.qmd | `P_PAGE_ERROR` | offline.rs (same test) | ✅ |
| `show-notes` in YAML | shownotes.qmd | `P_NOTES_VISIBLE` (warning) | offline.rs (same test) | ✅ |
| Chalkboard cannot be one file | chalkboard.qmd | export refused, `E_OFFLINE_INCOMPATIBLE` | export.rs `refuses_offline_incompatible_features` | ✅ |
| `about:blank` math-plugin request | (all exports) | never reported (FR-003) | offline.rs (math test) | ✅ |

And the fix side (SC-002): every renderable fixture exports with `check.ok`
(export.rs: math, bgvideo, bgiframe, skillsetup; plotly in CI).

Timing (SC-004, `export_30_slides_under_a_minute`): 30 slides with math and a
fragment each, export + check + PDF = **9.1 s**, 61 PDF pages (macOS, without
the Plotly figure — Python plotly is not installed locally; CI runs it with one).
