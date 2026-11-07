---
type: epic
title: Codex drives working and idle
parent: initiative-coffeegent
covers: ["R11"]
sources:
  - _bmad-output/initiative-coffeegent/initiative-coffeegent.md
  - docs/SPEC.md
refined: true
risk: high
---

# Codex drives working and idle

## Description

M7 from SPEC §15. After the M6 v0.1 release, the human installs Codex. The work observes real notify payloads and rollout files, then adds the adapter, the chained installer, and the file-activity detector on M4's hook and installer pattern. An existing notify command stays in place.

## Outcome

A Codex session drives working and idle, and an existing notify command is preserved.

## Requirements

- R1 (parent R11): A Codex session drives working and idle, and an existing notify command is left in place. M7 owns this.

## Constraints

- The installer must not replace an existing `notify` command. It chains through `coffeegent hook codex -- <original argv...>` (SPEC §9.2).
- Codex stays out of the v0.1 checklist (SPEC §1, §15).
- The builder stops and asks the human when Codex is not installed or reality contradicts `docs/SPEC.md` (SPEC §15).

## Done when

- Codex session drives Working/Idle; existing `notify` preserved
- fmt, clippy (`-D warnings`), tests pass; public items documented; no TODO without an issue; README updated for user-visible changes.
- The Codex path is in the same daemon the v0.1 release built.

## Boundaries

- Heuristic detection of arbitrary agents, a D-Bus control interface, and a preferences GUI stay in SPEC §14, outside this epic.
- Must not change: v0.1 hook installers for Claude and Cursor.

## References

- parent — _bmad-output/initiative-coffeegent/initiative-coffeegent.md, Requirements R11
- spec — docs/SPEC.md, §9.2, §9.3, and §15 M7

## Notes

- Decision: 2026-11-07 — tracer bullet is story 1, this epic's only story. A refactor sweep was declined with the one-to-one milestone map.
- Waits on epic-m6-release because: v0.1 released so Codex stays outside that checklist.
- Waits on epic-m4-hooks because: the hook and installer pattern.
