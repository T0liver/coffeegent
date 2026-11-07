---
type: epic
title: The laptop stays awake and can cut off
parent: initiative-coffeegent
covers: ["R2", "R3", "R4", "R7"]
sources:
  - _bmad-output/initiative-coffeegent/initiative-coffeegent.md
  - docs/SPEC.md
refined: true
risk: high
---

# The laptop stays awake and can cut off

## Description

M3 from SPEC §15. Inside the M2 daemon, the real backends take the logind inhibitor fd, read UPower with a sysfs fallback, poll thermal zones by type name, track the lid, lock the session on lid close, and recompute on resume. M4 lands hook events in this daemon, and the M6 checklist exercises this lock. The human runs the acceptance check on the target laptop.

## Outcome

On the target laptop the lock is visible, a killed daemon drops it, a closed lid behaves as in SPEC §0, and the lock-on-close result is recorded.

## Requirements

- R1 (parent R2): While that lock is held, closing the lid keeps the machine awake without root, and the inhibitor includes handle-lid-switch. M3 owns the real logind lock.
- R2 (parent R3): When the lid closes while the lock is held, the screen locks, or the user is notified that it could not be locked. M3 owns this.
- R3 (parent R4): A low battery or too-high temperature releases the hold according to the cutoff rules in SPEC §5. M1 owns the pure latch, and M3 owns the live power and thermal readings.
- R4 (parent R7): If the daemon dies, the OS releases the lock. If an agent dies, its session is removed. M3 shows the lock is gone after SIGKILL, and M2 removes a session whose pid is gone.

## Constraints

- While the inhibitor is held, `handle-lid-switch` is included. `LockedHint` is not a lock signal (SPEC §0, §6.1).
- No root, no polkit rule, and no network (parent constraints).
- The builder stops and asks the human when reality contradicts `docs/SPEC.md` (SPEC §15).

## Done when

- On the target laptop: `reachable 10m` shows the lock in `systemd-inhibit --list`; SIGKILL drops it; lid-closed run behaves as in section 0; lock-on-close result recorded (works, or notification fallback documented)
- fmt, clippy (`-D warnings`), tests pass; public items documented; no TODO without an issue; README updated for user-visible changes.
- The real backends are what the later checklist and the hook milestone run against.

## Boundaries

- Hook adapters belong to M4. The tray belongs to M5.
- Must not change: discharging means UPower state 2 or sysfs status `Discharging` only (SPEC §5, §6.2).

## References

- parent — _bmad-output/initiative-coffeegent/initiative-coffeegent.md, Requirements R2, R3, R4, and R7
- spec — docs/SPEC.md, §0, §6, and §15 M3

## Notes

- Decision: 2026-11-07 — tracer bullet is story 1, this epic's only story. A refactor sweep was declined with the one-to-one milestone map.
- Waits on epic-m2-daemon because: a daemon that hosts a backend and applies a decision.
- Open question: D5, lock-on-close on COSMIC — this epic records that it works, or the notification fallback (SPEC §16).
- Open question: D6, thermal cutoff only while the lid is closed — this epic records the closed-lid result when the check can show it (SPEC §16).
