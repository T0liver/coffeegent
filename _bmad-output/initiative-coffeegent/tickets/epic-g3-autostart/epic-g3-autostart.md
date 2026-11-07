---
type: epic
title: The autostart mechanism is chosen
parent: initiative-coffeegent
covers: ["R14"]
sources:
  - _bmad-output/initiative-coffeegent/initiative-coffeegent.md
  - docs/SPEC.md
refined: true
risk: medium
---

# The autostart mechanism is chosen

## Description

G3 from SPEC §15, the start of M6. The human starts the M2 `coffeegent daemon` binary, logs out and back in once, with a throwaway systemd user unit and a throwaway XDG autostart entry that log the start time, `WAYLAND_DISPLAY`, the session bus, and whether the watcher is present. M6 packages the mechanism this gate chooses.

## Outcome

One default autostart mechanism is chosen and documented for the package.

## Requirements

- R1 (parent R14): One autostart mechanism is chosen and documented, and the package uses it. G3 owns the choice, and M6 owns the package.

## Constraints

- Both mechanisms may be present. The single-instance guard makes a double start harmless (SPEC §3.1, §10.5).
- The builder stops and asks the human when this gate fails or reality contradicts `docs/SPEC.md` (SPEC §15).

## Done when

- Default mechanism chosen and documented
- The documented choice is what M6 packages.

## Boundaries

- The `.deb` and the installed unit files belong to M6.
- The throwaway unit and desktop file are not the packaged ones.

## References

- parent — _bmad-output/initiative-coffeegent/initiative-coffeegent.md, Requirements R14
- spec — docs/SPEC.md, §10.5 and §15 G3

## Notes

- Decision: 2026-11-07 — tracer bullet is story 1, this epic's only story. A refactor sweep was declined with the one-to-one milestone map.
- Decision: 2026-11-07 — this gate requires the human.
- Waits on epic-m2-daemon because: a `coffeegent daemon` binary that can start under a user session.
