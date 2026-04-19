/// SHA-256 hashing of raw PCM audio samples.
///
/// Provides two modes:
/// - `hash_pcm_samples`: O(1) cast of an in-memory `&[i16]` buffer.
/// - `hash_pcm_streaming`: Incremental hashing from a chunk iterator,
///   for files too large to fit in memory.
use sha2::{Sha256, Digest};

use crate::audio::reader::PcmChunkIter;
use crate::error::WaveBranchError;

/// Computes the SHA-256 hash of raw 16-bit PCM samples in memory.
///
/// Uses `bytemuck::cast_slice` for zero-cost `&[i16]` → `&[u8]`
/// reinterpretation — a pointer cast with no copies.
///
/// # Arguments
/// * `samples` — A contiguous slice of 16-bit PCM audio samples.
///
/// # Returns
/// A 64-character lowercase hexadecimal string representing the SHA-256 digest.
pub fn hash_pcm_samples(samples: &[i16]) -> String {
    let bytes: &[u8] = bytemuck::cast_slice(samples);
    let digest = Sha256::digest(bytes);
    hex::encode(digest)
}

/// Computes the SHA-256 hash of PCM samples incrementally from a
/// chunk iterator.
///
/// Instead of loading the entire file into memory, this function
/// feeds each chunk into the hasher as it arrives. Memory usage
/// is bounded by the chunk size (~1MB) regardless of file size.
///
/// # Errors
/// - Propagates any `WaveBranchError` from the chunk iterator
///   (e.g., WAV decode failures mid-stream).
pub fn hash_pcm_streaming(chunks: PcmChunkIter) -> Result<String, WaveBranchError> {
    let mut hasher = Sha256::new();

    for chunk_result in chunks {
        let chunk = chunk_result?;
        let bytes: &[u8] = bytemuck::cast_slice(&chunk);
        hasher.update(bytes);
    }

    Ok(hex::encode(hasher.finalize()))
}
