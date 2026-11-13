---
name: pr-audit
description: Concision audit before a quartz-deck PR is marked ready (constitution, Development Workflow). A few sub-agents review the branch's diff — source code and infrastructure — with the ponytail review rules; findings are verified, applied without behaviour change, or recorded as skipped in the PR description. Use before `gh pr create` or before marking a draft PR ready, and when asked to audit a branch for redundancy or bloat.
---

# PR concision audit

## 1. Scope the diff

```bash
git fetch -q origin && git diff --stat origin/main...HEAD
```

Split the changed files into slices (skip any slice with no changes):

| Slice | Files |
|---|---|
| core | `crates/qd/src/**/*.rs` |
| editor | `crates/qd/src/ui/**` (JS, CSS, HTML) |
| tests | `crates/qd/tests/**/*.rs`, `crates/qd/tests/fixtures/**`, `tests/skills/**` |
| infrastructure | `.github/**`, `Cargo.toml`/`Cargo.lock` (dependencies, features), `.specify/scripts/**`, `.claude/**`, build or helper scripts |

Specs, checklists and other prose under `specs/` are not audited.

## 2. One sub-agent per slice, in parallel

Launch them in a single message (Agent tool, `general-purpose`), each with
this brief, filled in:

> You are auditing one slice of a pull request in the Rust crate at
> `<repo path>` (binary `qd`, a Quarto/reveal.js editor; editor UI is plain
> ES modules with no build step). Slice: **<name>**, files: <list>. See the
> changes with `git diff origin/main...HEAD -- <files>`; read surrounding
> code as needed, and grep the whole repository for existing helpers before
> calling anything new.
>
> Invoke the `ponytail:ponytail-review` skill and review **only this diff**
> by its rules. Look especially for: code that duplicates a helper elsewhere
> in the repo (name the existing one, file:line); hand-rolled versions of
> std / an already-used crate / a browser built-in; dependencies, features,
> CI steps or test helpers that nothing needs; abstractions with one caller;
> long code with an equivalent short form.
>
> Never propose removing or weakening tests, safety/conflict checks,
> validation at trust boundaries, error handling that prevents data loss, or
> accessibility. Do not edit files. Report one line per finding in ponytail
> format (`path:Lline: tag what. replacement.`), at most 10, most valuable
> first, then one line estimating lines saved. Say "no findings" if none.

## 3. Verify and apply

For each finding: read the code yourself. Apply it only if it holds and
preserves behaviour; otherwise mark it skipped with a reason. After applying:

```bash
cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
```

All must pass; if a change breaks anything, revert that change and mark it
skipped.

Commit the applied findings as one commit: `Concision audit: <n> findings
applied (−<lines> lines)`.

## 4. Record it in the PR (required before it is marked ready)

Add to the PR description:

```markdown
## Concision audit
- Applied: <finding> (−N lines) …
- Skipped: <finding> — <reason> …
- Net: −N lines; tests, fmt, clippy pass.
```
