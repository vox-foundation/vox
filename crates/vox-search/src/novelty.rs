//! Per-hit novelty scoring via 4-gram character shingling.

use std::collections::HashSet;

pub fn fnv1a(s: &str) -> u64 {
    let mut hash: u64 = 14_695_981_039_346_656_037;
    for byte in s.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash
}

fn fnv1a_chars(chars: &[char]) -> u64 {
    let mut hash: u64 = 14_695_981_039_346_656_037;
    let mut buf = [0u8; 4];
    for &c in chars {
        for &byte in c.encode_utf8(&mut buf).as_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(1_099_511_628_211);
        }
    }
    hash
}

/// Computes zero-allocation rolling character hash shingles of length `n`.
pub fn shingle_hashes(content: &str, n: usize) -> Vec<u64> {
    if content.is_empty() || n == 0 {
        return Vec::new();
    }
    let chars: Vec<char> = content.chars().flat_map(|c| c.to_lowercase()).collect();
    if chars.len() < n {
        return vec![fnv1a_chars(&chars)];
    }

    const BASE: u64 = 1_099_511_628_211;
    let mut power: u64 = 1;
    for _ in 0..(n - 1) {
        power = power.wrapping_mul(BASE);
    }

    let mut current_hash: u64 = 0;
    for &c in &chars[..n] {
        current_hash = current_hash.wrapping_mul(BASE).wrapping_add(c as u64);
    }

    let mut hashes = Vec::with_capacity(chars.len() - n + 1);
    hashes.push(current_hash);

    for i in n..chars.len() {
        let out_char = chars[i - n] as u64;
        let in_char = chars[i] as u64;
        current_hash = current_hash.wrapping_sub(out_char.wrapping_mul(power));
        current_hash = current_hash.wrapping_mul(BASE).wrapping_add(in_char);
        hashes.push(current_hash);
    }

    hashes
}

/// Tracks seen content fingerprints across a research session.
#[derive(Debug, Default)]
pub struct NoveltyScorer {
    seen: HashSet<u64>,
}

impl NoveltyScorer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Score content novelty: fraction of 4-gram shingles NOT yet in the seen-set.
    /// Returns 0.0 (all seen) to 1.0 (all new).
    #[must_use]
    pub fn score(&self, content: &str) -> f64 {
        let hashes = shingle_hashes(content, 4);
        if hashes.is_empty() {
            return 0.0;
        }
        let new_count = hashes.iter().filter(|h| !self.seen.contains(h)).count();
        new_count as f64 / hashes.len() as f64
    }

    /// Commit content to the seen-set.
    pub fn accept(&mut self, content: &str) {
        for h in shingle_hashes(content, 4) {
            self.seen.insert(h);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_scorer_scores_any_content_as_fully_novel() {
        let scorer = NoveltyScorer::new();
        assert_eq!(
            scorer.score("the quick brown fox jumped over the lazy dog"),
            1.0
        );
    }

    #[test]
    fn exact_duplicate_after_accept_scores_zero() {
        let mut scorer = NoveltyScorer::new();
        let text = "the quick brown fox jumped over the lazy dog";
        scorer.accept(text);
        assert_eq!(scorer.score(text), 0.0);
    }

    #[test]
    fn partial_overlap_scores_between_zero_and_one() {
        let mut scorer = NoveltyScorer::new();
        scorer.accept("the quick brown fox");
        let score = scorer.score("the quick brown lazy dog");
        assert!(score > 0.0 && score < 1.0, "score={score}");
    }

    #[test]
    fn short_content_treated_as_single_shingle() {
        let scorer = NoveltyScorer::new();
        assert_eq!(scorer.score("hi"), 1.0);
        let mut s = scorer;
        s.accept("hi");
        assert_eq!(s.score("hi"), 0.0);
    }

    #[test]
    fn empty_content_scores_zero() {
        let scorer = NoveltyScorer::new();
        assert_eq!(scorer.score(""), 0.0);
    }

    #[test]
    fn large_content_rolling_hashes() {
        let content = "a".repeat(25_000);
        let hashes = shingle_hashes(&content, 4);
        assert_eq!(hashes.len(), 25_000 - 4 + 1);
        let mut scorer = NoveltyScorer::new();
        scorer.accept(&content);
        assert_eq!(scorer.score(&content), 0.0);
    }
}
