//! `coffeegent` binary entry point (daemon and CLI land in later milestones).

mod adapters;
mod backends;
mod cli;
mod daemon;
mod doctor;
mod hook;
mod install;
mod ui;

fn main() {
    if let Err(err) = cli::run() {
        eprintln!("{err:#}");
        std::process::exit(1);
    }
}
