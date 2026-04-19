/// CLI module — defines the command-line interface structure.
///
/// Separating CLI parsing from business logic keeps the `main.rs`
/// thin and allows each subcommand to be routed to its domain module
/// without coupling `clap` types to core VCS logic.
pub mod commands;

pub use commands::{Cli, Commands};
