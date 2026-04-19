use std::env;
use std::fs;

use chrono::Utc;

use crate::audio::merge::three_way_audio_merge;
use crate::core::object::{read_blob, read_object, write_blob, write_object};
use crate::core::refs::checkout_branch; // We can reuse logic or just trigger checkout
use crate::core::types::{Commit, Tree, TreeEntry};
use crate::error::WaveBranchError;

/// Combine a target branch into the current HEAD utilizing physical audio merging.
pub fn merge_branch(target_branch: &str) -> Result<String, WaveBranchError> {
    let cwd = env::current_dir()?;
    let repo_path = cwd.join(".wavebranch");

    // 1. Find the current branch pointer
    let head_path = repo_path.join("HEAD");
    let head_content = fs::read_to_string(&head_path)?;
    let head_ref = head_content.trim().trim_start_matches("ref: ");
    let main_ref_path = repo_path.join(head_ref);
    let current_commit_hash = fs::read_to_string(&main_ref_path)?;
    let current_commit_hash = current_commit_hash.trim();
    let current_commit: Commit = read_object(&repo_path, current_commit_hash)?;

    // 2. Find the target branch pointer
    let target_branch_path = repo_path.join("refs").join("heads").join(target_branch);
    if !target_branch_path.exists() {
        return Err(WaveBranchError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Branch '{}' does not exist.", target_branch),
        )));
    }
    let target_commit_hash = fs::read_to_string(&target_branch_path)?;
    let target_commit_hash = target_commit_hash.trim();
    let target_commit: Commit = read_object(&repo_path, target_commit_hash)?;

    // 3. Find the Common Ancestor
    // Phase 4 simplification: Assumes the target branch's parent is the base commit.
    let base_commit_hash = target_commit.parent_hash.as_ref().ok_or_else(|| {
        WaveBranchError::IoError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Target branch has no parent; cannot determine a common base for 3-way merge.",
        ))
    })?;
    let base_commit: Commit = read_object(&repo_path, base_commit_hash)?;

    // 4. Extract track_v1.wav Blob Hash from Base, Current, Target
    let load_audio = |commit: &Commit| -> Result<Vec<i16>, WaveBranchError> {
        let tree: Tree = read_object(&repo_path, &commit.tree_hash)?;
        let hash = tree.entries.iter()
            .find(|e| e.name == "track_v1.wav")
            .map(|e| &e.hash)
            .ok_or_else(|| WaveBranchError::IoError(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "track_v1.wav not found in tree."
            )))?;
        Ok(read_blob(&repo_path, hash)?.samples)
    };

    let base_samples = load_audio(&base_commit)?;
    let current_samples = load_audio(&current_commit)?;
    let target_samples = load_audio(&target_commit)?;

    // 5. Execute Mathematical DSP 3-Way Merge
    let merged_samples = three_way_audio_merge(&base_samples, &current_samples, &target_samples);

    // 6. Save new Merged Blob
    let merged_blob_hash = write_blob(&repo_path, &merged_samples)?;

    // 7. Write new Merged Tree
    let merged_tree = Tree {
        entries: vec![TreeEntry {
            mode: "100644".to_string(),
            name: "track_v1.wav".to_string(),
            hash: merged_blob_hash,
        }],
    };
    let merged_tree_hash = write_object(&repo_path, &merged_tree)?;

    // 8. Create Multi-Parent Merge Commit
    // Realistically, second parent should be inside a Vec, but since Commit layout 
    // only holds Option<String> in Phase 3, we leave the parent pointer pointing to our current head
    // to preserve linearity or we drop the secondary parent tracking until Phase 5.
    let merge_commit = Commit {
        tree_hash: merged_tree_hash,
        parent_hash: Some(current_commit_hash.to_string()),
        author: "MergeBot".to_string(),
        timestamp: Utc::now().timestamp(),
        message: format!("Merge branch '{}' into current", target_branch),
    };
    let merge_commit_hash = write_object(&repo_path, &merge_commit)?;

    // 9. Automate Pointer and Workspace alignment
    fs::write(&main_ref_path, format!("{}\n", merge_commit_hash))?;
    
    // We can rely simply on checking out the current branch to overwrite the track_v1.wav accurately
    let branch_name = head_ref.replace("refs/heads/", "");
    checkout_branch(&branch_name)?;

    Ok(merge_commit_hash)
}
