use std::env;
use std::fs;
use hound::{WavWriter, WavSpec, SampleFormat};

use crate::core::object::{read_object, read_blob};
use crate::core::types::{Commit, Tree};
use crate::error::WaveBranchError;

/// Create a new branch pointing to the current commit.
pub fn create_branch(name: &str) -> Result<(), WaveBranchError> {
    let cwd = env::current_dir()?;
    let repo_path = cwd.join(".wavebranch");

    if !repo_path.exists() {
        return Err(WaveBranchError::RepoNotFound);
    }

    let head_path = repo_path.join("HEAD");
    let head_content = fs::read_to_string(&head_path)?;
    let head_ref = head_content.trim().trim_start_matches("ref: ");

    let main_ref_path = repo_path.join(head_ref);
    if !main_ref_path.exists() {
        // Can't create a branch if there is no commit yet
        return Err(WaveBranchError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Current branch has no commits.",
        )));
    }

    let current_commit_hash = fs::read_to_string(&main_ref_path)?;
    let current_commit_hash = current_commit_hash.trim();

    let new_branch_path = repo_path.join("refs").join("heads").join(name);
    fs::write(&new_branch_path, format!("{}\n", current_commit_hash))?;

    Ok(())
}

/// Switch HEAD to a specific branch and restore its contents to the workspace.
pub fn checkout_branch(name: &str) -> Result<(), WaveBranchError> {
    let cwd = env::current_dir()?;
    let repo_path = cwd.join(".wavebranch");

    let branch_path = repo_path.join("refs").join("heads").join(name);
    if !branch_path.exists() {
        return Err(WaveBranchError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Branch '{}' does not exist.", name),
        )));
    }

    // 1. Update HEAD
    let head_path = repo_path.join("HEAD");
    fs::write(&head_path, format!("ref: refs/heads/{}\n", name))?;

    // 2. Read the branch's pointed Commit
    let commit_hash = fs::read_to_string(&branch_path)?;
    let commit_hash = commit_hash.trim();
    let commit: Commit = read_object(&repo_path, commit_hash)?;

    // 3. Read the associated Tree
    let tree: Tree = read_object(&repo_path, &commit.tree_hash)?;

    // 4. Find the file (track_v1.wav) inside the Tree
    // In Phase 4, we assume there's exactly one track
    let entry = tree.entries.iter().find(|e| e.name == "track_v1.wav");
    
    if let Some(track_entry) = entry {
        // 5. Read the Blob samples
        let blob = read_blob(&repo_path, &track_entry.hash)?;

        // 6. Rewrite the physical WAV file
        let spec = WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: SampleFormat::Int,
        };

        let target_file = cwd.join("track_v1.wav");
        let mut writer = WavWriter::create(&target_file, spec)
            .map_err(|e| WaveBranchError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to write WAV file: {}", e)
            )))?;

        for sample in blob.samples {
            writer.write_sample(sample).map_err(|e| WaveBranchError::IoError(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to write sample: {}", e)
            )))?;
        }
        writer.finalize().map_err(|e| WaveBranchError::IoError(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("Failed to finalize WAV file: {}", e)
        )))?;
    }

    Ok(())
}
