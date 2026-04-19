use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

use chrono::Utc;

use crate::core::index::read_index;
use crate::core::object::write_object;
use crate::core::types::{Commit, Tree, TreeEntry};
use crate::error::WaveBranchError;

#[derive(Default)]
struct DirNode {
    files: Vec<(String, String)>, // (name, blob_hash)
    dirs: HashMap<String, DirNode>,
}

fn build_tree(node: &DirNode, repo_path: &Path) -> Result<String, WaveBranchError> {
    let mut tree = Tree { entries: Vec::new() };

    for (name, child_node) in &node.dirs {
        let child_hash = build_tree(child_node, repo_path)?;
        tree.entries.push(TreeEntry {
            mode: "040000".to_string(),
            name: name.clone(),
            hash: child_hash,
        });
    }

    for (name, hash) in &node.files {
        tree.entries.push(TreeEntry {
            mode: "100644".to_string(), // Regular file mode
            name: name.clone(),
            hash: hash.clone(),
        });
    }

    // Sort entries alphabetically to ensure deterministic tree hashing
    tree.entries.sort_by(|a, b| a.name.cmp(&b.name));

    write_object(repo_path, &tree)
}

/// Create a new commit capturing the current state of tracked files in the Index.
pub fn create_commit(message: String, author: String) -> Result<String, WaveBranchError> {
    let cwd = env::current_dir()?;
    let repo_path = cwd.join(".wavebranch");

    if !repo_path.exists() {
        return Err(WaveBranchError::RepoNotFound);
    }

    let index = read_index(&repo_path)?;
    if index.entries.is_empty() {
        return Err(WaveBranchError::IndexError("Nothing to commit (index is empty).".to_string()));
    }

    let mut root_node = DirNode::default();
    
    // Flat map paths directly onto hierarchical memory mappings
    for entry in index.entries {
        let parts: Vec<&str> = entry.path.split('/').collect();
        let file_name = parts.last()
            .ok_or_else(|| WaveBranchError::IndexError(format!("Invalid path in index: {}", entry.path)))?
            .to_string();
            
        let dir_parts = &parts[..parts.len() - 1];
        
        let mut current_node = &mut root_node;
        for part in dir_parts {
            current_node = current_node.dirs.entry(part.to_string()).or_insert_with(DirNode::default);
        }
        current_node.files.push((file_name, entry.hash));
    }

    // Hash tree recursively from memory and retrieve base root hash
    let root_tree_hash = build_tree(&root_node, &repo_path)?;

    // Resolve HEAD to get parent_hash
    let head_path = repo_path.join("HEAD");
    let head_content = fs::read_to_string(&head_path)?;
    let head_ref = head_content.trim().trim_start_matches("ref: ");
    
    let main_ref_path = repo_path.join(head_ref);
    let parent_hash = if main_ref_path.exists() {
        let parent = fs::read_to_string(&main_ref_path)?;
        Some(parent.trim().to_string())
    } else {
        None
    };

    // Build Commit
    let commit = Commit {
        tree_hash: root_tree_hash,
        parent_hash,
        author,
        timestamp: Utc::now().timestamp(),
        message,
    };
    let commit_hash = write_object(&repo_path, &commit)?;

    // Update branch pointer
    fs::write(&main_ref_path, format!("{}\n", commit_hash))?;

    // According to Git spec we do NOT clear the index upon commit. 
    // The index structurally represents the 'next' staging mapping.

    Ok(commit_hash)
}
