/// Clap-derived CLI definition for WaveBranch.
///
/// Uses the `derive` API (v4) for compile-time validation of argument
/// structure. Each variant of `Commands` maps 1:1 to a top-level
/// subcommand exposed by the `wave` binary.
use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Root CLI parser for the `wave` binary.
///
/// All subcommands are dispatched through the `command` field.
/// Adding a new command only requires extending the `Commands` enum —
/// no changes to parsing logic are needed.
#[derive(Parser, Debug)]
#[command(
    name = "wave",
    about = "WaveBranch — Version Control for Audio",
    version
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Enumeration of all supported subcommands.
///
/// Each variant carries its own arguments (if any) and is routed
/// to the corresponding domain module in `main.rs`.
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new WaveBranch repository in the current directory.
    Init,

    /// Compute the content-addressable SHA-256 hash of a WAV file's
    /// raw PCM audio data (OOM-safe streaming mode).
    #[command(name = "hash-object")]
    HashObject {
        /// Path to the `.wav` file to hash.
        path: PathBuf,
    },

    /// [Debug] Compute the audio diff (phase cancellation) between two
    /// WAV files, store the delta as a compressed blob, and print its hash.
    #[command(name = "debug-diff")]
    DebugDiff {
        /// Path to the original (base) `.wav` file.
        file1: PathBuf,
        /// Path to the modified `.wav` file.
        file2: PathBuf,
    },

    /// Create a new commit capturing the current state of tracked files.
    Commit {
        /// The commit message.
        #[arg(short, long)]
        message: String,
    },

    /// Add a track or directory to the staging area to be committed.
    Add {
        /// Path to the WAV file(s). Use `.` to add everything in the directory.
        path: String,
    },

    /// Create a new branch pointing to the current commit.
    Branch {
        /// The name of the new branch to create.
        name: String,
    },

    /// Switch HEAD to a specific branch and restore its contents to the workspace.
    Checkout {
        /// The branch name to checkout.
        branch: String,
    },

    /// Combine a target branch into the current HEAD utilizing physical audio merging.
    Merge {
        /// The target branch name to merge into current.
        branch: String,
    },

    /// View chronological history backwards from current HEAD commit mapping.
    Log,

    /// Revert working workspace and internal Index bounds exactly to historical target pointer.
    Reset {
        /// The Commit hash explicitly mapping backward bounds natively.
        hash: String,
    },

    /// Start the distributed Network server listening to generic TCP limits.
    Serve {
        /// The port explicitly wrapping boundaries tracking tcp targets.
        #[arg(short, long, default_value = "8080")]
        port: u16,
    },

    /// Fetch entire repositories tracking across TCP networks cloning into a fresh layout locally.
    Clone {
        /// The explicit target path IP mappings properly executing.
        url: String,
    },

    /// Send missing Blobs/Commits synchronously into the Remote repository pointer effectively matching logic.
    Push {
        /// Target target path IP.
        url: String,
    },

    /// Download and sync remote pointers implicitly resetting heads reliably mimicking Git explicitly perfectly overriding explicitly correctly implicitly.
    Pull {
        /// Download destination path IP.
        url: String,
    },
}
