/// Content-Addressable Storage (CAS) for compressed audio blobs.
///
/// Mirrors Git's loose object format: objects are stored at
/// `objects/{hash[0..2]}/{hash[2..]}` after Zlib compression.
/// This gives O(1) lookup by hash and avoids filesystem performance
/// degradation from millions of files in a single directory.
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use serde::{de::DeserializeOwned, Serialize};
use sha2::{Digest, Sha256};

use crate::crypto::hasher::hash_pcm_samples;
use crate::error::WaveBranchError;

/// An audio blob: the atomic unit of storage in WaveBranch.
///
/// Each blob is uniquely identified by the SHA-256 hash of its raw
/// PCM sample data. The samples are stored compressed but always
/// surfaced to consumers in their original `i16` form.
#[allow(dead_code)]
pub struct Blob {
    /// SHA-256 hex digest of the raw PCM data.
    pub hash: String,
    /// Decompressed 16-bit PCM audio samples.
    pub samples: Vec<i16>,
}

/// Compresses and writes raw PCM samples to the object store.
///
/// # Object Path Layout
/// Given hash `a3f1c8...`, the object is stored at:
/// `{repo_path}/objects/a3/f1c8...`
///
/// # Pipeline
/// 1. Hash raw `&[i16]` via `hash_pcm_samples`
/// 2. Reinterpret `&[i16]` as `&[u8]` via `bytemuck::cast_slice`
/// 3. Compress with Zlib (default compression level)
/// 4. Write to `objects/{prefix}/{suffix}`
///
/// # Returns
/// The SHA-256 hex string that serves as the blob's key.
///
/// # Errors
/// - `WaveBranchError::IoError` on filesystem failures.
/// - `WaveBranchError::CompressionError` if Zlib encoding fails.
pub fn write_blob(repo_path: &Path, samples: &[i16]) -> Result<String, WaveBranchError> {
    let hash = hash_pcm_samples(samples);
    let bytes: &[u8] = bytemuck::cast_slice(samples);

    // Compress the raw PCM bytes with Zlib.
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes).map_err(|e| {
        WaveBranchError::CompressionError(format!("Zlib encode failed: {}", e))
    })?;
    let compressed = encoder.finish().map_err(|e| {
        WaveBranchError::CompressionError(format!("Zlib finalize failed: {}", e))
    })?;

    // Split hash into 2-char prefix and remainder (Git convention).
    let (prefix, suffix) = hash.split_at(2);
    let object_dir = repo_path.join("objects").join(prefix);
    fs::create_dir_all(&object_dir)?;
    fs::write(object_dir.join(suffix), &compressed)?;

    Ok(hash)
}

/// Reads and decompresses a blob from the object store by its hash.
///
/// # Errors
/// - `WaveBranchError::ObjectNotFound` if no object file exists for the hash.
/// - `WaveBranchError::CompressionError` if Zlib decompression fails.
/// - `WaveBranchError::IoError` on filesystem failures.
#[allow(dead_code)]
pub fn read_blob(repo_path: &Path, hash: &str) -> Result<Blob, WaveBranchError> {
    let (prefix, suffix) = hash.split_at(2);
    let object_path = repo_path.join("objects").join(prefix).join(suffix);

    if !object_path.exists() {
        return Err(WaveBranchError::ObjectNotFound(hash.to_string()));
    }

    let compressed = fs::read(&object_path)?;

    // Decompress Zlib data back to raw bytes.
    let mut decoder = ZlibDecoder::new(&compressed[..]);
    let mut decompressed = Vec::new();
    decoder.read_to_end(&mut decompressed).map_err(|e| {
        WaveBranchError::CompressionError(format!("Zlib decode failed: {}", e))
    })?;

    // Reinterpret raw bytes back to i16 samples.
    // Ensure the byte count is even (2 bytes per i16 sample).
    if decompressed.len() % 2 != 0 {
        return Err(WaveBranchError::CompressionError(
            "Decompressed data has odd byte count, cannot cast to i16".to_string(),
        ));
    }

    let samples: Vec<i16> = bytemuck::cast_slice::<u8, i16>(&decompressed).to_vec();

    Ok(Blob { hash: hash.to_string(), samples })
}

/// Generic function to write a serializable object to the object store.
/// 
/// It serializes the object to JSON, hashes the JSON bytes (SHA-256),
/// compresses them with Zlib, and writes to `objects/{prefix}/{suffix}`.
pub fn write_object<T: Serialize>(repo_path: &Path, obj: &T) -> Result<String, WaveBranchError> {
    let json_str = serde_json::to_string(obj)?;
    let bytes = json_str.as_bytes();

    let digest = Sha256::digest(bytes);
    let hash = hex::encode(digest);

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes).map_err(|e| {
        WaveBranchError::CompressionError(format!("Zlib encode failed: {}", e))
    })?;
    let compressed = encoder.finish().map_err(|e| {
        WaveBranchError::CompressionError(format!("Zlib finalize failed: {}", e))
    })?;

    let (prefix, suffix) = hash.split_at(2);
    let object_dir = repo_path.join("objects").join(prefix);
    fs::create_dir_all(&object_dir)?;
    fs::write(object_dir.join(suffix), &compressed)?;

    Ok(hash)
}

/// Generic function to read and deserialize an object from the object store.
pub fn read_object<T: DeserializeOwned>(repo_path: &Path, hash: &str) -> Result<T, WaveBranchError> {
    let (prefix, suffix) = hash.split_at(2);
    let object_path = repo_path.join("objects").join(prefix).join(suffix);

    if !object_path.exists() {
        return Err(WaveBranchError::ObjectNotFound(hash.to_string()));
    }

    let compressed = fs::read(&object_path)?;

    let mut decoder = ZlibDecoder::new(&compressed[..]);
    let mut decompressed = String::new();
    decoder.read_to_string(&mut decompressed).map_err(|e| {
        WaveBranchError::CompressionError(format!("Zlib decode failed: {}", e))
    })?;

    let obj: T = serde_json::from_str(&decompressed)?;
    Ok(obj)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Verifies the full write → read roundtrip: compress, store,
    /// retrieve, decompress, and compare against the original samples.
    #[test]
    fn blob_write_read_roundtrip() {
        let tmp = TempDir::new().expect("test: create temp dir");
        let repo_path = tmp.path().join(".wavebranch");
        fs::create_dir_all(repo_path.join("objects")).expect("test: create objects dir");

        let original_samples: Vec<i16> = vec![0, 1000, -1000, 32767, -32768, 42, -42];

        // Write blob.
        let hash = write_blob(&repo_path, &original_samples).expect("test: write_blob");
        assert_eq!(hash.len(), 64, "SHA-256 hex digest must be 64 chars");

        // Read blob back.
        let blob = read_blob(&repo_path, &hash).expect("test: read_blob");
        assert_eq!(blob.hash, hash);
        assert_eq!(blob.samples, original_samples, "Roundtrip must be lossless");
    }

    use crate::core::types::{Tree, TreeEntry};

    /// Verifies the full write → read roundtrip for generic serialized objects.
    #[test]
    fn object_write_read_roundtrip() {
        let tmp = TempDir::new().expect("test: create temp dir");
        let repo_path = tmp.path().join(".wavebranch");
        fs::create_dir_all(repo_path.join("objects")).expect("test: create objects dir");

        let original_tree = Tree {
            entries: vec![
                TreeEntry {
                    mode: "100644".to_string(),
                    name: "track_v1.wav".to_string(),
                    hash: "a3f1c8...".to_string(),
                }
            ]
        };

        let hash = write_object(&repo_path, &original_tree).expect("test: write_object");
        assert_eq!(hash.len(), 64);

        let retrieved_tree: Tree = read_object(&repo_path, &hash).expect("test: read_object");
        assert_eq!(retrieved_tree, original_tree);
    }
}
