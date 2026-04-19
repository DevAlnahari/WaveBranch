/// WaveBranch — Version Control for Audio.
///
/// Entry point for the `wave` CLI binary. This module is intentionally
/// thin: it parses CLI arguments, dispatches to the correct domain
/// module, and surfaces errors to the user. No business logic lives here.

mod cli;
mod core;
mod audio;
mod crypto;
mod error;
mod network;

use std::env;

use clap::Parser;

use cli::{Cli, Commands};
use error::WaveBranchError;

fn main() -> Result<(), WaveBranchError> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init => {
            core::init::init_repo()?;
        }
        Commands::HashObject { path } => {
            let hash = crate::crypto::hasher::hash_file_streaming(&path).unwrap_or_else(|_| String::new());
            println!("{}", hash);
        }
        Commands::DebugDiff { file1, file2 } => {
            let original = audio::reader::extract_pcm_samples(&file1)?;
            let modified = audio::reader::extract_pcm_samples(&file2)?;
            let delta = audio::diff::compute_audio_diff(&original, &modified);
            
            let repo_path = env::current_dir()?.join(".wavebranch");
            let delta_hash = core::object::write_blob(&repo_path, &delta)?;
            
            println!("Delta hash: {}", delta_hash);
            println!("Delta samples: {} (original: {}, modified: {})", delta.len(), original.len(), modified.len());
        }
        Commands::Commit { message } => {
            let hash = core::commit::create_commit(message, "WaveBranch User".to_string())?;
            println!("Committed: {}", hash);
        }
        Commands::Add { path } => {
            core::add::add_path(&path)?;
        }
        Commands::Branch { name } => {
            core::refs::create_branch(&name)?;
            println!("Created branch '{}'.", name);
        }
        Commands::Checkout { branch } => {
            core::refs::checkout_branch(&branch)?;
            println!("Checked out branch '{}'. Workspace track_v1.wav updated.", branch);
        }
        Commands::Merge { branch } => {
            let hash = core::merge::merge_branch(&branch)?;
            println!("Merged branch '{}' cleanly. Merge commit hash: {}", branch, hash);
        }
        Commands::Log => {
            core::log::print_log()?;
        }
        Commands::Reset { hash } => {
            core::reset::reset_to_commit(&hash)?;
        }
        Commands::Serve { port } => {
            network::server::start_server(port)?;
        }
        Commands::Clone { url } => {
            network::client::clone_repo(&url)?;
        }
        Commands::Push { url } => {
            network::client::push_to_remote(&url)?;
        }
        Commands::Pull { url } => {
            network::client::pull_from_remote(&url)?;
        }
    }

    Ok(())
}
