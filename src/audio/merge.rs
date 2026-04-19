use crate::audio::diff::compute_audio_diff;

/// Three-way DSP logic for audio arrays.
///
/// Merges two divergent series (branch_a and branch_b) on top of the original base representation
/// utilizing mathematical `wrapping_add` to apply the deltas accurately without clipping exceptions.
pub fn three_way_audio_merge(base: &[i16], branch_a: &[i16], branch_b: &[i16]) -> Vec<i16> {
    // 1. Calculate numerical offsets relative to the common base.
    let diff_a = compute_audio_diff(base, branch_a);
    let diff_b = compute_audio_diff(base, branch_b);

    // Ensure we iterate until the end of the longest series
    let max_len = std::cmp::max(base.len(), std::cmp::max(diff_a.len(), diff_b.len()));

    let mut merged = Vec::with_capacity(max_len);

    for i in 0..max_len {
        let sample_base = base.get(i).copied().unwrap_or(0);
        let sample_da = diff_a.get(i).copied().unwrap_or(0);
        let sample_db = diff_b.get(i).copied().unwrap_or(0);

        // 2`base + diff_a + diff_b`. Use wrapping_add for standard bitwise integer overflow.
        let merged_sample = sample_base.wrapping_add(sample_da).wrapping_add(sample_db);
        merged.push(merged_sample);
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_three_way_audio_merge() {
        let base = vec![0, 100, -100, 50, 0];
        // branch_a increased amplitude by 50
        let a    = vec![0, 150, -150, 50, 0];
        // branch_b increased amplitude by 10 and changed the last sample
        let b    = vec![0, 110, -110, 50, 10];

        // diff_a: [0, 50, -50, 0, 0]
        // diff_b: [0, 10, -10, 0, 10]
        // result = base + diff_a + diff_b = [0, 160, -160, 50, 10]
        let expected = vec![0, 160, -160, 50, 10];
        
        let merged = three_way_audio_merge(&base, &a, &b);
        assert_eq!(merged, expected);
    }
}
