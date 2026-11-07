---
type: epic
title: The repo builds in CI
parent: initiative-coffeegent
covers: ["R12"]
sources:
  - _bmad-output/initiative-coffeegent/initiative-coffeegent.md
  - docs/SPEC.md
refined: true
risk: low
---

# The repo builds in CI

## Description

M0 from SPEC §15. The workspace, MIT license, CI, README skeleton, `coffeegent --version`, tracing, and the copied inhibit spike exist. This epic carries the four rulings confirmed on 2026-11-07 so later milestones inherit one baseline.

## Outcome

A clone of the repo builds in CI, and the four confirmed rulings have a home later epics can cite.

## Requirements

- R1 (parent R12): The workspace, MIT license, CI, README skeleton, `coffeegent --version`, tracing, and the copied inhibit spike exist. M0 owns this.

## Constraints

- Rust, edition 2021, MIT, and no network (parent constraints).
- The four rulings confirmed on 2026-11-07 are recorded on the initiative and bind M1, M2, M4, and M5.
- Each milestone ends fmt-clean, clippy `-D warnings`, tests passing, public items documented, no TODO without an issue, and a README update when the user can see the change (SPEC §15).

## Done when

- CI green
- fmt, clippy (`-D warnings`), tests pass; public items documented; no TODO without an issue; README updated for user-visible changes.
- The scaffold is what CI builds for every later milestone.

## Boundaries

- Product behavior, the daemon, and the hook belong to later epics.
- Must not change: the decisions in `docs/SPEC.md`.

## References

- parent — _bmad-output/initiative-coffeegent/initiative-coffeegent.md, Requirements R12
- spec — docs/SPEC.md, §15 M0 and §3.2

## Notes

- Decision: 2026-11-07 — tracer bullet is story 1, this epic's only story. A refactor sweep was declined with the one-to-one milestone map.
- Decision: 2026-11-07 — this epic is the home of the four confirmed rulings. M1, M2, M4, and M5 wait on it.
