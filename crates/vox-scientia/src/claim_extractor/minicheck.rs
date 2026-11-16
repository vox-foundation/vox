use crate::claim_extractor::atomic::fnv1a_hash;
use crate::claim_extractor::types::VerifierOutput;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MiniCheckBackend {
    #[cfg(test)]
    Mock,
    Http {
        endpoint: String,
    },
    /// Calibrated hybrid verifier combining lexical/numeric/negation containment
    /// with model-agnostic LLM entailment fallback via `vox_actor_runtime::llm`.
    CalibratedHybrid {
        model: String,
    },
}

pub struct MiniCheckVerifier {
    pub backend: MiniCheckBackend,
    pub abstain_threshold: f64,
}

/// Negation tokens used by the Mock backend's lexical-negation asymmetry heuristic.
/// If one side (claim or context sentence) contains any of these tokens and the other
/// does not, the pair is treated as a potential contradiction.  This is a surface-form
/// heuristic — it is NOT a full NLI model.
#[cfg(test)]
const NEGATORS: &[&str] = &[
    "not", "no", "never", "fails", "cannot", "doesn't", "does not", "won't", "isn't", "aren't",
];

// Serialization types for the MiniCheck HTTP API.
#[derive(Serialize)]
struct MiniCheckRequest<'a> {
    claim: &'a str,
    context: &'a str,
}

#[derive(Deserialize)]
struct MiniCheckResponse {
    #[serde(default)]
    verdict: Option<String>,
    #[serde(default)]
    confidence: Option<f64>,
    #[serde(default)]
    support_score: f64,
    /// Optional contradiction score returned by the endpoint.
    /// Absent in responses from endpoints that do not implement it (defaults to 0.0).
    #[serde(default)]
    contradiction_score: f64,
}

impl MiniCheckVerifier {
    #[cfg(test)]
    pub fn mock() -> Self {
        Self {
            backend: MiniCheckBackend::Mock,
            abstain_threshold: 0.3,
        }
    }

    /// Returns `true` only when backed by the test-only `Mock` verifier.
    #[must_use]
    pub const fn is_mock_backend(&self) -> bool {
        #[cfg(test)]
        {
            matches!(self.backend, MiniCheckBackend::Mock)
        }
        #[cfg(not(test))]
        {
            false
        }
    }

    pub fn http(endpoint: impl Into<String>) -> Self {
        Self {
            backend: MiniCheckBackend::Http {
                endpoint: endpoint.into(),
            },
            abstain_threshold: 0.3,
        }
    }

    pub fn calibrated_hybrid(model: impl Into<String>) -> Self {
        Self {
            backend: MiniCheckBackend::CalibratedHybrid {
                model: model.into(),
            },
            abstain_threshold: 0.3,
        }
    }

    /// Construct a production verifier: HTTP endpoint when `VOX_MINICHECK_ENDPOINT`
    /// is configured, otherwise `CalibratedHybrid` routed through `vox_actor_runtime::llm`.
    pub fn from_env_or_calibrated() -> Self {
        Self::from_env()
    }

    pub fn from_env_with_lookup(
        mut lookup: impl FnMut(&str) -> Result<String, std::env::VarError>,
    ) -> Self {
        if let Ok(url) = lookup("VOX_MINICHECK_ENDPOINT")
            && !url.trim().is_empty()
        {
            return Self::http(url.trim().to_string());
        }
        let model = lookup("VOX_MINICHECK_MODEL")
            .ok()
            .map(|m| m.trim().to_string())
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| vox_config::model_defaults::CLAIM_EXTRACTION.to_string());
        Self::calibrated_hybrid(model)
    }

    pub fn from_env() -> Self {
        let resolved_endpoint =
            vox_secrets::resolve_secret(vox_secrets::SecretId::VoxMinicheckEndpoint);
        if let Some(url) = resolved_endpoint
            .expose()
            .map(str::trim)
            .filter(|u| !u.is_empty())
        {
            return Self::http(url.to_string());
        }
        let resolved_model = vox_secrets::resolve_secret(vox_secrets::SecretId::VoxMinicheckModel);
        let model = resolved_model
            .expose()
            .map(str::trim)
            .filter(|m| !m.is_empty())
            .unwrap_or(vox_config::model_defaults::CLAIM_EXTRACTION);
        Self::calibrated_hybrid(model.to_string())
    }

    pub async fn verify_claim(
        &self,
        claim: &str,
        context: &str,
    ) -> Result<VerifierOutput, Box<dyn std::error::Error + Send + Sync>> {
        let claim_id = fnv1a_hash(claim);
        match &self.backend {
            #[cfg(test)]
            MiniCheckBackend::Mock => {
                let claim_words: Vec<&str> = claim.split_whitespace().collect();

                // Split context into sentences for best-overlap detection.
                // Naive split on '.', '!', '?' — sufficient for the heuristic.
                let context_sentences: Vec<&str> = context
                    .split(['.', '!', '?'])
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .collect();

                // Helper: word-overlap fraction for one context sentence.
                // Tokens are compared punctuation-stripped and inflection-tolerant
                // ("reduces" matches "reduce"): exact equality, or a shared prefix
                // when both tokens are ≥4 chars.
                let word_matches = |a: &str, b: &str| -> bool {
                    a == b
                        || (a.len() >= 4 && b.len() >= 4 && (a.starts_with(b) || b.starts_with(a)))
                };
                let norm = |w: &str| -> String {
                    w.trim_matches(|c: char| !c.is_alphanumeric())
                        .to_ascii_lowercase()
                };
                let overlap_with = |sentence: &str| -> f64 {
                    if claim_words.is_empty() {
                        return 0.5;
                    }
                    let sentence_words: Vec<String> = sentence
                        .split_whitespace()
                        .map(norm)
                        .filter(|w| !w.is_empty())
                        .collect();
                    let n = claim_words
                        .iter()
                        .map(|w| norm(w))
                        .filter(|w| !w.is_empty())
                        .filter(|w| sentence_words.iter().any(|s| word_matches(w, s)))
                        .count();
                    n as f64 / claim_words.len() as f64
                };

                // Find the context sentence with the highest overlap fraction.
                let best_sentence = context_sentences.iter().copied().max_by(|a, b| {
                    overlap_with(a)
                        .partial_cmp(&overlap_with(b))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

                let (overlap_score, best_sentence) = if let Some(s) = best_sentence {
                    (overlap_with(s), s)
                } else {
                    // No context at all: fall back to full-context overlap (original behaviour).
                    let n = if claim_words.is_empty() {
                        return Ok(VerifierOutput {
                            claim_id,
                            support_score: 0.5,
                            contradiction_score: 0.0,
                            abstained: 0.5_f64 < self.abstain_threshold,
                            verifier_model: "mock".to_string(),
                        });
                    } else {
                        claim_words
                            .iter()
                            .filter(|w| {
                                context
                                    .to_ascii_lowercase()
                                    .contains(&w.to_ascii_lowercase())
                            })
                            .count()
                    };
                    (n as f64 / claim_words.len() as f64, context)
                };

                // Lexical-negation asymmetry heuristic (NOT an NLI model):
                // if one of the two texts contains a negator and the other does not,
                // treat the pair as potentially contradictory.
                let contains_negator = |text: &str| -> bool {
                    let lower = text.to_ascii_lowercase();
                    NEGATORS.iter().any(|n| {
                        if n.contains(' ') {
                            // Multi-word negators ("does not") match as phrases.
                            lower.contains(n)
                        } else {
                            // Single-word negators match whole tokens only —
                            // "no" must not fire inside "know" or "nominal".
                            lower
                                .split_whitespace()
                                .map(|w| {
                                    w.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'')
                                })
                                .any(|w| w == *n)
                        }
                    })
                };
                let claim_negated = contains_negator(claim);
                let context_negated = contains_negator(best_sentence);
                let negation_asymmetry = claim_negated != context_negated;

                let (support_score, contradiction_score) = if negation_asymmetry {
                    // One side negates the other: high overlap with flipped polarity
                    // → contradiction.  Reduce support to signal inversion.
                    let reduced_support = (1.0 - overlap_score).min(overlap_score);
                    (reduced_support, overlap_score)
                } else {
                    let raw = 0.5 + 0.5 * overlap_score;
                    (raw, 0.0_f64)
                };

                Ok(VerifierOutput {
                    claim_id,
                    support_score,
                    contradiction_score,
                    abstained: support_score < self.abstain_threshold,
                    verifier_model: "mock".to_string(),
                })
            }
            MiniCheckBackend::Http { endpoint } => {
                let client = vox_http_client::client_builder()
                    .timeout(vox_config::timeouts::D_10S)
                    .build()?;
                let payload = MiniCheckRequest { claim, context };
                let resp = client
                    .post(endpoint)
                    .json(&payload)
                    .send()
                    .await?
                    .error_for_status()?;
                let body: MiniCheckResponse = resp.json().await?;
                let score = body.support_score.clamp(0.0, 1.0);
                let contradiction_score = body.contradiction_score.clamp(0.0, 1.0);
                Ok(VerifierOutput {
                    claim_id,
                    support_score: score,
                    contradiction_score,
                    abstained: score < self.abstain_threshold,
                    verifier_model: endpoint.clone(),
                })
            }
            MiniCheckBackend::CalibratedHybrid { model } => {
                let tier0 =
                    crate::claim_extractor::tier0_lexical::evaluate_tier0_lexical(claim, context);
                if tier0.contradiction_score >= 0.60 || tier0.support_score >= 0.75 {
                    return Ok(VerifierOutput {
                        claim_id,
                        support_score: tier0.support_score,
                        contradiction_score: tier0.contradiction_score,
                        abstained: tier0.support_score < self.abstain_threshold
                            && tier0.contradiction_score < 0.60,
                        verifier_model: "calibrated-hybrid:lexical".to_string(),
                    });
                }

                if let Some((support_score, contradiction_score)) =
                    llm_entailment_verify(model, claim, context).await
                {
                    let support_score = support_score.clamp(0.0, 1.0);
                    let contradiction_score = contradiction_score.clamp(0.0, 1.0);
                    if support_score.max(contradiction_score) >= self.abstain_threshold {
                        return Ok(VerifierOutput {
                            claim_id,
                            support_score,
                            contradiction_score,
                            abstained: false,
                            verifier_model: format!("calibrated-hybrid:{model}"),
                        });
                    }
                }

                Ok(VerifierOutput {
                    claim_id,
                    support_score: tier0.support_score,
                    contradiction_score: tier0.contradiction_score,
                    abstained: tier0.support_score < self.abstain_threshold
                        && tier0.contradiction_score < 0.60,
                    verifier_model: "calibrated-hybrid:lexical".to_string(),
                })
            }
        }
    }
}

async fn llm_entailment_verify(model: &str, claim: &str, context: &str) -> Option<(f64, f64)> {
    use vox_actor_runtime::ActivityResult;
    use vox_actor_runtime::llm::cascade::{ResearchStage, cascade_for_research_stage};
    use vox_actor_runtime::llm::{LlmChatMessage, infer_with_retry};
    use vox_actor_runtime::llm_result::maybe_strip_markdown_json_fences;
    use vox_actor_runtime::model_resolution::RouteResolutionInput;

    if context.trim().is_empty() || claim.trim().is_empty() || model.trim().is_empty() {
        return None;
    }
    let mut input = RouteResolutionInput::default();
    input.openrouter_model = model.to_string();
    let mut candidates = cascade_for_research_stage(ResearchStage::Verification, &input);
    if candidates.is_empty() {
        return None;
    }
    for candidate in &mut candidates {
        candidate.temperature = Some(0.0);
        candidate.max_tokens = Some(160);
    }
    let system = "You are a SciFact scientific claim entailment verifier. Given a claim and evidence context, classify entailment (Supported, Contradicted, or Abstain) and respond ONLY with JSON: {\"verdict\": \"Supported|Contradicted|Abstain\", \"support_score\": <0.0..1.0>, \"contradiction_score\": <0.0..1.0>}.".to_string();
    let truncated_ctx: String = context.chars().take(4000).collect();
    let user = format!("Claim: {claim}\nEvidence Context:\n{truncated_ctx}");
    let options = vox_actor_runtime::ActivityOptions::default();
    match infer_with_retry(
        &options,
        vec![
            LlmChatMessage {
                role: "system".to_string(),
                content: system,
                ..Default::default()
            },
            LlmChatMessage {
                role: "user".to_string(),
                content: user,
                ..Default::default()
            },
        ],
        candidates,
    )
    .await
    {
        ActivityResult::Ok(Ok((resp, _cfg))) => {
            let clean = maybe_strip_markdown_json_fences(&resp.content);
            let parsed: MiniCheckResponse = serde_json::from_str(clean).ok()?;
            if parsed.support_score > 0.0 || parsed.contradiction_score > 0.0 {
                return Some((parsed.support_score, parsed.contradiction_score));
            }
            if let Some(label) = parsed.verdict.as_deref() {
                let conf = parsed.confidence.unwrap_or(0.85).clamp(0.0, 1.0);
                return match label.trim().to_ascii_lowercase().as_str() {
                    "supported" | "support" => Some((conf, 0.0)),
                    "contradicted" | "contradict" => Some((0.0, conf)),
                    _ => Some((0.0, 0.0)),
                };
            }
            Some((parsed.support_score, parsed.contradiction_score))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_verifier_returns_result() {
        let verifier = MiniCheckVerifier::mock();
        assert!(verifier.is_mock_backend());
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt
            .block_on(verifier.verify_claim(
                "latency increased",
                "The provider's p95 latency rose by 12ms in April 2026.",
            ))
            .unwrap();
        assert!(result.support_score >= 0.0 && result.support_score <= 1.0);
    }

    #[test]
    fn negation_mismatch_yields_contradiction_score() {
        let verifier = MiniCheckVerifier::mock();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt
            .block_on(verifier.verify_claim(
                "The cache reduces latency.",
                "The cache does not reduce latency.",
            ))
            .unwrap();
        assert!(
            result.contradiction_score >= 0.5,
            "expected contradiction_score >= 0.5, got {}",
            result.contradiction_score
        );
        assert!(
            result.support_score < result.contradiction_score,
            "support_score {} should be less than contradiction_score {} when context negates claim",
            result.support_score,
            result.contradiction_score
        );
    }

    #[test]
    fn agreeing_text_has_zero_contradiction() {
        let verifier = MiniCheckVerifier::mock();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let text = "The cache reduces latency significantly.";
        let result = rt.block_on(verifier.verify_claim(text, text)).unwrap();
        assert_eq!(
            result.contradiction_score, 0.0,
            "identical affirmative text should have zero contradiction_score"
        );
    }

    #[test]
    fn calibrated_hybrid_supports_clean_containment_and_flags_numeric_conflict() {
        let verifier = MiniCheckVerifier::calibrated_hybrid("anthropic/claude-3.5-haiku");
        assert!(!verifier.is_mock_backend());
        let rt = tokio::runtime::Runtime::new().unwrap();
        let clean = rt
            .block_on(verifier.verify_claim(
                "The cache reduces latency by 35ms.",
                "The cache reduces latency by 35ms across all production workloads.",
            ))
            .unwrap();
        assert!(clean.support_score >= 0.85, "{clean:?}");
        assert_eq!(clean.contradiction_score, 0.0);
        assert_eq!(clean.verifier_model, "calibrated-hybrid:lexical");

        let neg = rt
            .block_on(verifier.verify_claim(
                "The cache reduces latency.",
                "The cache does not reduce latency.",
            ))
            .unwrap();
        assert!(neg.contradiction_score >= 0.85, "{neg:?}");
        assert!(neg.support_score <= 0.15, "{neg:?}");
    }

    #[test]
    fn tier0_calibrated_handles_antonym_numeric_and_statcheck_contradictions() {
        let verifier = MiniCheckVerifier::calibrated_hybrid("anthropic/claude-3.5-haiku");
        let rt = tokio::runtime::Runtime::new().unwrap();

        let antonym = rt
            .block_on(verifier.verify_claim(
                "The April 2026 deployment reduced p95 latency to 12ms.",
                "The April 2026 deployment increased p95 latency to 84ms across all regions.",
            ))
            .unwrap();
        assert!(antonym.contradiction_score >= 0.85, "{antonym:?}");
        assert!(antonym.support_score <= 0.15, "{antonym:?}");

        let numeric = rt
            .block_on(verifier.verify_claim(
                "Cache hit rate reached 95 percent under load.",
                "Cache hit rate reached 41 percent under load.",
            ))
            .unwrap();
        assert!(numeric.contradiction_score >= 0.85, "{numeric:?}");
        assert!(numeric.support_score <= 0.15, "{numeric:?}");

        let statcheck = rt
            .block_on(verifier.verify_claim(
                "The effect is statistically significant (p < 0.01).",
                "The effect is not statistically significant (p = 0.42).",
            ))
            .unwrap();
        assert!(statcheck.contradiction_score >= 0.85, "{statcheck:?}");
        assert!(statcheck.support_score <= 0.15, "{statcheck:?}");
    }
}

#[cfg(test)]
mod semcov_wave2_tests {
    #![allow(unused_imports)]
    use super::*;

    #[test]
    fn from_env_returns_calibrated_hybrid_when_env_var_absent() {
        let verifier =
            MiniCheckVerifier::from_env_with_lookup(|_| Err(std::env::VarError::NotPresent));
        assert!(
            matches!(verifier.backend, MiniCheckBackend::CalibratedHybrid { .. }),
            "expected CalibratedHybrid backend when VOX_MINICHECK_ENDPOINT is unset"
        );
        assert!(!verifier.is_mock_backend());
    }

    #[test]
    fn from_env_returns_http_when_env_var_set() {
        let verifier = MiniCheckVerifier::from_env_with_lookup(|k| {
            if k == "VOX_MINICHECK_ENDPOINT" {
                Ok("http://localhost:9090/verify".to_string())
            } else {
                Err(std::env::VarError::NotPresent)
            }
        });
        match verifier.backend {
            MiniCheckBackend::Http { endpoint } => {
                assert_eq!(endpoint, "http://localhost:9090/verify");
            }
            other => panic!("expected Http backend, got {:?}", other),
        }
    }
}
