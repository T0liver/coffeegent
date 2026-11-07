---
type: epic
title: The core decides the hold with no I/O
parent: initiative-coffeegent
covers: ["R1", "R4"]
sources:
  - _bmad-output/initiative-coffeegent/initiative-coffeegent.md
  - docs/SPEC.md
refined: true
risk: medium
status: in-progress
---

# The core decides the hold with no I/O

## Description

M1 from SPEC §15. `coffeegent-core` holds the types, config, session machine, `decide()`, both cutoff latches, and duration parsing, including the `working_ttl_secs` default that G1 may replace from measured gaps. Time and samples are injected. The crate performs no I/O.

## Outcome

The pure decision matches SPEC §4.3 and §5, and the core crate has no I/O dependencies.

## Requirements

- R1 (parent R1): While any tracked Claude Code or Cursor session is working, or waiting within the grace period, the daemon holds the logind sleep lock. M1 owns the pure decision, M2 applies it in the daemon, and M4 feeds it from real hook events.
- R2 (parent R4): A low battery or too-high temperature releases the hold according to the cutoff rules in SPEC §5. M1 owns the pure latch, and M3 owns the live power and thermal readings.

## Constraints

- The core crate has no I/O and forbids unsafe code (parent constraint).
- `min_hold_secs` delays only a release because work ended. A latched cutoff, pause, and mode off release the lock immediately (confirmed 2026-11-07).
- Policy and the session machine stay free of side effects (SPEC §15).

## Done when

- All 11.1 tests pass; core has no I/O deps
- fmt, clippy (`-D warnings`), tests pass; public items documented; no TODO without an issue; README updated for user-visible changes.
- The core is what the daemon in later milestones calls.

## Boundaries

- The socket, the fake backend, and the real logind, UPower, and thermal code belong to M2 and M3.
- Must not change: the precedence and cutoff rules in SPEC §5.

## References

- parent — _bmad-output/initiative-coffeegent/initiative-coffeegent.md, Requirements R1 and R4
- spec — docs/SPEC.md, §4.3, §5, §8, §11.1, and §15 M1

## Notes

- Decision: 2026-11-07 — tracer bullet is story 1, this epic's only story. A refactor sweep was declined with the one-to-one milestone map.
- Waits on epic-m0-scaffold because: the workspace and the four confirmed rulings.
