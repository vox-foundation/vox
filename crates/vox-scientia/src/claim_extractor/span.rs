use crate::claim_extractor::types::SpanBound;

pub struct SpanChecker {
    pub min_overlap_fraction: f64,
}

impl Default for SpanChecker {
    fn default() -> Self {
        Self {
            min_overlap_fraction: 0.6,
        }
    }
}

impl SpanChecker {
    pub fn check(&self, claim_text: &str, span: &SpanBound, source: &str) -> bool {
        if span.start >= span.end {
            return false;
        }
        // Offsets are not guaranteed to belong to `source` (see ExtractionPipeline::extract):
        // out of range or off a char boundary (e.g. inside an emoji in web text) means the
        // span cannot be verified — never a panic.
        let Some(span_slice) = source.get(span.start..span.end) else {
            return false;
        };
        let claim_words: std::collections::HashSet<&str> = claim_text.split_whitespace().collect();
        let span_words: std::collections::HashSet<&str> = span_slice.split_whitespace().collect();
        if claim_words.is_empty() {
            return false;
        }
        let overlap = claim_words.intersection(&span_words).count();
        (overlap as f64 / claim_words.len() as f64) >= self.min_overlap_fraction
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_within_source_passes() {
        let checker = SpanChecker::default();
        let source = "p95 latency rose by 10ms in April.";
        assert!(checker.check(
            "p95 latency rose by 10ms",
            &SpanBound { start: 0, end: 24 },
            source
        ));
    }

    #[test]
    fn span_outside_source_fails() {
        let checker = SpanChecker::default();
        let source = "short text";
        assert!(!checker.check("other claim", &SpanBound { start: 0, end: 50 }, source));
    }

    /// Live Task 15 crash: a span's byte offsets landed inside '🥈' in Tavily content
    /// and `&source[start..end]` panicked, killing the whole chat turn.
    #[test]
    fn span_off_char_boundary_is_unverifiable_not_a_panic() {
        let checker = SpanChecker::default();
        let source = "SearXNG 🥈 second place; 検索エンジン Tavily first.";
        let emoji_start = source.find('🥈').unwrap();
        // End inside the 4-byte emoji.
        let mid_emoji = SpanBound {
            start: 0,
            end: emoji_start + 2,
        };
        assert!(!checker.check("SearXNG", &mid_emoji, source));
        // Start inside a 3-byte CJK char.
        let cjk = source.find('検').unwrap();
        let mid_cjk = SpanBound {
            start: cjk + 1,
            end: source.len(),
        };
        assert!(!checker.check("Tavily first.", &mid_cjk, source));
        // A boundary-aligned span over the same text still verifies.
        let ok = SpanBound {
            start: 0,
            end: emoji_start,
        };
        assert!(checker.check("SearXNG", &ok, source));
    }
}
