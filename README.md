# coffeegent

Linux user-level daemon that holds a systemd-logind sleep inhibitor while AI coding
agents (Claude Code and Cursor in v0.1) are working, and releases it when they idle,
the battery is low, or the machine is too hot — including with the laptop lid closed.

**Status:** The core decides when a sleep lock should be held: session state,
cutoffs, pause, reachable, and mode. The daemon, hooks, and tray are not
wired up yet ([`docs/SPEC.md`](docs/SPEC.md) §15).

## Known limitations

- If a low-battery or thermal cutoff releases the sleep lock and the OS still
  does not suspend (for example a docked laptop with an external monitor), the
  machine stays awake. coffeegent can only notify.
- UPower's own critical action may ignore a block lock. The default battery
  cutoff is 20% so the hold can drop before that action.

## Specification and rulings

- [`docs/SPEC.md`](docs/SPEC.md) — full product spec (v0.2).
- [`docs/rulings.md`](docs/rulings.md) — four confirmed implementation rulings (2026-11-07).
- [`docs/reference/inhibit_spike.rs`](docs/reference/inhibit_spike.rs) — Phase 0 logind inhibitor spike.

## Build and test

```bash
cargo build
coffeegent --version
cargo test --workspace
```

Set log verbosity with `COFFEEGENT_LOG` (for example `COFFEEGENT_LOG=debug`).

## CI

GitHub Actions runs `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
and `cargo test --workspace` (see [`.github/workflows/ci.yml`](.github/workflows/ci.yml)).

## License

MIT — see [`LICENSE`](LICENSE).
