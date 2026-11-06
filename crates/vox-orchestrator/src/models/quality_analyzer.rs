//! Deterministic Flesch-Kincaid `ResponseQualityAnalyzer` & `7d`/`30d`/`90d` Bucket Aggregator (`T3-CONSENSUS-QA-02..03`).
//!
//! Adapted from `lulkebit/t3-cloneathon` (`src/lib/response-quality-analyzer.ts`):
//! Computes a zero-LLM-cost `0..=100` `quality_score` for every response:
//! `quality_score = relevance * 0.30 + clarity * 0.25 + coherence * 0.20 + completeness * 0.20 + readability * 0.05`
//! using Flesch Reading Ease (`206.835 - 1.015 * ASL - 84.6 * ASW`), vowel-group syllable counting,
//! 19 logical connectors, structural markdown features, and prompt-keyword overlap.

use serde::{Deserialize, Serialize};

const LOGICAL_CONNECTORS: &[&str] = &[
    "however",
    "therefore",
    "furthermore",
    "moreover",
    "consequently",
    "nevertheless",
    "additionally",
    "specifically",
    "for example",
    "for instance",
    "in contrast",
    "on the other hand",
    "as a result",
    "in conclusion",
    "first",
    "second",
    "third",
    "finally",
    "meanwhile",
];

/// Quality bucket classification (`7d` / `30d` / `90d` analytics distribution).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityBucket {
    /// `90.0..=100.0`
    Excellent,
    /// `80.0..<90.0`
    Good,
    /// `70.0..<80.0`
    Fair,
    /// `60.0..<70.0`
    Poor,
    /// `< 60.0`
    VeryPoor,
}

impl QualityBucket {
    /// Classify a `0..=100` `quality_score` into its bucket.
    #[must_use]
    pub fn from_score(score: f64) -> Self {
        if score >= 90.0 {
            Self::Excellent
        } else if score >= 80.0 {
            Self::Good
        } else if score >= 70.0 {
            Self::Fair
        } else if score >= 60.0 {
            Self::Poor
        } else {
            Self::VeryPoor
        }
    }
}

/// Per-turn deterministic quality metrics (`ResponseQualityMetrics`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResponseQualityMetrics {
    pub quality_score: f64,
    pub relevance_score: f64,
    pub clarity_score: f64,
    pub coherence_score: f64,
    pub completeness_score: f64,
    pub readability_score: f64,
    pub bucket: QualityBucket,
}

/// Count English vowel-group syllables in a word (`a, e, i, o, u, y`), subtracting silent trailing `e`
/// and clamping to `>= 1`.
#[must_use]
pub fn count_syllables_in_word(word: &str) -> usize {
    let cleaned: String = word
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if cleaned.len() <= 3 {
        return 1;
    }
    let bytes = cleaned.as_bytes();
    let is_vowel = |b: u8| matches!(b, b'a' | b'e' | b'i' | b'o' | b'u' | b'y');
    let mut groups = 0usize;
    let mut prev_vowel = false;
    for &b in bytes {
        let v = is_vowel(b);
        if v && !prev_vowel {
            groups += 1;
        }
        prev_vowel = v;
    }
    if cleaned.ends_with("es") || cleaned.ends_with("ed") || cleaned.ends_with('e') {
        if !cleaned.ends_with("le") && groups > 1 {
            groups -= 1;
        }
    }
    groups.max(1)
}

/// Compute Flesch Reading Ease (`0.0..=100.0`) from words and sentences.
#[must_use]
pub fn compute_flesch_reading_ease(text: &str) -> f64 {
    let words: Vec<&str> = text.split_whitespace().filter(|w| !w.is_empty()).collect();
    if words.is_empty() {
        return 50.0;
    }
    let sentence_count = text
        .split(['.', '!', '?'])
        .filter(|s| !s.trim().is_empty())
        .count()
        .max(1);
    let total_syllables: usize = words.iter().map(|w| count_syllables_in_word(w)).sum();
    let asl = (words.len() as f64) / (sentence_count as f64);
    let asw = (total_syllables as f64) / (words.len() as f64);
    (206.835 - 1.015 * asl - 84.6 * asw).clamp(0.0, 100.0)
}

/// Deterministically analyze `response` against `prompt` without calling an LLM judge.
#[must_use]
pub fn analyze_response_quality(prompt: &str, response: &str) -> ResponseQualityMetrics {
    let lower_resp = response.to_ascii_lowercase();
    let prompt_keywords: Vec<String> = prompt
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .map(|w| w.to_ascii_lowercase())
        .filter(|w| w.len() > 3)
        .collect();

    // 1. Relevance (`0..=100`): keyword overlap + direct answer bonus
    let overlap_ratio = if prompt_keywords.is_empty() {
        0.8
    } else {
        let matched = prompt_keywords
            .iter()
            .filter(|kw| lower_resp.contains(kw.as_str()))
            .count();
        (matched as f64) / (prompt_keywords.len() as f64)
    };
    let relevance_score = (55.0 + overlap_ratio * 45.0).clamp(0.0, 100.0);

    // 2. Readability (`0..=100`): Flesch Reading Ease
    let readability_score = compute_flesch_reading_ease(response);

    // 3. Clarity (`0..=100`): structural formatting (code blocks, lists, headings) + readability blend
    let has_code_or_list =
        response.contains("```") || response.contains("\n- ") || response.contains("\n1. ");
    let structure_bonus = if has_code_or_list { 15.0 } else { 5.0 };
    let clarity_score = (65.0 + structure_bonus + readability_score * 0.20).clamp(0.0, 100.0);

    // 4. Coherence (`0..=100`): logical connectors + multi-paragraph flow
    let connector_hits = LOGICAL_CONNECTORS
        .iter()
        .filter(|&&conn| lower_resp.contains(conn))
        .count();
    let coherence_score = (70.0 + (connector_hits.min(5) as f64) * 6.0).clamp(0.0, 100.0);

    // 5. Completeness (`0..=100`): non-truncated response with substantive length
    let word_count = response.split_whitespace().count();
    let completeness_score = if word_count >= 25 {
        92.0
    } else if word_count >= 8 {
        80.0
    } else if word_count > 0 {
        65.0
    } else {
        0.0
    };

    let quality_score = (relevance_score * 0.30
        + clarity_score * 0.25
        + coherence_score * 0.20
        + completeness_score * 0.20
        + readability_score * 0.05)
        .clamp(0.0, 100.0);

    ResponseQualityMetrics {
        quality_score,
        relevance_score,
        clarity_score,
        coherence_score,
        completeness_score,
        readability_score,
        bucket: QualityBucket::from_score(quality_score),
    }
}

/// Timestamped quality observation for `7d`/`30d`/`90d` window aggregation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimestampedQualityRecord {
    pub recorded_at_ms: u64,
    pub model_id: String,
    pub metrics: ResponseQualityMetrics,
}

/// Aggregated quality distribution over a rolling time window (`7d`, `30d`, `90d`).
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualityWindowSummary {
    pub sample_count: usize,
    pub mean_quality_score: f64,
    pub excellent_count: usize,
    pub good_count: usize,
    pub fair_count: usize,
    pub poor_count: usize,
    pub very_poor_count: usize,
}

/// Aggregate quality records within `[now_ms - window_days * 86_400_000, now_ms]`.
#[must_use]
pub fn aggregate_quality_window(
    records: &[TimestampedQualityRecord],
    now_ms: u64,
    window_days: u64,
) -> QualityWindowSummary {
    let cutoff = now_ms.saturating_sub(window_days.saturating_mul(86_400_000));
    let mut summary = QualityWindowSummary::default();
    let mut sum = 0.0;
    for r in records.iter().filter(|r| r.recorded_at_ms >= cutoff) {
        summary.sample_count += 1;
        sum += r.metrics.quality_score;
        match r.metrics.bucket {
            QualityBucket::Excellent => summary.excellent_count += 1,
            QualityBucket::Good => summary.good_count += 1,
            QualityBucket::Fair => summary.fair_count += 1,
            QualityBucket::Poor => summary.poor_count += 1,
            QualityBucket::VeryPoor => summary.very_poor_count += 1,
        }
    }
    if summary.sample_count > 0 {
        summary.mean_quality_score = sum / (summary.sample_count as f64);
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::{
        QualityBucket, TimestampedQualityRecord, aggregate_quality_window,
        analyze_response_quality, count_syllables_in_word,
    };

    #[test]
    fn syllable_counter_and_response_quality_analyzer_score_deterministically() {
        assert_eq!(count_syllables_in_word("code"), 1);
        assert_eq!(count_syllables_in_word("orchestrator"), 4);

        let prompt = "How does VoxIdAllocatorV2 derive deterministic entity IDs?";
        let response = "First, `VoxIdAllocatorV2` uses `percent_encode_id_segment` on the namespace and each part. Therefore, deterministic entity IDs never collide across event replays.\n\n```rust\nlet id = alloc.derive(\"turn\", &[\"thread:1\"]);\n```";
        let m = analyze_response_quality(prompt, response);
        assert!(m.quality_score >= 75.0, "score: {}", m.quality_score);
        assert!(matches!(
            m.bucket,
            QualityBucket::Good | QualityBucket::Excellent
        ));

        let now = 1_700_000_000_000;
        let records = vec![TimestampedQualityRecord {
            recorded_at_ms: now - 1_000,
            model_id: "claude-sonnet-4-5".into(),
            metrics: m,
        }];
        let win7 = aggregate_quality_window(&records, now, 7);
        assert_eq!(win7.sample_count, 1);
    }
}
