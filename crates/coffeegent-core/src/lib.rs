//! Pure decision logic for coffeegent: types, config parsing, the session
//! machine, and [`policy::decide`].
//!
//! Time and samples are arguments. This crate does not read the clock, the
//! filesystem, the network, or `/proc`, and it does not log. The daemon (a later
//! milestone) loads config text, checks pids, and passes the results in.
//!
//! Confirmed rulings that bind the pure rules live in `docs/rulings.md`.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod config;
pub mod duration;
pub mod policy;
pub mod session;
pub mod types;

/// Package version shared with the `coffeegent` binary (`coffeegent --version`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
