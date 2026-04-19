/// Raw PCM sample extraction from WAV files.
///
/// Uses `hound` to parse RIFF/WAV headers and iterate over the audio
/// data section. The key invariant is: **only the PCM sample bytes
/// leave this module** — no headers, no metadata, no ID3 tags.
/// This guarantees that two WAV files with identical audio content
/// but different metadata will produce the same downstream hash.
use std::path::Path;

use hound::WavReader;

use crate::error::WaveBranchError;

/// Default chunk size for streaming: 512K samples = 1MB at 16-bit.
const DEFAULT_CHUNK_SAMPLES: usize = 512 * 1024;

/// Extracts raw 16-bit PCM samples from a WAV file into memory.
///
/// Opens the file at `path`, reads all samples as `i16`, and returns
/// them as a contiguous `Vec<i16>`. Headers and metadata are discarded
/// by `hound` during parsing — only the data chunk is surfaced.
///
/// Use this for operations that need the full buffer (diff, blob write).
/// For OOM-safe hashing of large files, use `stream_pcm_chunks` instead.
///
/// # Errors
/// - `WaveBranchError::InvalidWavFormat` if `hound` cannot parse the file.
pub fn extract_pcm_samples(path: &Path) -> Result<Vec<i16>, WaveBranchError> {
    let reader = WavReader::open(path).map_err(|e| {
        WaveBranchError::InvalidWavFormat(format!("{}: {}", path.display(), e))
    })?;

    let samples: Result<Vec<i16>, _> = reader.into_samples::<i16>().collect();

    samples.map_err(|e| {
        WaveBranchError::InvalidWavFormat(format!(
            "Failed to decode PCM samples from {}: {}",
            path.display(),
            e
        ))
    })
}

/// An iterator that yields PCM samples in fixed-size chunks.
///
/// Each call to `next()` returns a `Vec<i16>` of up to `chunk_size`
/// samples. The last chunk may be shorter than `chunk_size`.
/// This enables streaming hash computation without loading the
/// entire file into memory — critical for multi-GB audio files.
pub struct PcmChunkIter {
    /// The underlying hound sample iterator, boxed to hide the
    /// concrete file-backed reader type.
    samples: Box<dyn Iterator<Item = Result<i16, hound::Error>>>,
    /// Maximum number of i16 samples per chunk.
    chunk_size: usize,
}

impl Iterator for PcmChunkIter {
    type Item = Result<Vec<i16>, WaveBranchError>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut chunk = Vec::with_capacity(self.chunk_size);

        for _ in 0..self.chunk_size {
            match self.samples.next() {
                Some(Ok(sample)) => chunk.push(sample),
                Some(Err(e)) => {
                    return Some(Err(WaveBranchError::InvalidWavFormat(
                        format!("Failed to decode PCM sample: {}", e),
                    )));
                }
                None => break,
            }
        }

        if chunk.is_empty() {
            None
        } else {
            Some(Ok(chunk))
        }
    }
}

/// Opens a WAV file and returns an iterator yielding PCM sample chunks.
///
/// Each chunk contains up to `DEFAULT_CHUNK_SAMPLES` (512K) samples,
/// equivalent to ~1MB of 16-bit audio. This allows the caller to
/// process arbitrarily large files without loading them fully into memory.
///
/// # Errors
/// - `WaveBranchError::InvalidWavFormat` if the file cannot be opened as WAV.
pub fn stream_pcm_chunks(path: &Path) -> Result<PcmChunkIter, WaveBranchError> {
    let reader = WavReader::open(path).map_err(|e| {
        WaveBranchError::InvalidWavFormat(format!("{}: {}", path.display(), e))
    })?;

    Ok(PcmChunkIter {
        samples: Box::new(reader.into_samples::<i16>()),
        chunk_size: DEFAULT_CHUNK_SAMPLES,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::hasher::hash_pcm_samples;
    use std::io::Cursor;

    /// Verifies that hashing the same PCM buffer twice yields an
    /// identical digest — the foundational determinism guarantee
    /// that the entire content-addressable store depends on.
    #[test]
    fn deterministic_hash_from_in_memory_wav() {
        // Build a minimal 16-bit mono WAV in memory.
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };

        let mut buffer = Cursor::new(Vec::new());
        {
            let mut writer = hound::WavWriter::new(&mut buffer, spec).expect("test: writer init");
            // Write a short deterministic waveform.
            for sample in [0_i16, 1000, -1000, 32767, -32768, 0] {
                writer.write_sample(sample).expect("test: write sample");
            }
            writer.finalize().expect("test: finalize");
        }

        // Reset cursor and read back the PCM data.
        buffer.set_position(0);
        let reader = WavReader::new(&mut buffer).expect("test: reader init");
        let samples: Vec<i16> = reader
            .into_samples::<i16>()
            .map(|s| s.expect("test: sample decode"))
            .collect();

        // Hash twice and assert determinism.
        let hash_a = hash_pcm_samples(&samples);
        let hash_b = hash_pcm_samples(&samples);

        assert_eq!(hash_a, hash_b, "Hashing must be deterministic");
        assert_eq!(hash_a.len(), 64, "SHA-256 hex digest must be 64 chars");
    }
}
