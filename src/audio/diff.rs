/// Audio Diffing Engine — Phase Cancellation via Mathematical Subtraction.
///
/// This is the core DSP innovation of WaveBranch: instead of diffing text
/// lines, we diff raw audio signals sample-by-sample. The "delta" signal
/// is the mathematical difference between two audio buffers. Applying
/// the delta to the original perfectly reconstructs the modified version.
///
/// # Theory
/// ```text
/// Delta = Modified - Original    (compute_audio_diff)
/// Modified = Original + Delta    (apply_audio_diff)
/// ```
///
/// This mirrors audio phase cancellation: subtracting identical signals
/// yields silence (zero delta), while differences surface as an audible
/// residual.

/// Computes the sample-wise difference between two audio buffers.
///
/// `delta[i] = modified[i] - original[i]` using wrapping arithmetic
/// to avoid panic on `i16` overflow (e.g., `i16::MAX - i16::MIN`).
///
/// If the signals differ in length, the shorter one is implicitly
/// zero-padded — equivalent to appending silence.
///
/// # Arguments
/// * `original` — The base audio signal (e.g., previous version).
/// * `modified` — The changed audio signal (e.g., current version).
///
/// # Returns
/// A `Vec<i16>` delta signal whose length equals `max(original.len(), modified.len())`.
pub fn compute_audio_diff(original: &[i16], modified: &[i16]) -> Vec<i16> {
    let max_len = original.len().max(modified.len());

    // Chain each signal with infinite zeros to handle length mismatches,
    // then take exactly max_len samples. Zero-cost for equal-length inputs
    // because the chain iterator never advances past the slice.
    original
        .iter()
        .copied()
        .chain(std::iter::repeat(0i16))
        .zip(
            modified
                .iter()
                .copied()
                .chain(std::iter::repeat(0i16)),
        )
        .take(max_len)
        .map(|(orig, modif)| modif.wrapping_sub(orig))
        .collect()
}

/// Applies a delta signal to an original buffer to reconstruct the
/// modified version.
///
/// `result[i] = original[i] + delta[i]` using wrapping arithmetic,
/// which is the exact inverse of `compute_audio_diff`.
///
/// # Invariant
/// `apply_audio_diff(A, compute_audio_diff(A, B)) == B` for all A, B.
#[allow(dead_code)]
pub fn apply_audio_diff(original: &[i16], delta: &[i16]) -> Vec<i16> {
    let max_len = original.len().max(delta.len());

    original
        .iter()
        .copied()
        .chain(std::iter::repeat(0i16))
        .zip(
            delta
                .iter()
                .copied()
                .chain(std::iter::repeat(0i16)),
        )
        .take(max_len)
        .map(|(orig, d)| orig.wrapping_add(d))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The fundamental correctness guarantee: applying the diff to
    /// the original must exactly reproduce the modified signal.
    #[test]
    fn diff_then_apply_reconstructs_original() {
        let original = vec![0_i16, 1000, -1000, 32767, -32768, 100];
        let modified = vec![500_i16, -500, 0, 0, 32767, -100];

        let delta = compute_audio_diff(&original, &modified);
        let reconstructed = apply_audio_diff(&original, &delta);

        assert_eq!(reconstructed, modified, "Diff+Apply must perfectly reconstruct");
    }

    /// When signals differ in length, the shorter one is zero-padded.
    /// The delta length must equal the longer signal.
    #[test]
    fn diff_handles_length_mismatch() {
        let short = vec![100_i16, 200];
        let long = vec![100_i16, 200, 300, 400, 500];

        // short → long
        let delta = compute_audio_diff(&short, &long);
        assert_eq!(delta.len(), long.len(), "Delta length must match the longer signal");
        let reconstructed = apply_audio_diff(&short, &delta);
        assert_eq!(reconstructed, long);

        // long → short (the "missing" samples in modified are treated as silence)
        let delta_rev = compute_audio_diff(&long, &short);
        assert_eq!(delta_rev.len(), long.len());
        let reconstructed_rev = apply_audio_diff(&long, &delta_rev);
        // Reconstructed should equal short + trailing zeros
        let mut expected = short.clone();
        expected.resize(long.len(), 0);
        assert_eq!(reconstructed_rev, expected);
    }

    /// Diffing identical signals must yield all zeros (silence).
    /// This is the mathematical equivalent of phase cancellation.
    #[test]
    fn identical_signals_produce_zero_delta() {
        let signal = vec![1000_i16, -2000, 32767, -32768, 0];
        let delta = compute_audio_diff(&signal, &signal);
        assert!(delta.iter().all(|&d| d == 0), "Phase cancellation: identical signals → silence");
    }
}
