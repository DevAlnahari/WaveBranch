use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::error::WaveBranchError;

/// An entry in the Index representing a staged file
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct IndexEntry {
    /// Relative path format (standardized to use Unix-style `/` cross-platform)
    pub path: String,
    /// The SHA-1 hash sum of the Blob representing this file's data
    pub hash: String,
}

/// The Index model which acts as the staging area.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Index {
    pub entries: Vec<IndexEntry>,
}

impl Index {
    /// Inserts or updates an entry within the index. Paths must be canonicalized and matching exactly.
    pub fn update_entry(&mut self, path: String, hash: String) {
        if let Some(existing) = self.entries.iter_mut().find(|e| e.path == path) {
            existing.hash = hash;
        } else {
            self.entries.push(IndexEntry { path, hash });
        }
    }
}

/// Helper function to locate the physical index file.
fn get_index_path(repo_path: &Path) -> std::path::PathBuf {
    repo_path.join("index")
}

/// Reads the Index from `.wavebranch/index`. If the file doesn't exist, it resolves to an empty Index.
pub fn read_index(repo_path: &Path) -> Result<Index, WaveBranchError> {
    let index_file = get_index_path(repo_path);
    if !index_file.exists() {
        return Ok(Index::default());
    }

    let contents = fs::read_to_string(&index_file)
        .map_err(|e| WaveBranchError::IndexError(format!("Failed to read index file: {}", e)))?;
        
    let index: Index = serde_json::from_str(&contents)
        .map_err(|e| WaveBranchError::IndexError(format!("Failed to parse index JSON: {}", e)))?;
        
    Ok(index)
}

/// Writes the specific Index sequentially back to `.wavebranch/index`.
pub fn write_index(repo_path: &Path, index: &Index) -> Result<(), WaveBranchError> {
    let index_file = get_index_path(repo_path);
    
    let json_data = serde_json::to_string_pretty(index)
        .map_err(|e| WaveBranchError::IndexError(format!("Failed to serialize index: {}", e)))?;
        
    fs::write(&index_file, json_data)
        .map_err(|e| WaveBranchError::IndexError(format!("Failed to write index file: {}", e)))?;
        
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_index_read_write() {
        let dir = tempdir().unwrap();
        let repo_path = dir.path();
        
        // Ensure .wavebranch directory exists
        fs::create_dir_all(repo_path.join(".wavebranch")).unwrap();

        let mut index = read_index(repo_path).unwrap();
        assert!(index.entries.is_empty(), "Expected empty index");

        index.update_entry("audio/vocals.wav".to_string(), "abc123hash".to_string());
        write_index(repo_path, &index).unwrap();

        let read_back = read_index(repo_path).unwrap();
        assert_eq!(read_back.entries.len(), 1);
        assert_eq!(read_back.entries[0].path, "audio/vocals.wav");
        assert_eq!(read_back.entries[0].hash, "abc123hash");
    }
}
