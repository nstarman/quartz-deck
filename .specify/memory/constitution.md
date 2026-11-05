<!--
Sync Impact Report
- Version: 1.1.0 → 1.2.0 (MINOR: Constraints materially expanded, spec 004)
- Constraints: external tools uv (default) or pixi (PyPI packages, not conda),
  needed only to run code cells or render decks with code; one locked
  environment per deck, never plain pip; code runs only on an explicit action
  or `qd run`, and code-running endpoints require the per-session token
- Principles, Development Workflow, Governance: unchanged
- Templates: none need changes (plans already list external tools)
- Deferred TODOs: none
- Previous: 1.0.0 → 1.1.0 (MINOR: workflow gate added)
- Development Workflow: added the pre-PR concision audit (`/pr-audit`),
  which enforces Principle VII on every PR's code and infrastructure
- Templates: tasks-template.md gains a standard final "Concision audit" task;
  new repo skill `.claude/skills/pr-audit/SKILL.md`
- Deferred TODOs: none
- Previous: none → 1.0.0 (initial ratification: Principles I–VII,
  Constraints, Development Workflow, Governance)
-->

# quartz-deck Constitution

## Core Principles

### I. Clean-Room Implementation (NON-NEGOTIABLE)

No code, assets, styles, file formats, UI design, or wording from slides.com
(or any proprietary slide editor) may be consulted, copied, or reproduced.
Only public documentation and permissively licensed source of reveal.js,
Quarto, and Pandoc, plus our own specs, may inform the implementation.
`CLEANROOM.md` is binding; any suspected contamination halts work on the
affected feature until it is removed.

### II. The `.qmd` File Is the Source of Truth

The Quarto document on disk is the only persistent state. The GUI and CLI
MUST edit it through minimal, localized text patches; bytes outside the
edited region MUST be preserved exactly (formatting, comments, ordering).
Parsing and re-writing an unedited document MUST be byte-identical. No
sidecar database or hidden state that the `.qmd` cannot express. Positioned
elements use standard Quarto syntax (e.g. `.absolute` divs) so the file
renders with stock Quarto.

### III. CLI-First, LLM-Friendly

Every editing capability exists as a CLI command before (or alongside) its
GUI control; the GUI MUST NOT have abilities the CLI lacks. Commands are
non-interactive, idempotent where possible, address slides and elements by
stable IDs, print machine-readable JSON by default (human format on
request), write errors to stderr with a stable error code and non-zero exit,
and are self-describing (`qd schema`). An agent Skill documents the workflow
and is kept in sync with the CLI.

### IV. Live Browser Preview

The editor runs in a standard web browser served from localhost, so humans
and LLM agents see and drive the same UI. Changes to the `.qmd` from any
source (GUI, CLI, external editor) appear in the preview without a manual
reload. The rendered output is real reveal.js produced by Quarto, not an
approximation that can drift from final output.

### V. Safe Concurrent Edits

GUI, CLI, and external editors may touch the same file. Writes MUST be
atomic, and every edit MUST detect that the file changed since it was read
and refuse (or rebase) rather than silently overwrite. No edit may lose user
data.

### VI. Test-First Core

The document model, parser, and patcher are developed test-first, with
round-trip (byte-identical) tests over a corpus of real Quarto decks. Each
CLI command has an integration test asserting its JSON output and the
resulting file. GUI behavior is verified in a real browser.

### VII. Simplicity

One Rust workspace, one `qd` binary. Prefer the standard library, stock
Quarto/reveal.js features, and existing well-maintained crates over custom
code. No speculative abstractions, plugin systems, or configuration until a
concrete need exists. Complexity must be justified in the plan's Complexity
Tracking table.

## Constraints

- Language: Rust (stable) for the core, CLI, and server. The browser UI is
  plain HTML/CSS/JS served by the binary unless a plan justifies a framework.
- Runs fully locally and offline; the server binds to localhost only.
- External runtime dependency: the Quarto CLI (which bundles Pandoc).
- Decks with code cells also need **uv** (default) or **pixi** (PyPI
  packages, not conda) — only to run cells or render such decks; decks
  without code never do. Each deck has its own locked environment (manifest
  and lockfile beside the deck or at the Quarto project root, committed with
  it); qd never installs into a global Python or uses plain pip.
- Code runs only on an explicit action: the person's Run click or key in the
  editor, or a `qd run` command. The server's code-running endpoints MUST
  require the per-session token, which is never served to a page that did not
  come from the editor link.
- License: proprietary, personal use only (`LICENSE`). Third-party
  dependencies MUST have licenses compatible with private use and are listed
  in each plan.
- Target platform: macOS first; nothing should preclude Linux.

## Development Workflow

- Spec-kit flow for every feature: specify → (clarify) → plan → tasks →
  implement. Specs live in `specs/NNN-name/`.
- Each plan includes a Constitution Check against Principles I–VII.
- Small, atomic commits; tests pass before merge.
- Before a PR is marked ready, it passes a **concision audit** (`/pr-audit`):
  a few sub-agents review the PR's diff with the ponytail review rules —
  source code and infrastructure alike (CI, Cargo dependencies, test
  harnesses, fixtures, scripts) — looking for redundancy, reinvented library
  features, unneeded dependencies and speculative abstraction. Findings are
  verified; those that hold are applied without changing behaviour (tests,
  fmt, clippy still pass), the rest are recorded as skipped with a reason in
  the PR description. The audit never removes tests, safety checks,
  validation at trust boundaries, or anything a principle above requires.

## Governance

This constitution supersedes other practices. Amendments are made with
`/speckit-constitution`, recorded in the Sync Impact Report, and versioned
with semantic versioning (MAJOR: principle removed or redefined; MINOR:
principle or section added; PATCH: wording). Every plan and review verifies
compliance.

**Version**: 1.2.0 | **Ratified**: 2026-11-03 | **Last Amended**: 2026-11-04
