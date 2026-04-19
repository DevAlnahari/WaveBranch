/// Repository initialization logic.
///
/// Mirrors Git's `git init` but creates the `.wavebranch/` directory
/// structure instead. The layout is designed for a content-addressable
/// object store where audio blobs are keyed by their PCM SHA-256 hash.
use std::env;
use std::fs;
use std::path::Path;

use crate::error::WaveBranchError;

/// The hidden directory name for a WaveBranch repository.
const REPO_DIR: &str = ".wavebranch";

/// Initializes a new WaveBranch repository in the current working directory.
///
/// Creates the following layout:
/// ```text
/// .wavebranch/
/// ├── objects/       # Content-addressable blob store (SHA-256 keyed)
/// ├── refs/
/// │   └── heads/    # Branch tip pointers (Phase 2)
/// └── HEAD          # Symbolic ref pointing to the active branch
/// ```
///
/// # Errors
/// - `WaveBranchError::RepoAlreadyExists` if `.wavebranch/` already exists.
/// - `WaveBranchError::IoError` on any filesystem failure.
pub fn init_repo() -> Result<(), WaveBranchError> {
    let cwd = env::current_dir()?;
    let repo_path = cwd.join(REPO_DIR);

    if repo_path.exists() {
        return Err(WaveBranchError::RepoAlreadyExists);
    }

    // Create the object store and ref namespace in one pass each.
    fs::create_dir_all(repo_path.join("objects"))?;
    fs::create_dir_all(repo_path.join("refs").join("heads"))?;

    // HEAD is a symbolic reference, not a direct commit pointer (yet).
    // This matches Git's convention: `ref: refs/heads/<branch>`.
    fs::write(repo_path.join("HEAD"), "ref: refs/heads/main\n")?;

    println!(
        "Initialized empty WaveBranch repository in {}",
        display_path(&repo_path)
    );

    Ok(())
}

/// Renders a path for user-facing output.
///
/// Falls back to the debug representation if the path contains
/// non-UTF-8 sequences (rare on modern systems but handled correctly).
fn display_path(path: &Path) -> String {
    path.to_str()
        .map(String::from)
        .unwrap_or_else(|| format!("{:?}", path))
}
