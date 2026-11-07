//! Agent hook shim (SPEC §3.1, §11.5). Implemented in M4; M0 records the exit-0 ruling.

/// Hook entry: must always exit 0 and print nothing (see `docs/rulings.md`).
pub fn run() {
    std::process::exit(0);
}
