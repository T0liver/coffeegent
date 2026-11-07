---
type: epic
title: The COSMIC tray gate is recorded
parent: initiative-coffeegent
covers: ["R13"]
sources:
  - _bmad-output/initiative-coffeegent/initiative-coffeegent.md
  - docs/SPEC.md
refined: true
risk: high
---

# The COSMIC tray gate is recorded

## Description

G2 from SPEC §15, the start of M5. The human runs a minimal ksni probe on COSMIC: icon, menu, notification, survival of a Status Area restart, and a daemon that starts before the watcher. The result decides what M5 ships.

## Outcome

A recorded tray result that M5 follows, including the CLI-only fallback if ksni fails.

## Requirements

- R1 (parent R13): The COSMIC tray gate result is recorded, and the tray milestone ships a tray or ships notifications and the CLI only, as that result requires. G2 owns the result, and M5 owns the shipping behavior.

## Constraints

- A missing watcher must not be treated as a failed daemon start (SPEC §10.2).
- The builder stops and asks the human when this gate fails or reality contradicts `docs/SPEC.md` (SPEC §15).

## Done when

- Result recorded; if ksni fails, M5 ships notifications + CLI only and logs a tray issue
- The recorded result is what M5 implements.

## Boundaries

- Shipping the tray, the icons, and doctor belongs to M5.
- The probe is throwaway and does not edit the packaged daemon.

## References

- parent — _bmad-output/initiative-coffeegent/initiative-coffeegent.md, Requirements R13
- spec — docs/SPEC.md, §10.2 and §15 G2

## Notes

- Decision: 2026-11-07 — tracer bullet is story 1, this epic's only story. A refactor sweep was declined with the one-to-one milestone map.
- Decision: 2026-11-07 — this gate requires the human.
