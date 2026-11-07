//! Command-line interface (`clap`). M0 wires `--version`, tracing, and command stubs.

use std::process::ExitCode;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use tracing::info;

/// User-facing CLI for coffeegent (SPEC §10).
#[derive(Debug, Parser)]
#[command(
    name = "coffeegent",
    version = coffeegent_core::VERSION,
    about = "Hold a sleep lock while AI coding agents are working"
)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

/// Subcommands; behavior is implemented milestone by milestone (SPEC §15).
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run the background daemon (M2).
    Daemon {
        /// Use in-memory fake backends for integration tests (M2).
        #[arg(long)]
        backend: Option<String>,
    },
    /// Print daemon status (M2).
    Status {
        #[arg(long)]
        json: bool,
    },
    /// List tracked sessions (M2).
    Sessions {
        #[arg(long)]
        json: bool,
    },
    /// Flip mode between while-working and off (M2; see `docs/rulings.md`).
    Toggle,
    /// Pause holds (M2).
    Pause { duration: Option<String> },
    /// Resume from pause (M2).
    Resume,
    /// Stay-reachable timer (M2).
    Reachable { duration: Option<String> },
    /// Set operating mode (M2).
    Mode { mode: Option<String> },
    /// Install agent hooks (M4).
    InstallHooks {
        target: Option<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        yes: bool,
    },
    /// Remove agent hooks (M4).
    UninstallHooks {
        target: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Agent hook entry point — always exit 0 (M4; ruling in `docs/rulings.md`).
    Hook { agent: Option<String> },
    /// Environment and integration checks (M5).
    Doctor {
        #[arg(long)]
        json: bool,
    },
}

/// Initialize `tracing` from `COFFEEGENT_LOG` (SPEC §12).
pub fn init_tracing() {
    use tracing_subscriber::EnvFilter;

    let filter =
        std::env::var("COFFEEGENT_LOG").unwrap_or_else(|_| String::from("coffeegent=info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(filter))
        .with_target(true)
        .try_init();
}

/// Parse argv and run the selected command.
pub fn run() -> Result<()> {
    init_tracing();
    let cli = Cli::parse();
    match cli.command {
        None => {
            info!("coffeegent {}", coffeegent_core::VERSION);
            println!("coffeegent {}", coffeegent_core::VERSION);
            Ok(())
        }
        Some(cmd) => handle_command(cmd),
    }
}

fn handle_command(cmd: Command) -> Result<()> {
    match cmd {
        Command::Hook { .. } => {
            // M4 implements the real hook; M0 documents the always-exit-0 ruling.
            crate::hook::run();
            Ok(())
        }
        Command::Daemon { .. }
        | Command::Status { .. }
        | Command::Sessions { .. }
        | Command::Toggle
        | Command::Pause { .. }
        | Command::Resume
        | Command::Reachable { .. }
        | Command::Mode { .. }
        | Command::InstallHooks { .. }
        | Command::UninstallHooks { .. }
        | Command::Doctor { .. } => {
            bail!("not implemented yet (see SPEC §15 milestone plan)");
        }
    }
}

/// Map a result to process exit codes: success = 0, error = 1 (SPEC §10).
#[allow(dead_code)]
pub fn exit_code_from_result(result: Result<()>) -> ExitCode {
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::from(1),
    }
}
