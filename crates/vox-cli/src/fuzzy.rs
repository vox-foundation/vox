//! Fuzzy matching utilities for CLI.
//! Uses `nucleo-matcher` for fast scoring when the `fuzzy-search` feature is enabled.

#[cfg(feature = "fuzzy-search")]
pub struct FuzzyMatcher {
    matcher: nucleo_matcher::Matcher,
}

#[cfg(feature = "fuzzy-search")]
impl Default for FuzzyMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "fuzzy-search")]
impl FuzzyMatcher {
    /// Create a new fuzzy matcher (allocates ~135KB scratch memory; reuse across calls).
    pub fn new() -> Self {
        Self {
            matcher: nucleo_matcher::Matcher::new(nucleo_matcher::Config::DEFAULT),
        }
    }

    /// Ranks candidates by fuzzy match score for the given pattern.
    /// Returns a list of (index, score) sorted by score descending.
    pub fn rank<T: AsRef<str>>(&mut self, pattern: &str, candidates: &[T]) -> Vec<(usize, u32)> {
        use nucleo_matcher::pattern::{CaseMatching, Normalization, Pattern};

        let pat = Pattern::parse(pattern, CaseMatching::Smart, Normalization::Smart);
        let mut results = Vec::with_capacity(candidates.len());

        for (idx, candidate) in candidates.iter().enumerate() {
            let mut haystack_buf = Vec::new();
            let haystack = nucleo_matcher::Utf32Str::new(candidate.as_ref(), &mut haystack_buf);
            if let Some(score) = pat.score(haystack, &mut self.matcher) {
                if score > 0 {
                    results.push((idx, score));
                }
            }
        }

        results.sort_by(|a, b| b.1.cmp(&a.1));
        results
    }
}

/// Fallback implementation when fuzzy-search feature is disabled.
#[cfg(not(feature = "fuzzy-search"))]
pub struct FuzzyMatcher;

#[cfg(not(feature = "fuzzy-search"))]
impl FuzzyMatcher {
    /// No-op constructor.
    pub fn new() -> Self { Self }
    /// Returns original indices with zero score (no re-ranking).
    pub fn rank<T: AsRef<str>>(&mut self, _pattern: &str, candidates: &[T]) -> Vec<(usize, u32)> {
        candidates.iter().enumerate().map(|(i, _)| (i, 0)).collect()
    }
}
