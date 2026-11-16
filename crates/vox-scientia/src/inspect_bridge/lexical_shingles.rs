//! Lexical shingle Jaccard and 64-permutation MinHash similarity using `vox_crypto::fast_hash`.

use std::collections::BTreeSet;

/// Number of permutations in each MinHash sketch.
pub const MINHASH_PERMUTATIONS: usize = 64;

/// Default k-shingle width (3-word shingles).
pub const DEFAULT_SHINGLE_K: usize = 3;

/// Normalize input text into lowercase alphanumeric tokens.
#[must_use]
pub fn tokenize_normalized(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            for lc in ch.to_lowercase() {
                current.push(lc);
            }
        } else if !current.is_empty() {
            tokens.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    tokens
}

/// Extract word k-shingles from normalized tokens.
///
/// When `tokens.len() < k` and `tokens` is non-empty, returns unigrams plus the
/// joined short phrase so short titles still produce comparable shingles.
#[must_use]
pub fn word_shingles(tokens: &[String], k: usize) -> BTreeSet<String> {
    let k = k.max(1);
    let mut out = BTreeSet::new();
    if tokens.is_empty() {
        return out;
    }
    if tokens.len() < k {
        out.insert(tokens.join(" "));
        for tok in tokens {
            out.insert(tok.clone());
        }
        return out;
    }
    for window in tokens.windows(k) {
        out.insert(window.join(" "));
    }
    out
}

/// Compute exact Jaccard similarity over 3-gram word shingles (`0.0` when either side is empty).
#[must_use]
pub fn shingle_jaccard(a: &str, b: &str) -> f64 {
    let ta = tokenize_normalized(a);
    let tb = tokenize_normalized(b);
    let sa = word_shingles(&ta, DEFAULT_SHINGLE_K);
    let sb = word_shingles(&tb, DEFAULT_SHINGLE_K);
    if sa.is_empty() || sb.is_empty() {
        return 0.0;
    }
    let inter = sa.intersection(&sb).count();
    let union = sa.union(&sb).count();
    if union == 0 {
        0.0
    } else {
        inter as f64 / union as f64
    }
}

/// Compute a deterministic 64-permutation MinHash signature using `vox_crypto::fast_hash`.
#[must_use]
pub fn minhash_signature(text: &str) -> Option<[u64; MINHASH_PERMUTATIONS]> {
    let tokens = tokenize_normalized(text);
    let shingles = word_shingles(&tokens, DEFAULT_SHINGLE_K);
    if shingles.is_empty() {
        return None;
    }
    let mut sig = [u64::MAX; MINHASH_PERMUTATIONS];
    for shingle in shingles {
        let base_hash = vox_crypto::fast_hash(shingle.as_bytes());
        let mut buf = [0u8; 16];
        buf[8..].copy_from_slice(&base_hash.to_le_bytes());
        for (perm, slot) in sig.iter_mut().enumerate() {
            // Distinct non-zero odd-spaced permutation seed per MinHash lane.
            let perm_seed = (perm as u64)
                .wrapping_add(1)
                .wrapping_mul(0x9E37_79B9_7F4A_7C15);
            buf[..8].copy_from_slice(&perm_seed.to_le_bytes());
            let permuted = vox_crypto::fast_hash(&buf);
            if permuted < *slot {
                *slot = permuted;
            }
        }
    }
    Some(sig)
}

/// Estimate Jaccard similarity via 64-permutation MinHash sketches.
#[must_use]
pub fn minhash_similarity(a: &str, b: &str) -> f64 {
    let (Some(sa), Some(sb)) = (minhash_signature(a), minhash_signature(b)) else {
        return 0.0;
    };
    let matches = sa.iter().zip(sb.iter()).filter(|(x, y)| x == y).count();
    matches as f64 / MINHASH_PERMUTATIONS as f64
}

/// Combined lexical text overlap from 3-gram shingle Jaccard and 64-perm MinHash.
#[must_use]
pub fn text_overlap_score(a: &str, b: &str) -> f64 {
    let jaccard = shingle_jaccard(a, b);
    let minhash = minhash_similarity(a, b);
    if jaccard == 0.0 && minhash == 0.0 {
        return 0.0;
    }
    (0.70 * jaccard + 0.30 * minhash).clamp(0.0, 1.0)
}

/// Calibrate lexical overlap from an explicit provider/bundle score and computed text similarity.
#[must_use]
pub fn calibrated_lexical_overlap(raw_score: Option<f64>, text_sim: Option<f64>) -> f64 {
    let clean_raw = raw_score
        .filter(|v| v.is_finite())
        .map(|v| v.clamp(0.0, 1.0));
    let clean_text = text_sim
        .filter(|v| v.is_finite())
        .map(|v| v.clamp(0.0, 1.0));
    match (clean_raw, clean_text) {
        (Some(raw), Some(text)) => raw.max(text),
        (Some(raw), None) => raw,
        (None, Some(text)) => text,
        (None, None) => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_normalizes_case_and_punctuation() {
        assert_eq!(
            tokenize_normalized("Graph-Guided Test Repair in Rust!"),
            vec!["graph", "guided", "test", "repair", "in", "rust"]
        );
    }

    #[test]
    fn identical_and_disjoint_shingles_match_bounds() {
        let a = "Calibrated novelty detection via lexical shingle minhash";
        let b = "Completely orthogonal cryptographic hardware verification";
        assert!((shingle_jaccard(a, a) - 1.0).abs() < 1e-9);
        assert_eq!(shingle_jaccard(a, b), 0.0);
        assert_eq!(shingle_jaccard("", a), 0.0);
    }

    #[test]
    fn minhash_64_perm_is_deterministic_and_sensitive() {
        let a = "Fused lexical and semantic novelty scoring for research bundles";
        let near = "Fused lexical and semantic novelty scoring for academic bundles";
        let far = "Zero knowledge succinct arguments over pairing friendly curves";
        assert_eq!(minhash_signature(a), minhash_signature(a));
        assert!((minhash_similarity(a, a) - 1.0).abs() < 1e-9);
        assert!(minhash_similarity(a, near) > minhash_similarity(a, far));
        assert!(text_overlap_score(a, near) > 0.45);
        assert_eq!(text_overlap_score(a, far), 0.0);
    }

    #[test]
    fn calibrated_lexical_overlap_handles_partial_and_non_finite_inputs() {
        assert!((calibrated_lexical_overlap(Some(0.95), Some(0.4)) - 0.95).abs() < 1e-9);
        assert!((calibrated_lexical_overlap(None, Some(0.72)) - 0.72).abs() < 1e-9);
        assert_eq!(calibrated_lexical_overlap(Some(f64::NAN), None), 0.0);
        assert!((calibrated_lexical_overlap(Some(1.4), Some(-0.2)) - 1.0).abs() < 1e-9);
    }
}
