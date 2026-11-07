---
type: epic
title: Claude and Cursor hooks drive sessions
parent: initiative-coffeegent
covers: ["R1", "R10"]
sources:
  - _bmad-output/initiative-coffeegent/initiative-coffeegent.md
  - docs/SPEC.md
refined: true
risk: medium
---

# Claude and Cursor hooks drive sessions

## Description

M4 from SPEC §15. The hook shim, the Claude and Cursor adapters, and the installers follow the G1 fixtures and the G1 `working_ttl_secs` default, and the hook always exits 0 as M0 recorded. Events land in the M3 daemon. M5, M6, and M7 consume these hooks.

## Outcome

A real Claude session moves Working, then Waiting, then Idle, and install followed by uninstall restores the agent config byte for byte.

## Requirements

- R1 (parent R1): While any tracked Claude Code or Cursor session is working, or waiting within the grace period, the daemon holds the logind sleep lock. M1 owns the pure decision, M2 applies it in the daemon, and M4 feeds it from real hook events.
- R2 (parent R10): Sanitized real Claude Code and Cursor fixtures exist, section 9.2 matches what was observed, `working_ttl_secs` is set from those gaps, and the adapters follow that mapping. G1 owns the fixtures and the default, and M4 owns the adapters and installers.

## Constraints

- `coffeegent hook` uses only the standard library, always exits 0, prints nothing, and finishes within 150 ms (confirmed 2026-11-07).
- Installers are idempotent, back up the first change, and uninstall only their own entries (SPEC §9.1).
- Never forward prompt content (SPEC §9.1).

## Done when

- Real sessions drive Working -> Waiting (Claude) -> Idle; install/uninstall round trip is byte-identical
- fmt, clippy (`-D warnings`), tests pass; public items documented; no TODO without an issue; README updated for user-visible changes.
- The hooks are what the v0.1 package tells the user to install.

## Boundaries

- Codex belongs to M7.
- The tray belongs to M5.
- Must not change: the hook command stays synchronous (SPEC §9.1).

## References

- parent — _bmad-output/initiative-coffeegent/initiative-coffeegent.md, Requirements R1 and R10
- spec — docs/SPEC.md, §9 and §15 M4

## Notes

- Decision: 2026-11-07 — tracer bullet is story 1, this epic's only story. A refactor sweep was declined with the one-to-one milestone map.
- Waits on epic-m3-backends because: the daemon with a real-backend seam so hook events land in a running daemon.
- Waits on epic-g1-fixtures because: sanitized fixtures and the `working_ttl_secs` default.
- Waits on epic-m0-scaffold because: the hook always-exit-0 ruling.
