---
type: epic
title: Tray, notifications, and doctor follow the gate
parent: initiative-coffeegent
covers: ["R5", "R6", "R8", "R13"]
sources:
  - _bmad-output/initiative-coffeegent/initiative-coffeegent.md
  - docs/SPEC.md
refined: true
risk: medium
---

# Tray, notifications, and doctor follow the gate

## Description

M5 from SPEC §15. Notifications and doctor ship in every case, on the daemon M4 already drives with real sessions, and the tray's toggle uses the M0 ruling. The tray ships only when the G2 result says ksni works on COSMIC. M6 packages this tray-or-CLI behavior and doctor. The human reviews the result on COSMIC.

## Outcome

Notifications are throttled, doctor reports the machine, and the tray matches the recorded gate.

## Requirements

- R1 (parent R5): The user can pause, set a stay-reachable timer, and switch mode from the CLI, and from the tray when the tray gate says the tray ships. M2 owns the CLI and saved state, and M5 owns the tray.
- R2 (parent R6): The user is notified when Claude is waiting, when an agent finishes, and when a cutoff releases the lock. M5 owns this.
- R3 (parent R8): `coffeegent doctor` reports the lock path, the power and thermal sensors it selected, and the autostart mechanism it observed. M5 owns doctor.
- R4 (parent R13): The COSMIC tray gate result is recorded, and the tray milestone ships a tray or ships notifications and the CLI only, as that result requires. G2 owns the result, and M5 owns the shipping behavior.

## Constraints

- The tray is optional. A missing watcher does not stop the daemon (SPEC §10.2).
- `toggle` from the tray uses the same two-state flip as the CLI (confirmed 2026-11-07).
- If G2 records a ksni failure, this epic ships notifications and the CLI only and logs a tray issue (SPEC §15 G2).

## Done when

- Reviewed on COSMIC; notifications throttled
- fmt, clippy (`-D warnings`), tests pass; public items documented; no TODO without an issue; README updated for user-visible changes.
- Doctor and the notifications are what the packaged daemon runs.

## Boundaries

- Choosing the autostart default belongs to G3. Packaging it belongs to M6.
- Must not change: waiting notifications are Claude-only (SPEC §10.3).

## References

- parent — _bmad-output/initiative-coffeegent/initiative-coffeegent.md, Requirements R5, R6, R8, and R13
- spec — docs/SPEC.md, §10 and §15 M5

## Notes

- Decision: 2026-11-07 — tracer bullet is story 1, this epic's only story. A refactor sweep was declined with the one-to-one milestone map.
- Waits on epic-g2-tray because: the recorded ksni result.
- Waits on epic-m4-hooks because: a daemon that already runs real sessions.
- Waits on epic-m0-scaffold because: the toggle ruling.
