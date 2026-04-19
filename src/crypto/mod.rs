/// Cryptography module — content-addressable hashing for audio blobs.
///
/// This is the identity function of WaveBranch's object store: given
/// raw PCM bytes, produce a deterministic, collision-resistant digest
/// that serves as the blob's key in `objects/`.
pub mod hasher;
