# Specification Quality Checklist: Shared Focus

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-11-05
**Feature**: [spec.md](../spec.md)

## Content Quality

- [x] No implementation details (languages, frameworks, APIs)
- [x] Focused on user value and business needs
- [x] Written for non-technical stakeholders
- [x] All mandatory sections completed

## Requirement Completeness

- [x] No [NEEDS CLARIFICATION] markers remain
- [x] Requirements are testable and unambiguous
- [x] Success criteria are measurable
- [x] Success criteria are technology-agnostic (no implementation details)
- [x] All acceptance scenarios are defined
- [x] Edge cases are identified
- [x] Scope is clearly bounded
- [x] Dependencies and assumptions identified

## Feature Readiness

- [x] All functional requirements have clear acceptance criteria
- [x] User scenarios cover primary flows
- [x] Feature meets measurable outcomes defined in Success Criteria
- [x] No implementation details leak into specification

## Notes

- `qd focus` / `--set` and JSON appear in the spec because the CLI is the
  product's user interface for agents (constitution III), as in specs 001–004;
  no transport, storage or code structure is specified.
- "Same rules that protect running code" (FR-009) refers to spec 004's
  user-visible guarantee (only the person's editor and local qd), not to a
  mechanism.
- Informed defaults instead of clarification markers (see Assumptions):
  one command for both directions (no `qd show-slide` alias); a point request
  moves the view immediately unless the person is typing, otherwise it is
  offered and deferred; selected text capped at 2,000 characters; the most
  recently used tab wins. `/speckit-clarify` can revisit any of these.
