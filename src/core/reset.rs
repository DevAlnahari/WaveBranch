use std::path::{Path, PathBuf};
use std::env;
use std::fs;
use hound::{WavWriter, WavSpec, SampleFormat};

use crate::error::WaveBranchError;
use crate::core::types::{Commit, Tree};
use crate::core::object::{read_object, read_blob};
use crate::core::index::{Index, IndexEntry, write_index};

pub fn reset_to_commit(target_hash: &str) -> Result<(), WaveBranchError> {
    let current_dir = env::current_dir().map_err(|e| WaveBranchError::IoError(e))?;
    let repo_root = find_repo_root(&current_dir)?;
    let repo_path = repo_root.join(".wavebranch");

    // Read the target Commit
    let commit: Commit = match read_object(&repo_path, target_hash) {
        Ok(c) => c,
        Err(_) => return Err(WaveBranchError::ObjectNotFound(target_hash.to_string())),
    };

    println!("Resetting working directory back to commit: {}...", target_hash);
    
    // Read the Root Tree corresponding to the target commit
    let root_tree: Tree = match read_object(&repo_path, &commit.tree_hash) {
        Ok(t) => t,
        Err(_) => return Err(WaveBranchError::ObjectNotFound(commit.tree_hash.clone())),
    };

    let mut new_index_entries = Vec::new();

    // In a full implementation, we process trees recursively. 
    // Here we'll map structural tree rebuilding logically:
    // WaveBranch Phase 5 stores nested structures. We'll reconstruct the working directory.
    reconstruct_tree(&repo_path, &repo_root, &repo_root, &root_tree, &mut new_index_entries)?;

    // Rewrite the index locally entirely mirroring exactly what we materialized
    let new_index = Index { entries: new_index_entries };
    write_index(&repo_path, &new_index)?;

    // Finally, update our HEAD (if we are on a branch, overwrite the branch pointer)
    let head_path = repo_path.join("HEAD");
    if head_path.exists() {
        let head_content = fs::read_to_string(&head_path).map_err(|e| WaveBranchError::IoError(e))?;
        if head_content.starts_with("ref: ") {
            let ref_path = head_content.trim_start_matches("ref: ").trim();
            let target_ref_path = repo_path.join(ref_path);
            fs::write(&target_ref_path, target_hash).map_err(|e| WaveBranchError::IoError(e))?;
        } else {
            // Detached HEAD or hard hash pointed, overwrite directly to HEAD
            fs::write(&head_path, target_hash).map_err(|e| WaveBranchError::IoError(e))?;
        }
    }

    println!("HEAD is now at {}: {}", target_hash.chars().take(7).collect::<String>(), commit.message);
    Ok(())
}

fn reconstruct_tree(repo_path: &Path, repo_root: &Path, base_path: &Path, tree: &Tree, index_entries: &mut Vec<IndexEntry>) -> Result<(), WaveBranchError> {
    for entry in &tree.entries {
        let entry_path = base_path.join(&entry.name);
        
        if entry.mode == "040000" { // Directory
            if !entry_path.exists() {
                fs::create_dir_all(&entry_path).map_err(|e| WaveBranchError::IoError(e))?;
            }
            let sub_tree: Tree = read_object(repo_path, &entry.hash)?;
            reconstruct_tree(repo_path, repo_root, &entry_path, &sub_tree, index_entries)?;
        } else if entry.mode == "100644" { // File Blob
            let blob = read_blob(repo_path, &entry.hash)?;
            write_physical_wav(&entry_path, &blob.samples)?;
            
            // Re-store exactly back into the Index mappings
            let rel_path = entry_path.strip_prefix(repo_root)
                .map_err(|_| WaveBranchError::PathError("Rel path mismatch".to_string()))?
                .to_string_lossy()
                .replace("\\", "/");
                
            index_entries.push(IndexEntry {
                path: rel_path.to_string(),
                hash: entry.hash.clone(),
            });
        }
    }
    Ok(())
}

fn write_physical_wav(path: &Path, samples: &[i16]) -> Result<(), WaveBranchError> {
    // Standard WAV specs mapped across WaveBranch phases
    let spec = WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    
    // Ensure parent directories exist
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).map_err(|e| WaveBranchError::IoError(e))?;
        }
    }

    let mut writer = WavWriter::create(path, spec).map_err(|e| WaveBranchError::AudioError(e.to_string()))?;
    for &sample in samples {
        writer.write_sample(sample).map_err(|e| WaveBranchError::AudioError(e.to_string()))?;
    }
    writer.finalize().map_err(|e| WaveBranchError::AudioError(e.to_string()))?;
    Ok(())
}

fn find_repo_root(current_dir: &Path) -> Result<PathBuf, WaveBranchError> {
    let mut dir = current_dir;
    loop {
        if dir.join(".wavebranch").exists() {
            return Ok(dir.to_path_buf());
        }
        match dir.parent() {
            Some(p) => dir = p,
            None => return Err(WaveBranchError::RepoNotFound),
        }
    }
}
