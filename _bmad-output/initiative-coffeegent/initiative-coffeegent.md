---
type: initiative
title: coffeegent
parent: none
sources:
  - docs/SPEC.md
companions:
  - docs/SPEC.md
risk: high
---

# coffeegent

## Description

coffeegent keeps a Pop!_OS laptop awake, including with the lid closed, while Claude Code or Cursor is working, and releases the lock when they are idle, the battery is low, or the machine is too hot. Codex follows in M7. The behavior and the measurements are `docs/SPEC.md` (v0.2). This initiative slices that spec one-to-one onto milestones M0–M7 and gates G1–G3.

## Outcome

For the maintainer on Pop!_OS 24.04 with COSMIC, the laptop stays awake with the lid closed only while a tracked agent is working, and a fresh install passes the manual checklist in SPEC §11.6.

## Requirements

- R1 (SPEC §2 Goals 1, §4.3, §5): While any tracked Claude Code or Cursor session is working, or waiting within the grace period, the daemon holds the logind sleep lock. M1 owns the pure decision, M2 applies it in the daemon, and M4 feeds it from real hook events.
- R2 (SPEC §2 Goals 2, §6.1): While that lock is held, closing the lid keeps the machine awake without root, and the inhibitor includes handle-lid-switch. M3 owns the real logind lock.
- R3 (SPEC §2 Goals 3, §6.1): When the lid closes while the lock is held, the screen locks, or the user is notified that it could not be locked. M3 owns this.
- R4 (SPEC §2 Goals 4, §5): A low battery or too-high temperature releases the hold according to the cutoff rules in SPEC §5. M1 owns the pure latch, and M3 owns the live power and thermal readings.
- R5 (SPEC §2 Goals 5, §10.1, §10.2): The user can pause, set a stay-reachable timer, and switch mode from the CLI, and from the tray when the tray gate says the tray ships. M2 owns the CLI and saved state, and M5 owns the tray.
- R6 (SPEC §2 Goals 6, §10.3): The user is notified when Claude is waiting, when an agent finishes, and when a cutoff releases the lock. M5 owns this.
- R7 (SPEC §2 Goals 7, §0 item 3, §4.3): If the daemon dies, the OS releases the lock. If an agent dies, its session is removed. M3 shows the lock is gone after SIGKILL, and M2 removes a session whose pid is gone.
- R8 (SPEC §2 Goals 8, §10): `coffeegent doctor` reports the lock path, the power and thermal sensors it selected, and the autostart mechanism it observed. M5 owns doctor.
- R9 (SPEC §13, §11.6, §15 M6): A fresh install on the target laptop from the .deb, and a `cargo install` build, passes the manual checklist in SPEC §11.6. M6 owns this.
- R10 (SPEC §9.2, §15 G1, §15 M4): Sanitized real Claude Code and Cursor fixtures exist, section 9.2 matches what was observed, `working_ttl_secs` is set from those gaps, and the adapters follow that mapping. G1 owns the fixtures and the default, and M4 owns the adapters and installers.
- R11 (SPEC §9.2 Codex, §15 M7): A Codex session drives working and idle, and an existing notify command is left in place. M7 owns this.
- R12 (SPEC §15 M0, §3.2): The workspace, MIT license, CI, README skeleton, `coffeegent --version`, tracing, and the copied inhibit spike exist. M0 owns this.
- R13 (SPEC §10.2, §15 G2, §15 M5): The COSMIC tray gate result is recorded, and the tray milestone ships a tray or ships notifications and the CLI only, as that result requires. G2 owns the result, and M5 owns the shipping behavior.
- R14 (SPEC §10.5, §15 G3, §15 M6): One autostart mechanism is chosen and documented, and the package uses it. G3 owns the choice, and M6 owns the package.

## Constraints

- The daemon is one root-less user process: no setuid, no polkit rule, no root helper, and no network access (SPEC §1, §12).
- The language is Rust, edition 2021, the license is MIT, and the core crate has no I/O and forbids unsafe code (SPEC §1, §3.2, §12).
- `coffeegent hook` uses only the standard library, always exits 0, prints nothing, and finishes within 150 ms (SPEC §3.1, §11.5). Confirmed 2026-11-07.
- While the inhibitor is held, `handle-lid-switch` is included. `LockedHint` is not a lock signal (SPEC §0, §6.1).
- Adapters and logs never carry prompt content, file contents, or environment variables (SPEC §9.1, §12).
- The tray is optional. A missing StatusNotifier watcher does not stop the daemon (SPEC §10.2).
- Claude Code and Cursor are the v0.1 agents. Codex stays off until M7 (SPEC §1, §9.2).
- Saved `mode` in `state.json` wins over `config.toml` once it has been saved. `reload` does not change mode, pause, or reachable (confirmed 2026-11-07; SPEC §7, §8).
- `min_hold_secs` delays only a release because work ended. A latched cutoff, pause, and mode off release the lock immediately (confirmed 2026-11-07; SPEC §5).
- `toggle` flips only `while_working` and `off`. From `always` it goes to `off`. From `off` it goes to `while_working` (confirmed 2026-11-07; SPEC §10.1).
- Package install never edits agent config files (SPEC §13).
- Each milestone ends fmt-clean, clippy `-D warnings`, tests passing, public items documented, no TODO without an issue, and a README update when the user can see the change (SPEC §15).
- The builder stops and asks the human when a gate fails or reality contradicts `docs/SPEC.md` (SPEC §15).

## Done when

- A fresh install on the target laptop passes the manual checklist in SPEC §11.6.
- Claude Code and Cursor keep the machine awake with the lid closed while they work, and the lock releases when they are idle, the battery is low, or the machine is too hot.
- `coffeegent doctor` and the CLI work when no tray is present.
- A Codex session drives working and idle, and an existing notify command is preserved.

## Boundaries

- Out: Codex in v0.1, heuristic detection, any agent besides Claude Code, Cursor, and (in M7) Codex, display control, docked-mode behavior beyond documenting it, a preferences GUI, automatic hotkey registration, Flatpak or Snap, X11-specific features, wake-on-LAN, non-Linux, telemetry, an updater, and a license check (SPEC §2).
- Tracer path: install the binary, a Claude or Cursor hook emits Working, the daemon holds the lid lock, and idle or a cutoff releases it. M7 repeats that path for Codex.
- Touch point: systemd-logind — the inhibitor fd and the session lock; owner: epic-m3-backends
- Touch point: UPower and sysfs power_supply — battery snapshot; owner: epic-m3-backends
- Touch point: sysfs thermal zones — temperature samples; owner: epic-m3-backends
- Touch point: `~/.claude/settings.json` and `~/.cursor/hooks.json` — hook install only; owner: epic-m4-hooks
- Touch point: `~/.codex/config.toml` — chained notify, in M7; owner: epic-m7-codex
- Touch point: the COSMIC StatusNotifier watcher and freedesktop notifications; owner: epic-m5-tray
- Touch point: the systemd user unit and the XDG autostart file; owner: epic-m6-release
- Touch point: GitHub Actions CI; owner: epic-m0-scaffold
- Touch point: `cosmic-idle` — the SPEC §11.6e checklist configures its suspend timers and records whether the sleep lock blocks them; owner: epic-m6-release. A `org.freedesktop.ScreenSaver.Inhibit` backend stays out unless that check fails, and then the builder asks (SPEC §16 D2).

## References

- spec — docs/SPEC.md, the approved v0.2 specification (sections 2, 5, 9, 10, 11, 15, and 16 bind this initiative)
- spec — docs/SPEC.md, adopted companion; the milestone text in §15 is the story acceptance criteria

## Notes

- Decision: 2026-11-07 — once `mode` is saved in `state.json` it wins over `config.toml`. `coffeegent mode` and `toggle` write state only. `reload` refreshes the rest of the config and leaves mode, pause, and reachable. Config `mode` applies only when state has no mode. User confirmed.
- Decision: 2026-11-07 — `min_hold_secs` delays only a release because work ended. A latched cutoff, pause, and mode off release the lock immediately. User confirmed.
- Decision: 2026-11-07 — `toggle` flips only `while_working` and `off`. From `always` it goes to `off`. From `off` it goes to `while_working`. User confirmed.
- Decision: 2026-11-07 — `coffeegent hook` always exits 0. Exit codes 0, 1, and 2 apply to the other commands. User confirmed.
- Decision: 2026-11-07 — one epic and one story for each of M0–M7 and G1–G3. Acceptance criteria are copied verbatim from SPEC §15. A finer story split was declined. Each epic's single story is its tracer bullet. A refactor sweep was declined.
- Decision: 2026-11-07 — G1, G2, and G3 require the human. M3, M4, M5, M6, and M7 also require the human because their acceptance criteria run on the maintainer's machine or agents.
- Decision: 2026-11-07 — `docs/SPEC.md` stays the authority where it tags `LockedHint` as OBSERVED and the Phase 0 report tagged it ASSUMED. Do not use `LockedHint`.
- Decision: 2026-11-07 — the four rulings live here and are carried by epic M0. M1, M2, M4, and M5 wait on M0 for them.
- Decision: 2026-11-07 — `cosmic-idle` is a touch point owned by epic M6, because SPEC §11.6e configures its timers and records the result. A ScreenSaver inhibit backend stays out unless that check fails, and then the builder asks (SPEC §16 D2).
- Open question: D1, docked with an external monitor — epic-m6-release records the result (SPEC §16).
- Open question: D2, whether the sleep lock blocks cosmic-idle — epic-m6-release checklist item 11.6e (SPEC §16).
- Open question: D3, the physical panel while the lid is closed — document only; display control is out (SPEC §16).
- Open question: D4, long closed-lid runs, low battery, and UPower's critical action — epic-m6-release checklist (SPEC §16).
- Open question: D5, lock-on-close on COSMIC — epic-m3-backends records that it works, or the notification fallback (SPEC §16).
- Open question: D6, thermal cutoff only when the lid is closed, under closed-lid load — epic-m3-backends and epic-m6-release record what the machine does (SPEC §16).
- The Phase 0 report was read at `/home/oliv/coffeegent-spike/PHASE0_REPORT.md`. It is not in this repo, so it is not a source. No measured result in it contradicted a decision in `docs/SPEC.md`.
