---
type: epic
title: A fresh install passes the manual checklist
parent: initiative-coffeegent
covers: ["R9", "R14"]
sources:
  - _bmad-output/initiative-coffeegent/initiative-coffeegent.md
  - docs/SPEC.md
refined: true
risk: high
---

# A fresh install passes the manual checklist

## Description

M6 from SPEC §15. The `.deb`, the docs, the README support matrix and Security section, and the `v0.1.0` release package G3's autostart choice, M5's tray-or-CLI behavior and doctor, and M4's hooks. Codex stays outside this release for M7. The human runs the SPEC §11.6 checklist on a fresh install, including the `cosmic-idle` timers. Package install does not edit agent configs.

## Outcome

A fresh install on the target laptop passes the manual checklist.

## Requirements

- R1 (parent R9): A fresh install on the target laptop from the `.deb`, and a `cargo install` build, passes the manual checklist in SPEC §11.6. M6 owns this.
- R2 (parent R14): One autostart mechanism is chosen and documented, and the package uses it. G3 owns the choice, and M6 owns the package.

## Constraints

- Package install never edits agent config files (SPEC §13).
- The README Security section explains that a failed lock-on-close leaves the screen unlocked (SPEC §12).
- The builder stops and asks the human when reality contradicts `docs/SPEC.md` (SPEC §15).

## Done when

- Fresh install on target passes the checklist (11.6)
- fmt, clippy (`-D warnings`), tests pass; public items documented; no TODO without an issue; README updated for user-visible changes.
- `v0.1.0` is the release the checklist was run against.

## Boundaries

- Codex belongs to M7 and is outside this release.
- Must not change: the post-install message tells the user to run `coffeegent install-hooks` (SPEC §13).

## References

- parent — _bmad-output/initiative-coffeegent/initiative-coffeegent.md, Requirements R9 and R14
- spec — docs/SPEC.md, §11.6, §12, §13, and §15 M6

## Notes

- Decision: 2026-11-07 — tracer bullet is story 1, this epic's only story. A refactor sweep was declined with the one-to-one milestone map.
- Waits on epic-m5-tray because: tray-or-CLI behavior and doctor.
- Waits on epic-g3-autostart because: the chosen autostart mechanism.
- Waits on epic-m4-hooks because: installable Claude and Cursor hooks.
- Waits on epic-m3-backends because: the real lid lock and cutoffs the checklist exercises.
- Open question: D1, D2, and D4 are recorded by the checklist, not solved in code (SPEC §16). `cosmic-idle` is this epic's touch point for D2. A ScreenSaver inhibit backend stays out unless that check fails, and then the builder asks.
