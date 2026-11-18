# Specification Quality Checklist: Slide groups

**Purpose**: Validate specification completeness and quality before proceeding to planning
**Created**: 2026-11-17
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

- The spec names `#`/`##` headings, `slide-level`, `navigation-mode` and
  `qd` commands. These are the deck's syntax and the product's interface
  (constitution principles II and III), not implementation choices; specs
  015 and 016 do the same.
- The four design questions were settled with the person before writing:
  a group is a Quarto `#` section; the deck keeps its navigation mode and
  gets a setting for it; collapse, group-as-one actions, and indent and
  outdent are in; numbering stays 1, 2, 3. A group-wide hide was left out.
- How Quarto reads sections was checked by rendering small decks with
  Quarto 1.11, not assumed: each `#` and the `##` slides after it render
  as one vertical stack in linear mode too; a `---` slide after a `##`
  joins that stack; a hidden `#` hides only the head; with `slide-level:
  3`, the stacks are the `#` sections and `##` section slides sit inside
  them.
- Left for the plan: command names, where the navigation setting sits in
  the editor, the drag threshold, and where collapse state is stored.
