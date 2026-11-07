---
type: epic
title: Real fixtures fix the hook mapping
parent: initiative-coffeegent
covers: ["R10"]
sources:
  - _bmad-output/initiative-coffeegent/initiative-coffeegent.md
  - docs/SPEC.md
refined: true
risk: high
---

# Real fixtures fix the hook mapping

## Description

G1 from SPEC §15, the start of M4. The human records real Claude Code and Cursor sessions, reviews the sanitized fixtures before they are committed, and answers the questions in the milestone scope. This replaces the `working_ttl_secs` default M1 shipped. M4's adapters follow the fixtures and that default. The agent does not invent payloads.

## Outcome

Committed fixtures, a §9.2 mapping marked OBSERVED, and a `working_ttl_secs` default taken from the measured gaps.

## Requirements

- R1 (parent R10): Sanitized real Claude Code and Cursor fixtures exist, section 9.2 matches what was observed, `working_ttl_secs` is set from those gaps, and the adapters follow that mapping. G1 owns the fixtures and the default, and M4 owns the adapters and installers.

## Constraints

- The human reviews sanitized fixtures before they are committed (SPEC §15 G1).
- Adapters and logs never carry prompt content (parent constraint).
- The builder stops and asks the human when this gate fails or reality contradicts `docs/SPEC.md` (SPEC §15).

## Done when

- Fixtures committed; mapping tables in 9.2 corrected to OBSERVED; `working_ttl_secs` default set from data
- The corrected mapping and the default are what M4's adapters follow.

## Boundaries

- Installing the production hooks belongs to M4.
- Must not change: a fixture contains no prompt content.

## References

- parent — _bmad-output/initiative-coffeegent/initiative-coffeegent.md, Requirements R10
- spec — docs/SPEC.md, §9.2 and §15 G1

## Notes

- Decision: 2026-11-07 — tracer bullet is story 1, this epic's only story. A refactor sweep was declined with the one-to-one milestone map.
- Decision: 2026-11-07 — this gate requires the human.
- Waits on epic-m1-core because: a core `working_ttl_secs` default that this gate can replace from measured gaps.
