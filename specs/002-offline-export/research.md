# Research: 002-offline-export

Sources: the quarto-revealjs skill and guide (nstarman/skills#25, verified
Quarto 1.11.9 / reveal.js 5.1.0 / MathJax 3.2.2 / plotly 7.1.0), reveal.js
docs (PDF export, config via URL), Chrome DevTools Protocol docs, Chrome for
Testing docs. Spikes below were run 2026-11-04 with Quarto 1.11.9 and Chrome
156 on macOS.

## R1. Offline setup without touching the deck

- **Decision**: render an *export copy* `<stem>.qd-export.qmd` next to the deck
  (same mechanism as the preview copy, so `_quarto.yml` projects apply) whose
  front matter is the deck's, merged with:
  ```yaml
  format:
    revealjs:
      embed-resources: true
      html-math-method: { method: mathjax, url: "about:blank" }
      include-in-header: [<user's entries…>, "<cache>/lib/qd-offline-header.html"]
  ```
  The header sets `window.MathJax` config and loads `<cache>/lib/mathjax-3.2.2/
  tex-svg-full.js` (and `plotly.min.js` when the deck has Plotly figures) by
  **absolute path**; `embed-resources` inlines them.
- **Verified (spike)**: output is one 5.7 MB HTML with MathJax inlined; no
  external `src`; MathJax 3.2.2 typesets both inline and display math with the
  network blocked.
- **Already set up** (deck/project sets `html-math-method.url: about:blank`,
  i.e. the skill's `_quarto.yml`): qd adds nothing — the user's header loads
  MathJax; adding ours would load it twice. Detection reads the deck's front
  matter and `_quarto.yml` files from the deck folder up to the project root.
- **Alternatives**: post-processing the HTML (fragile; Quarto has already
  inlined CDN scripts by then); `-M` metadata flags (cannot express the nested
  `html-math-method` object reliably).

## R2. Plotly figures

- **Decision**: when the deck has Python cells that import plotly, the export
  copy gets the skill's renderer cell (`_deck/setup.qmd` equivalent) inserted
  after the front matter, so figures are bare divs with no web MathJax or
  plotly.js; the header loads plotly.js once. plotly.js is copied from the
  installed Python plotly (`package_data/plotly.min.js`, matches the deck's
  figures); if Python plotly is not importable, export fails with a hint
  (rendering the cells would fail anyway).
- **Not verified locally** (no Python plotly on the dev machine): tests that
  need it skip when it is absent; CI installs `plotly` to run them.

## R3. Driving Chrome: check

- **Decision**: `chromiumoxide` (async, on tokio) with Fetch-domain interception on `*`
  (the spike used `headless_chrome`; same protocol calls):
  continue `file:`, `data:`, `blob:` and `about:blank` (with any query —
  the math plugin requests `about:blank?config=TeX-AMS_HTML-full`, FR-003),
  fail everything else with `InternetDisconnected` and record it. Listen for
  `Runtime.exceptionThrown`. Then, in the page: await `Reveal` ready and
  `MathJax.startup.promise`, visit every slide (`Reveal.getSlides()`,
  `Reveal.slide(h,v)`) and step its fragments, recording the current slide id
  as each request/exception arrives, and test each slide's visible text for
  TeX delimiters (`$$`, `\(`, `\[`, `$…$`, `\begin`, `\int`…).
- **Verified (spike)**: offline export → 3 slides / 5 steps / 2 MathJax
  containers / no raw TeX / no errors / only `about:blank?config=…`
  intercepted. Default Quarto deck → raw TeX on the math slide and a blocked
  `https://cdn.jsdelivr.net/npm/mathjax@2.7.9/MathJax.js…` request (SC-001).
- **Missing local files**: statically from the HTML (`data-background-video`,
  `data-background-iframe`, `data-background-image` left as paths,
  `parallax-background-image`) checked against the folder, plus
  `Network.loadingFailed` for `file:` URLs at run time.

## R4. PDF

- **Decision**: open `<html>?print-pdf&pdfSeparateFragments=true` (and
  `&showNotes=separate-page` when notes are requested; `pdfSeparateFragments=
  false` for one page per slide), wait for Reveal ready and MathJax, then
  `Page.printToPDF { preferCSSPageSize: true, printBackground: true }`.
- **Verified (spike)**: 5 pages for title + math + 2-fragment slide (3 without
  the flag: Quarto's default is not to separate); notes on a page of their own
  (6 pages); no notes without the flag; math present on page 2 (rasterized
  check — MathJax SVG has no extractable text, so tests count pages from the
  PDF structure and check math through the DOM, not `pdftotext`).
- The skill's finding stands: Chrome's `--print-to-pdf` CLI flag prints blank
  decks — not used.

## R5. Browser

- **Decision**: in order: `$QD_CHROME`; Google Chrome / Chromium / Chrome for
  Testing at standard paths (macOS `/Applications/…`, Linux `google-chrome`,
  `chromium`, `chromium-browser` on PATH); qd's cached download. If none and
  downloads are allowed (default; `--no-download` / `QD_NO_DOWNLOAD=1` forbid),
  download the **pinned Chrome for Testing** headless shell for the platform
  (mac-arm64, mac-x64, linux64) over HTTPS into `<cache>/browser/<version>/`,
  verify its SHA-256 against a value pinned in qd, unzip, report size and path.
  `qd browser` shows which browser would be used; `--install` / `--remove`.
- **Rationale**: user's choice (spec clarification); Chrome for Testing is
  Google's versioned, scriptable distribution. The pinned hashes are recorded
  when the version is bumped (quickstart documents how).

## R6. Libraries and licenses

Chrome for Testing pins (T008, 2026-11-04): version 157.0.8103.38,
`chrome-headless-shell` zips — mac-arm64 `e6a6b135…16ce82` (105 MB), mac-x64
`79821745…186de3`, linux64 `deafffa7…a4ba2d`, linux-arm64 `b9b910a5…6c42fd6c`
(full hashes in `src/browser.rs`). The real download, verification, unpack and
`--version` run were tested once on macOS (`QD_TEST_BROWSER_DOWNLOAD=1`).

| Item | License | Use |
|---|---|---|
| chromiumoxide 0.9 | MIT OR Apache-2.0 (whole tree permissive, 139 crates) | DevTools client |
| ~~headless_chrome 1.0~~ | MIT, but its build-dependency `auto_generate_cdp` 0.4.6 is **GPL-3.0** | rejected by the license check (T001) |
| webpki-roots (via ureq) | CDLA-Permissive-2.0 (Mozilla root certificates) | HTTPS trust roots; added to the license allowlist |
| ureq | MIT OR Apache-2.0 | downloads |
| zip | MIT | unpack browser |
| MathJax 3.2.2 (`es5/tex-svg-full.js`, sha256 `a4354ff94fd868aea0cc6eaaa79a57fda0588646fc46ee3700a349ee0a11cbe6`) | Apache-2.0 | offline math (downloaded, not vendored) |
| plotly.js (from the user's Python plotly) | MIT | offline figures |
| Chrome for Testing | Chromium BSD-style + bundled third-party | check/PDF (downloaded on demand, not vendored) |

The repository's license test (FR-041) covers the Rust crates; downloaded
assets are recorded with version + hash in `qd browser` / export output.

## R7. Offline-incompatible features

- **Decision**: refuse before rendering when the deck (front matter or
  project) enables `chalkboard` (refuses `embed-resources`) or `multiplex`
  (needs a server): problem `P_OFFLINE_INCOMPATIBLE` with the key and file.
  Warn (`P_NOTES_VISIBLE`) when `show-notes` is set in YAML (shows notes to the
  audience). Background video/iframe/parallax files are copied into the export
  at their relative paths (`P_MISSING_FILE` if absent).

## R8. Export folder

- **Decision**: build in `<cache>/export-staging/<n>/`, then replace
  `<out>` (default `<deck dir>/export/`) by rename; the old export is removed
  only after the new one is in place. On check failure the staging folder is
  kept and its path reported, and the previous export is left untouched.
