use crate::claim_extractor::atomic::{AtomicConfig, AtomicDecomposer};
use crate::claim_extractor::constrained::validate_claim_envelope;
use crate::claim_extractor::minicheck::MiniCheckVerifier;
use crate::claim_extractor::span::SpanChecker;
use crate::claim_extractor::types::{AtomicClaim, ClaimVerdict, ExtractionResult};
use crate::claim_extractor::veriscore::{VeriScoreConfig, VeriScoreGate};

#[derive(Debug, Clone)]
pub struct ExtractionConfig {
    pub veriscore: VeriScoreConfig,
    pub atomic: AtomicConfig,
    pub abstain_threshold: f64,
    pub promotion_threshold: f64,
    /// Minimum `contradiction_score` from the verifier that promotes a verdict to
    /// `Contradicted` instead of `Abstain` or `Contested`.  Checked before the
    /// abstain/support/contest ladder so a high-confidence contradiction always wins.
    pub contradiction_threshold: f64,
}

impl Default for ExtractionConfig {
    fn default() -> Self {
        Self {
            veriscore: VeriScoreConfig::default(),
            atomic: AtomicConfig::default(),
            abstain_threshold: 0.3,
            promotion_threshold: 0.7,
            contradiction_threshold: 0.6,
        }
    }
}

pub struct ExtractionPipeline {
    config: ExtractionConfig,
    gate: VeriScoreGate,
    decomposer: AtomicDecomposer,
    span_checker: SpanChecker,
    verifier: MiniCheckVerifier,
}

impl ExtractionPipeline {
    pub fn new(config: ExtractionConfig) -> Self {
        let gate = VeriScoreGate::new(config.veriscore.clone());
        let decomposer = AtomicDecomposer::new(config.atomic.clone());
        let span_checker = SpanChecker::default();
        let verifier = MiniCheckVerifier::from_env();
        Self {
            config,
            gate,
            decomposer,
            span_checker,
            verifier,
        }
    }

    pub async fn extract(
        &self,
        source_text: &str,
        context_passages: &[&str],
    ) -> Result<ExtractionResult, Box<dyn std::error::Error + Send + Sync>> {
        let indexed_sentences = split_sentences_with_offsets(source_text);
        // `filter_sentences` takes plain text and doesn't carry offsets (VeriScore gating
        // is out of scope for this fix); map each surviving `&str` back to its
        // source-absolute byte offset by pointer identity into `sentences` below.
        let sentences: Vec<String> = indexed_sentences.iter().map(|(_, s)| s.clone()).collect();
        let offset_by_ptr: std::collections::HashMap<usize, usize> = sentences
            .iter()
            .zip(indexed_sentences.iter())
            .map(|(s, (offset, _))| (s.as_str().as_ptr() as usize, *offset))
            .collect();
        let verifiable = self.gate.filter_sentences(&sentences);
        let abstained = sentences.len() - verifiable.len();

        let mut all_claims: Vec<AtomicClaim> = Vec::new();
        for (sentence, _score) in &verifiable {
            all_claims.extend(claims_for_sentence(
                &self.decomposer,
                sentence,
                &offset_by_ptr,
            ));
        }

        let valid_claims: Vec<AtomicClaim> = all_claims
            .into_iter()
            .filter(|c| self.span_checker.check(&c.text, &c.span, source_text))
            .collect();

        // Stage 6: Constrained envelope validation
        let valid_claims: Vec<AtomicClaim> = valid_claims.into_iter()
            .filter_map(|c| {
                match serde_json::to_value(&c) {
                    Ok(json) => {
                        if validate_claim_envelope(&json).is_ok() { Some(c) } else { None }
                    }
                    Err(e) => {
                        tracing::warn!(claim_id = c.id, error = %e, "claim serialization failed; dropping");
                        None
                    }
                }
            })
            .collect();

        let context = context_passages.join(" ");
        let mut verdicts: Vec<ClaimVerdict> = Vec::new();
        let mut promotable: Vec<u64> = Vec::new();

        for claim in &valid_claims {
            let output = self.verifier.verify_claim(&claim.text, &context).await?;
            // Contradiction is checked first: a high contradiction_score overrides
            // the abstain/support/contest ladder regardless of support_score.
            let verdict = if output.contradiction_score >= self.config.contradiction_threshold {
                ClaimVerdict::Contradicted {
                    confidence: output.contradiction_score,
                }
            } else if output.abstained {
                ClaimVerdict::Abstain {
                    reason: format!(
                        "support_score={:.2} < τ={:.2}",
                        output.support_score, self.config.abstain_threshold
                    ),
                }
            } else if output.support_score >= self.config.promotion_threshold {
                promotable.push(claim.id);
                ClaimVerdict::Supported {
                    confidence: output.support_score,
                }
            } else {
                ClaimVerdict::Contested {
                    confidence: output.support_score,
                }
            };
            verdicts.push(verdict);
        }

        Ok(ExtractionResult {
            source_text: source_text.to_string(),
            claims: valid_claims,
            verdicts,
            promotable_claim_ids: promotable,
            abstained_sentence_count: abstained,
        })
    }
}

/// Decomposes `sentence` and shifts its (sentence-relative) claim spans to be
/// source-absolute, using `sentence`'s byte offset in `offset_by_ptr` — looked
/// up by pointer identity, since `VeriScoreGate::filter_sentences` borrows
/// `&str`s directly out of the same `sentences: Vec<String>` that
/// `offset_by_ptr` was built from (see `extract`).
///
/// # Invariant
/// `sentence`'s pointer must be present in `offset_by_ptr`. That holds today
/// because the gate only ever borrows; it does not hold if the gate is ever
/// changed to trim, normalize, or otherwise return owned/re-allocated
/// sentence strings. A silent fallback to offset 0 in that case would exactly
/// reproduce Task 15b's original bug (every claim after the first sentence
/// checked against the wrong region of `source_text`, and dropped) with no
/// signal that it had regressed. So a miss is treated as a broken invariant,
/// not a benign default: logged loudly, `debug_assert!`-ed so tests catch it,
/// and the sentence is skipped rather than silently mis-offset.
fn claims_for_sentence(
    decomposer: &AtomicDecomposer,
    sentence: &str,
    offset_by_ptr: &std::collections::HashMap<usize, usize>,
) -> Vec<AtomicClaim> {
    let Some(sentence_offset) = offset_by_ptr.get(&(sentence.as_ptr() as usize)).copied() else {
        tracing::error!(
            sentence = %sentence,
            "offset-by-pointer-identity invariant broken: VeriScoreGate::filter_sentences returned a \
             sentence not borrowed from the original `sentences` Vec. Skipping it rather than silently \
             checking its claims against the wrong region of source_text (Task 15b's original bug)."
        );
        debug_assert!(
            false,
            "claims_for_sentence: pointer-identity miss on offset_by_ptr — VeriScoreGate::filter_sentences \
             must keep borrowing directly from `sentences` for claim spans to stay source-absolute"
        );
        return Vec::new();
    };
    decomposer
        .decompose(sentence)
        .into_iter()
        .map(|mut c| {
            c.span.start += sentence_offset;
            c.span.end += sentence_offset;
            c
        })
        .collect()
}

/// Test-only convenience over `split_sentences_with_offsets`: `extract` needs the
/// offsets to shift claim spans, but the splitting-rules tests below only assert
/// on sentence text.
#[cfg(test)]
fn split_sentences(text: &str) -> Vec<String> {
    split_sentences_with_offsets(text)
        .into_iter()
        .map(|(_, s)| s)
        .collect()
}

/// Same splitting rules as `split_sentences`, but also returns each sentence's
/// byte offset (source-absolute, char-boundary-aligned) within `text` — needed
/// so downstream claim spans can be shifted from sentence-relative to
/// source-absolute before `SpanChecker::check` verifies them against `text`.
fn split_sentences_with_offsets(text: &str) -> Vec<(usize, String)> {
    /// Trailing tokens after which a `.` does not end a sentence ("e.g.", "vs.", …).
    const NON_TERMINAL_SUFFIXES: [&str; 4] = ["e.g", "i.e", "etc", "vs"];
    let indexed: Vec<(usize, char)> = text.char_indices().collect();
    let mut sentences = Vec::new();
    let mut current = String::new();
    let mut seg_start_byte = 0usize;
    for i in 0..indexed.len() {
        let (byte_idx, ch) = indexed[i];
        current.push(ch);
        let terminal = match ch {
            '!' | '?' => true,
            '.' => {
                // "12.5ms", "v0.6.2": a dot between digits is decimal/version punctuation.
                let prev_digit = i > 0 && indexed[i - 1].1.is_ascii_digit();
                let next_digit = indexed.get(i + 1).is_some_and(|(_, c)| c.is_ascii_digit());
                let mid_number = prev_digit && next_digit;
                let trimmed = current.trim_end_matches('.');
                let abbrev = NON_TERMINAL_SUFFIXES
                    .iter()
                    .any(|s| trimmed.to_lowercase().ends_with(s));
                let next_starts_sentence =
                    match indexed[i + 1..].iter().find(|(_, c)| !c.is_whitespace()) {
                        None => true,
                        Some((_, c)) => {
                            c.is_uppercase()
                                || c.is_ascii_digit()
                                || matches!(c, '"' | '\'' | '(' | '[')
                        }
                    };
                !mid_number && !abbrev && next_starts_sentence
            }
            _ => false,
        };
        if terminal {
            let end_byte = byte_idx + ch.len_utf8();
            let raw = &text[seg_start_byte..end_byte];
            let t = raw.trim();
            if !t.is_empty() {
                let leading_ws = raw.len() - raw.trim_start().len();
                sentences.push((seg_start_byte + leading_ws, t.to_string()));
            }
            current.clear();
            seg_start_byte = end_byte;
        }
    }
    let raw = &text[seg_start_byte..];
    let t = raw.trim();
    if !t.is_empty() {
        let leading_ws = raw.len() - raw.trim_start().len();
        sentences.push((seg_start_byte + leading_ws, t.to_string()));
    }
    sentences
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Round 1 fix: `claims_for_sentence` must not silently fall back to offset
    /// 0 on a pointer-identity miss — that would exactly reproduce Task 15b's
    /// original bug with no signal. Build `offset_by_ptr` from one String
    /// allocation and pass a *different* allocation (same text) as `sentence`,
    /// simulating a `VeriScoreGate::filter_sentences` that started returning
    /// owned/re-allocated strings instead of borrowing. Direct unit test of the
    /// helper (rather than injecting a fake gate into `ExtractionPipeline`,
    /// which has no seam for one) per the reviewer's fallback instruction.
    #[test]
    #[should_panic(expected = "pointer-identity miss")]
    fn claims_for_sentence_debug_asserts_on_pointer_identity_miss() {
        let decomposer = AtomicDecomposer::default();
        let owned_elsewhere = String::from("Latency rose by 10ms.");
        let mut offset_by_ptr = std::collections::HashMap::new();
        offset_by_ptr.insert(owned_elsewhere.as_str().as_ptr() as usize, 5usize);

        let sentence = String::from("Latency rose by 10ms."); // distinct allocation, same text
        let _ = claims_for_sentence(&decomposer, &sentence, &offset_by_ptr);
    }

    #[test]
    fn split_does_not_break_decimal_numbers() {
        let s = split_sentences("Latency fell to 12.5ms. Throughput rose 3.4x! Done?");
        assert_eq!(
            s,
            vec!["Latency fell to 12.5ms.", "Throughput rose 3.4x!", "Done?"]
        );
    }

    #[test]
    fn split_does_not_break_common_abbreviations_or_versions() {
        let s = split_sentences("Vox v0.6.2 ships today. See e.g. the docs.");
        assert_eq!(s.len(), 2, "got: {s:?}");
    }

    #[test]
    fn split_handles_trailing_unterminated_text() {
        let s = split_sentences("First sentence. Trailing fragment without period");
        assert_eq!(s.len(), 2);
    }

    #[tokio::test]
    async fn pipeline_extracts_from_verifiable_sentence() {
        let pipeline = ExtractionPipeline::new(ExtractionConfig::default());
        let result = pipeline
            .extract(
                "Provider X p95 latency increased by 12ms after the April 2026 model update.",
                &[],
            )
            .await
            .unwrap();
        assert!(!result.claims.is_empty());
    }

    #[tokio::test]
    async fn pipeline_abstains_on_hedge() {
        let pipeline = ExtractionPipeline::new(ExtractionConfig::default());
        let result = pipeline
            .extract("Future work may potentially explore improvements.", &[])
            .await
            .unwrap();
        assert!(result.promotable_claim_ids.is_empty());
        assert!(result.abstained_sentence_count > 0);
    }

    /// When the context sentence explicitly negates the claim, the pipeline must
    /// emit at least one `Contradicted` verdict.
    ///
    /// Mock scoring for this pair:
    ///   claim = "The cache reduces latency."  (4 words)
    ///   context sentence = "The cache does not reduce latency."
    ///   overlap = 3/4 = 0.75  →  contradiction_score = 0.75
    ///   default contradiction_threshold = 0.6  →  0.75 ≥ 0.6  →  Contradicted ✓
    #[tokio::test]
    async fn negated_claim_against_contradicting_context_is_contradicted() {
        let pipeline = ExtractionPipeline::new(ExtractionConfig::default());
        let result = pipeline
            .extract(
                "The cache reduces latency.",
                &["The cache does not reduce latency."],
            )
            .await
            .unwrap();
        let has_contradicted = result
            .verdicts
            .iter()
            .any(|v| matches!(v, ClaimVerdict::Contradicted { .. }));
        assert!(
            has_contradicted,
            "expected at least one Contradicted verdict, got: {:?}",
            result.verdicts
        );
    }

    /// Task 15b: a claim in the third sentence must survive span checking. Before
    /// the fix, its span was measured relative to sentence 3 but checked against
    /// the whole `source_text`, so the word-overlap check compared it against an
    /// unrelated prefix of the source and (almost always) dropped it.
    #[tokio::test]
    async fn claim_in_third_sentence_survives_span_check() {
        let pipeline = ExtractionPipeline::new(ExtractionConfig::default());
        let source = "The report opens with background context here. \
                       This is filler text that also passes the veriscore gate. \
                       Provider X p95 latency increased by 12ms after the update.";
        let result = pipeline.extract(source, &[]).await.unwrap();
        assert!(
            result.claims.iter().any(|c| c.text.contains("12ms")),
            "expected the sentence-3 claim to survive span checking, got claims: {:?}",
            result.claims.iter().map(|c| &c.text).collect::<Vec<_>>()
        );
    }

    /// Two identical sentences at different positions: each claim's span must
    /// point at its own occurrence, not both resolve to the first one.
    #[tokio::test]
    async fn duplicate_sentences_each_keep_their_own_span() {
        let pipeline = ExtractionPipeline::new(ExtractionConfig::default());
        let source = "Latency rose by 10ms today. \
                       Some unrelated middle sentence with no digits at all. \
                       Latency rose by 10ms today.";
        let result = pipeline.extract(source, &[]).await.unwrap();
        let matches: Vec<_> = result
            .claims
            .iter()
            .filter(|c| c.text.contains("Latency rose by 10ms"))
            .collect();
        assert_eq!(
            matches.len(),
            2,
            "expected both occurrences to survive, got: {matches:?}"
        );
        assert_ne!(
            matches[0].span.start, matches[1].span.start,
            "each occurrence should keep its own source-absolute span"
        );
        for c in &matches {
            let slice = source
                .get(c.span.start..c.span.end)
                .expect("span must be char-boundary aligned");
            assert!(slice.contains("Latency rose by 10ms"));
        }
    }

    /// Multi-byte characters (emoji/CJK) before the claim sentence must not shift
    /// the claim's span off a char boundary; guards the c616e4f5e panic class.
    #[tokio::test]
    async fn multibyte_prefix_does_not_break_span_alignment() {
        let pipeline = ExtractionPipeline::new(ExtractionConfig::default());
        let source = "SearXNG 🥈 second place; 検索エンジン comparison happened here today. \
                       Provider X p95 latency increased by 12ms after the update.";
        let result = pipeline.extract(source, &[]).await.unwrap();
        let claim = result
            .claims
            .iter()
            .find(|c| c.text.contains("12ms"))
            .unwrap_or_else(|| {
                panic!(
                    "expected the numeric claim to survive, got: {:?}",
                    result.claims
                )
            });
        let slice = source
            .get(claim.span.start..claim.span.end)
            .expect("span must remain on a char boundary despite multi-byte prefix");
        assert!(slice.contains("12ms"));
    }
}
