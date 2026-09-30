use futures::stream::{self, StreamExt};

use crate::claim_extractor::atomic::{AtomicConfig, AtomicDecomposer};
use crate::claim_extractor::constrained::validate_claim_envelope;
use crate::claim_extractor::minicheck::MiniCheckVerifier;
use crate::claim_extractor::span::SpanChecker;
use crate::claim_extractor::types::{AtomicClaim, ClaimVerdict, ExtractionResult};
use crate::claim_extractor::veriscore::{VeriScoreConfig, VeriScoreGate};

/// Task 15d: a live deep-research run verifying claims sequentially (this
/// loop, and its sibling in `vox-research-shim::research::verifier`) took
/// 322s for ~50 claims and blew the 180s chat dispatch timeout.
///
/// Review round 1 (B2): the per-run *cap* stays only in
/// `vox-research-shim::research::verifier`, where the LLM latency actually
/// lives. This shared `ExtractionPipeline` is also used by `vox scientia`
/// publication scoring (`vox-cli`'s `scientia_phase_handlers.rs`, which counts
/// `Supported` toward `claim_evidence_coverage` and writes every verdict to
/// the claim ledger) and by `vox_research_shim::research::claims::extract_claims_from_text`
/// (which discards `result.verdicts` and keeps every claim regardless) — a cap
/// here would silently change publication-worthiness scoring for a
/// manuscript with more than 24 claims, while saving no latency on the
/// research path at all. Only bounded *concurrency* is kept here.
const VERIFY_CONCURRENCY: usize = 8;

/// Verifies `claims[indices]` concurrently (bounded by `concurrency`),
/// running `verify_one` for each and pairing each result with its original
/// index. `buffered` (not `buffer_unordered`) preserves the order `indices`
/// was given in. Always compiled and generic over `verify_one` so it is unit
/// testable with a stub, independent of any real verifier backend.
async fn verify_indices_concurrently<F, Fut>(
    claims: &[AtomicClaim],
    indices: &[usize],
    concurrency: usize,
    verify_one: F,
) -> Vec<(usize, ClaimVerdict)>
where
    F: Fn(AtomicClaim) -> Fut,
    Fut: std::future::Future<Output = ClaimVerdict>,
{
    // Collect into an owned Vec first (rather than piping a closure that
    // borrows `claims` straight into `stream::iter`) — see the identical note
    // in vox-research-shim's `verifier::verify_indices_concurrently`: this
    // avoids rustc inferring an over-specific higher-ranked lifetime that
    // later fails "implementation of Send/FnOnce is not general enough"
    // wherever this async fn's future gets composed into a `'static` future.
    let pairs: Vec<(usize, AtomicClaim)> =
        indices.iter().map(|&i| (i, claims[i].clone())).collect();
    stream::iter(pairs)
        .map(|(i, claim)| {
            let fut = verify_one(claim);
            async move { (i, fut.await) }
        })
        .buffered(concurrency.max(1))
        .collect()
        .await
}

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
        let contradiction_threshold = self.config.contradiction_threshold;
        let abstain_threshold = self.config.abstain_threshold;
        let promotion_threshold = self.config.promotion_threshold;
        let verifier = &self.verifier;
        let all_indices: Vec<usize> = (0..valid_claims.len()).collect();

        // Contradiction is checked first: a high contradiction_score overrides
        // the abstain/support/contest ladder regardless of support_score. A
        // failed verification (`Err`) becomes an honest `Abstain`, never a
        // `?` that aborts the whole batch over one claim.
        let verify_one = |claim: AtomicClaim| {
            let context = &context;
            async move {
                match verifier.verify_claim(&claim.text, context).await {
                    Ok(output) => {
                        if output.contradiction_score >= contradiction_threshold {
                            ClaimVerdict::Contradicted {
                                confidence: output.contradiction_score,
                            }
                        } else if output.abstained {
                            ClaimVerdict::Abstain {
                                reason: format!(
                                    "support_score={:.2} < τ={:.2}",
                                    output.support_score, abstain_threshold
                                ),
                            }
                        } else if output.support_score >= promotion_threshold {
                            ClaimVerdict::Supported {
                                confidence: output.support_score,
                            }
                        } else {
                            ClaimVerdict::Contested {
                                confidence: output.support_score,
                            }
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            claim_id = claim.id,
                            error = %e,
                            "claim verification failed; marking unverified rather than aborting the batch"
                        );
                        ClaimVerdict::Abstain {
                            reason: format!("verification error: {e}"),
                        }
                    }
                }
            }
        };

        // No per-run cap here (B2): every claim in `valid_claims` is verified,
        // just concurrently rather than sequentially. `verify_indices_concurrently`
        // already preserves input order via `buffered`, so the result is
        // already index-sorted — no separate merge/sort step needed once
        // there's no second (capped) source to interleave.
        let results = verify_indices_concurrently(
            &valid_claims,
            &all_indices,
            VERIFY_CONCURRENCY,
            verify_one,
        )
        .await;
        let verdicts: Vec<ClaimVerdict> = results.into_iter().map(|(_, v)| v).collect();

        // Recomputed from the verdict tag rather than tracked during
        // verification: `Supported` is only ever produced above when
        // `support_score >= promotion_threshold`, so this stays exactly
        // equivalent to the original per-claim check.
        let promotable: Vec<u64> = valid_claims
            .iter()
            .zip(verdicts.iter())
            .filter_map(|(c, v)| matches!(v, ClaimVerdict::Supported { .. }).then_some(c.id))
            .collect();

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
    use crate::claim_extractor::types::{SpanBound, VerifiabilityClass};

    fn stub_claim(id: u64, score: f64) -> AtomicClaim {
        AtomicClaim {
            id,
            text: format!("claim {id}"),
            tuple: None,
            span: SpanBound { start: 0, end: 0 },
            verifiability: VerifiabilityClass::Semantic,
            verifiability_score: score,
        }
    }

    /// Task 15d concurrency proof: with `VERIFY_CONCURRENCY` in-flight at a
    /// time, verifying 16 claims that each take a fixed 200ms must finish in
    /// well under the 16 * 200ms a sequential loop would take. Mutation
    /// guard: setting `VERIFY_CONCURRENCY` to 1 makes this fail (16 * 200ms =
    /// 3.2s > the 1.5s bound), proving the bound is real, not vacuous.
    #[tokio::test]
    async fn verify_indices_concurrently_bounds_wall_clock_time() {
        let claims: Vec<AtomicClaim> = (0..16).map(|i| stub_claim(i, 0.9)).collect();
        let indices: Vec<usize> = (0..claims.len()).collect();

        let start = std::time::Instant::now();
        let results = verify_indices_concurrently(
            &claims,
            &indices,
            VERIFY_CONCURRENCY,
            |_claim| async move {
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                ClaimVerdict::Supported { confidence: 0.9 }
            },
        )
        .await;
        let elapsed = start.elapsed();

        assert!(
            elapsed < std::time::Duration::from_millis(1500),
            "16 claims at 200ms each with concurrency {VERIFY_CONCURRENCY} took {elapsed:?}, expected well under 16*200ms"
        );
        // verdicts stay aligned with claim order (`buffered`, not `buffer_unordered`).
        let idx: Vec<usize> = results.iter().map(|(i, _)| *i).collect();
        assert_eq!(idx, (0..16).collect::<Vec<usize>>());
        assert!(
            results
                .iter()
                .all(|(_, v)| matches!(v, ClaimVerdict::Supported { .. }))
        );
    }

    /// Task 15d review round 1 (B3): the previous version of this test
    /// stubbed `verify_indices_concurrently`'s `verify_one` closure to return
    /// `Abstain` directly and then asserted `Abstain` — circular, since it
    /// never exercised `extract`'s actual `Err => Abstain` arm (deleting that
    /// arm, or having it return `Supported`, would still have passed).
    ///
    /// This version builds a real `ExtractionPipeline` whose verifier is
    /// `MiniCheckBackend::Http` pointed at an unreachable loopback port
    /// (`127.0.0.1:1` — nothing listens there, so the connection is refused
    /// immediately; no real network egress beyond that refused local
    /// connection). Every claim's `verify_claim` call therefore genuinely
    /// returns `Err`, and `extract` must still return `Ok` with every claim
    /// marked `Abstain { reason: "verification error: ..." }` — proving the
    /// `?` is really gone, not just that some intermediate helper preserves
    /// item counts.
    ///
    /// Mutation guard (manually verified, not committed as a second test):
    /// temporarily changing `extract`'s `Err(e) => ClaimVerdict::Abstain {
    /// ... }` arm back to `let output = verifier.verify_claim(...).await?;`
    /// makes this test fail with an `Err` return instead of `Ok`, and
    /// changing the arm to `ClaimVerdict::Supported { confidence: 1.0 }`
    /// makes the `all(|v| matches!(v, Abstain))` assertion fail.
    #[tokio::test]
    async fn extract_marks_every_claim_abstain_on_real_verification_error() {
        use crate::claim_extractor::minicheck::MiniCheckBackend;

        let pipeline = ExtractionPipeline {
            config: ExtractionConfig::default(),
            gate: VeriScoreGate::new(VeriScoreConfig::default()),
            decomposer: AtomicDecomposer::new(AtomicConfig::default()),
            span_checker: SpanChecker::default(),
            verifier: MiniCheckVerifier {
                backend: MiniCheckBackend::Http {
                    endpoint: "http://127.0.0.1:1".to_string(),
                },
                abstain_threshold: 0.3,
            },
        };

        let source = "Provider X p95 latency increased by 12ms after the update. \
                       Cache hit rate improved by 15 percent this quarter.";
        let result = pipeline
            .extract(source, &[])
            .await
            .expect("a failing verifier must not abort extract() via `?`");

        assert!(
            !result.claims.is_empty(),
            "expected at least one extracted claim to verify against"
        );
        assert_eq!(
            result.claims.len(),
            result.verdicts.len(),
            "every extracted claim must get exactly one verdict"
        );
        for verdict in &result.verdicts {
            match verdict {
                ClaimVerdict::Abstain { reason } => {
                    assert!(
                        reason.starts_with("verification error:"),
                        "expected the real Err path's reason text, got: {reason}"
                    );
                }
                other => panic!("expected Abstain for every claim, got: {other:?}"),
            }
        }
        assert!(
            result.promotable_claim_ids.is_empty(),
            "no claim can be promotable when every verdict is Abstain"
        );
    }

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
