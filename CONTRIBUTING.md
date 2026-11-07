# Contributing

coffeegent follows the milestone plan in [`docs/SPEC.md`](docs/SPEC.md) §15.

Before opening a PR:

1. `cargo fmt --all`
2. `cargo clippy --all-targets -- -D warnings`
3. `cargo test --workspace`

Confirmed product rulings live in [`docs/rulings.md`](docs/rulings.md); do not change them
without an explicit maintainer decision.
