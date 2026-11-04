# Clean-room policy

quartz-deck is an independent implementation. It must not contain, derive
from, or be shaped by the proprietary code or assets of slides.com (or any
other proprietary slide editor).

## Never

- Read, copy, decompile, or paste slides.com source code, minified JS, CSS,
  HTML templates, icons, fonts, images, or other assets.
- Inspect slides.com network traffic, DOM, or storage to reproduce its
  internals, file format, or API.
- Reproduce slides.com's visual design, UI layout, branding, or wording.
- Ask an LLM to "write it like slides.com does" or paste output that quotes
  its code.

## Allowed sources

- Public documentation and MIT/BSD/Apache-licensed source of
  [reveal.js](https://revealjs.com), [Quarto](https://quarto.org), and
  [Pandoc](https://pandoc.org).
- General, non-proprietary UI ideas common to many editors (a slide list,
  a canvas, drag handles, a properties panel).
- Our own design, specs (`specs/`), and the project constitution.

## Process

- Every new third-party dependency is checked for a compatible license
  before it is added, and listed in the feature's `plan.md`.
- Anyone contributing (human or LLM) who has seen slides.com internals
  does not write the equivalent feature here.
- If a contamination is suspected, stop, record it in an issue, and remove
  the affected code before continuing.
