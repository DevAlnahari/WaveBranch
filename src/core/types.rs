use serde::{Deserialize, Serialize};



/// An entry pointing to a Blob or Sub-tree, mimicking Git's tree structure.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct TreeEntry {
    /// E.g. "100644" for regular files
    pub mode: String,
    /// The filename or directory name
    pub name: String,
    /// SHA-256 hash of the object (Blob or Tree)
    pub hash: String,
}

/// A directory snapshot containing a list of entries.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Tree {
    pub entries: Vec<TreeEntry>,
}

/// A snapshot of the repository at a given point in time.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct Commit {
    /// SHA-256 hash of the root tree object
    pub tree_hash: String,
    /// SHA-256 hash of the parent commit (None if initial commit)
    pub parent_hash: Option<String>,
    /// Commit author
    pub author: String,
    /// Unix timestamp in seconds
    pub timestamp: i64,
    /// The commit message
    pub message: String,
}
