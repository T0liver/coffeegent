---
type: epic
title: The daemon applies the decision
parent: initiative-coffeegent
covers: ["R1", "R5", "R7"]
sources:
  - _bmad-output/initiative-coffeegent/initiative-coffeegent.md
  - docs/SPEC.md
refined: true
risk: medium
---

# The daemon applies the decision

## Description

M2 from SPEC §15. The daemon speaks the Unix socket, runs the fake backends, saves state, serves every command, and refuses a second instance. CLI mode and toggle follow the rulings confirmed on 2026-11-07. M3 hosts its real backends in this daemon, and G3 starts this binary.

## Outcome

A fake-backend daemon applies the core decision, and the SPEC §11.4 checks pass.

## Requirements

- R1 (parent R1): While any tracked Claude Code or Cursor session is working, or waiting within the grace period, the daemon holds the logind sleep lock. M1 owns the pure decision, M2 applies it in the daemon, and M4 feeds it from real hook events.
- R2 (parent R5): The user can pause, set a stay-reachable timer, and switch mode from the CLI, and from the tray when the tray gate says the tray ships. M2 owns the CLI and saved state, and M5 owns the tray.
- R3 (parent R7): If the daemon dies, the OS releases the lock. If an agent dies, its session is removed. M3 shows the lock is gone after SIGKILL, and M2 removes a session whose pid is gone.

## Constraints

- Saved `mode` in `state.json` wins once saved. `reload` does not change mode, pause, or reachable (confirmed 2026-11-07).
- `toggle` flips only `while_working` and `off` (confirmed 2026-11-07).
- The socket is user-only and checks peer credentials (SPEC §7).

## Done when

- 11.4 tests pass on `--backend fake`
- fmt, clippy (`-D warnings`), tests pass; public items documented; no TODO without an issue; README updated for user-visible changes.
- The daemon is what M3, M4, and the autostart gate start.

## Boundaries

- The real logind, UPower, and thermal backends belong to M3.
- The tray belongs to M5. This epic's controls are the CLI.
- Must not change: sessions are not persisted (SPEC §7).

## References

- parent — _bmad-output/initiative-coffeegent/initiative-coffeegent.md, Requirements R1, R5, and R7
- spec — docs/SPEC.md, §3.1, §7, §10.1, §11.4, and §15 M2

## Notes

- Decision: 2026-11-07 — tracer bullet is story 1, this epic's only story. A refactor sweep was declined with the one-to-one milestone map.
- Waits on epic-m1-core because: the pure session machine and decide().
- Waits on epic-m0-scaffold because: the mode, toggle, and min_hold rulings.
