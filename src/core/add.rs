use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::WaveBranchError;
use crate::audio::reader::extract_pcm_samples;
use crate::core::object::write_blob;
use crate::core::index::{read_index, write_index};

/// Adds WAV files from the specific path to the staging index.
/// If `target_path` is a single file, it's processed individually.
/// If it's a directory, it walks recursively yielding all `.wav` targets automatically.
pub fn add_path(target_path: &str) -> Result<(), WaveBranchError> {
    let cwd = env::current_dir()
        .map_err(|e| WaveBranchError::PathError(format!("Failed to get current directory: {}", e)))?;
    let repo_root = find_repo_root(&cwd)?;
    let repo_path = repo_root.join(".wavebranch");
        
    let base_add_path = cwd.join(target_path);
    if !base_add_path.exists() {
        return Err(WaveBranchError::PathError(format!("Path does not exist: {}", target_path)));
    }

    let mut index = read_index(&repo_path)?;
    let mut added_count = 0;

    let mut paths_to_process = Vec::new();
    if base_add_path.is_dir() {
        collect_wav_files(&base_add_path, &mut paths_to_process)?;
    } else if base_add_path.is_file() {
        paths_to_process.push(base_add_path.clone());
    }

    for file_path in paths_to_process {
        if file_path.extension().and_then(|s| s.to_str()) != Some("wav") {
            continue;
        }

        // Canonicalize relative paths specifically mapping into UNIX formatting explicitly for hashing structures
        let rel_path = file_path.strip_prefix(&repo_root)
            .map_err(|_| WaveBranchError::PathError("File is outside repository root".to_string()))?;
            
        let mut rel_path_str = rel_path.to_string_lossy().into_owned();
        if cfg!(windows) {
            rel_path_str = rel_path_str.replace("\\", "/");
        }

        // Process audio
        let samples = extract_pcm_samples(&file_path)?;
        
        // Write Blob and generate address
        let blob_hash = write_blob(&repo_path, &samples)?;
        
        // Update staging index
        index.update_entry(rel_path_str, blob_hash);
        added_count += 1;
    }

    write_index(&repo_path, &index)?;
    
    println!("Staged {} audio file(s) into the index.", added_count);
    
    Ok(())
}

fn collect_wav_files(dir: &Path, list: &mut Vec<PathBuf>) -> Result<(), WaveBranchError> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir).map_err(|e| WaveBranchError::IoError(e))? {
            let entry = entry.map_err(|e| WaveBranchError::IoError(e))?;
            let path = entry.path();
            
            // Ignore the `.wavebranch` internally mapping loops entirely
            if path.file_name().and_then(|n| n.to_str()) == Some(".wavebranch") {
                continue;
            }

            if path.is_dir() {
                collect_wav_files(&path, list)?;
            } else if path.is_file() {
                list.push(path);
            }
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
        if let Some(parent) = dir.parent() {
            dir = parent;
        } else {
            return Err(WaveBranchError::RepoNotFound);
        }
    }
}
