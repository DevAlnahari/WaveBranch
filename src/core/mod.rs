/// Core VCS module — houses repository initialization and ref management.
///
/// This module owns the `.wavebranch/` directory layout and all operations
/// that mutate the repository's structural state (as opposed to content
/// like audio blobs, which live in the `audio` and `crypto` modules).
pub mod init;
pub mod object;
pub mod types;
pub mod commit;
pub mod refs;
pub mod merge;
pub mod index;
pub mod add;
pub mod log;
pub mod reset;
