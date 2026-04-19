use std::path::{Path, PathBuf};
use std::env;

use chrono::{TimeZone, Utc};

use crate::error::WaveBranchError;
use crate::core::types::Commit;
use crate::core::object::read_object;

pub fn print_log() -> Result<(), WaveBranchError> {
    let current_dir = env::current_dir().map_err(|e| WaveBranchError::IoError(e))?;
    let repo_root = find_repo_root(&current_dir)?;
    let repo_path = repo_root.join(".wavebranch");

    // Start from HEAD
    let head_path = repo_path.join("HEAD");
    if !head_path.exists() {
        return Err(WaveBranchError::RepoNotFound);
    }
    
    let head_content = std::fs::read_to_string(&head_path).map_err(|e| WaveBranchError::IoError(e))?;
    let current_commit_hash = if head_content.starts_with("ref: ") {
        let ref_path = head_content.trim_start_matches("ref: ").trim();
        let target_ref_path = repo_path.join(ref_path);
        if !target_ref_path.exists() {
            println!("No commits yet on this branch.");
            return Ok(());
        }
        std::fs::read_to_string(&target_ref_path).map_err(|e| WaveBranchError::IoError(e))?.trim().to_string()
    } else {
        head_content.trim().to_string()
    };

    if current_commit_hash.is_empty() {
        println!("No commits yet.");
        return Ok(());
    }

    let mut current_hash = current_commit_hash;
    loop {
        let commit: Commit = read_object(&repo_path, &current_hash)?;
        
        let datetime = Utc.timestamp_opt(commit.timestamp, 0).single().unwrap();
        let formatted_time = datetime.format("%a %b %e %H:%M:%S %Y +0000").to_string();

        println!("commit {}", current_hash);
        println!("Author: {}", commit.author);
        println!("Date:   {}\n", formatted_time);
        println!("    {}\n", commit.message);

        match commit.parent_hash {
            Some(parent) => current_hash = parent,
            None => break,
        }
    }

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
