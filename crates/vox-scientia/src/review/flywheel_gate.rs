//! 4-layer external scholarly & nanopublication emission gate (Phase 451 PUBL-02).
//!
//! Enforces:
//! 1. SHA3-256 content-digest-bound [`ApprovalToken`] verification.
//! 2. [`crate::critic_gate::evaluate_gate`] clearance + negative-result quota enforcement.
//! 3. [`NoveltyVerdict`] clearance (`Novel` or `PossiblyNovel` only).
//! 4. Scholarly trust / `OfflineAttested` fallback clearance.

use serde::{Deserialize, Serialize};
use thiserror::Error;

use super::ApprovalToken;
use crate::critic_gate::{GateInputs, GateReason, evaluate_gate};
use crate::inspect_bridge::NoveltyVerdict;

/// Compute lowercase hex SHA3-256 using `vox-crypto`'s compliance hash.
#[must_use]
pub fn sha3_256_hex(bytes: &[u8]) -> String {
    vox_crypto::hex_encode(&vox_crypto::compliance_hash(bytes))
}

/// Trust provenance supplied for layer 4 of [`ExternalEmissionGate`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum ScholarlyTrustMode {
    VerifiedOnline {
        trust_score: f64,
        min_trust_score: f64,
    },
    OfflineAttested {
        attestation_digest: String,
        signer_orcid: Option<String>,
    },
    Unverified,
}

/// Full 4-layer gate input bundle.
pub struct EmissionGateInput<'a> {
    pub publication_id: &'a str,
    pub token: &'a ApprovalToken,
    pub content_sha3_256: &'a str,
    pub canonical_payload: Option<&'a [u8]>,
    pub critic_inputs: &'a GateInputs<'a>,
    pub negative_result_quota: u32,
    pub null_results_in_window: u32,
    pub is_negative_result: bool,
    pub novelty_verdict: &'a NoveltyVerdict,
    pub scholarly_trust: &'a ScholarlyTrustMode,
}

/// Clearance receipt returned when all 4 gate layers pass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EmissionClearance {
    pub publication_id: String,
    pub content_sha3_256: String,
    pub critic_reason: GateReason,
    pub novelty_label: String,
    pub trust_mode: String,
}

/// Explicit rejection reason when any gate layer blocks emission.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum EmissionGateError {
    #[error("approval token bound to publication `{actual}`, expected `{expected}`")]
    ApprovalTokenPublicationMismatch { expected: String, actual: String },
    #[error("approval token bound to stale digest `{bound}`, current is `{current}`")]
    ApprovalTokenStaleDigest { bound: String, current: String },
    #[error("canonical payload SHA3-256 `{computed}` does not match manifest `{expected}`")]
    PayloadSha3Mismatch { expected: String, computed: String },
    #[error("critic gate denied ({reason:?}): {}", diagnostics.join("; "))]
    CriticGateDenied {
        reason: GateReason,
        diagnostics: Vec<String>,
    },
    #[error("negative-result quota unmet: required {required}, observed {observed}")]
    NegativeResultQuotaUnmet { required: u32, observed: u32 },
    #[error("novelty gate denied emission: {reason}")]
    NoveltyDenied { reason: String },
    #[error("scholarly trust gate denied emission: {reason}")]
    ScholarlyTrustDenied { reason: String },
}

/// Pure 4-layer gate enforced before external scholarly or nanopub emission.
pub struct ExternalEmissionGate;

impl ExternalEmissionGate {
    /// Verify all 4 emission layers in strict fail-closed order.
    pub fn verify(input: &EmissionGateInput<'_>) -> Result<EmissionClearance, EmissionGateError> {
        // Layer 1: SHA3-256 content-digest-bound ApprovalToken.
        if input.token.publication_id() != input.publication_id {
            return Err(EmissionGateError::ApprovalTokenPublicationMismatch {
                expected: input.publication_id.to_string(),
                actual: input.token.publication_id().to_string(),
            });
        }
        if input.token.bound_digest() != input.content_sha3_256 {
            return Err(EmissionGateError::ApprovalTokenStaleDigest {
                bound: input.token.bound_digest().to_string(),
                current: input.content_sha3_256.to_string(),
            });
        }
        if let Some(payload) = input.canonical_payload {
            let computed = sha3_256_hex(payload);
            if computed != input.content_sha3_256 {
                return Err(EmissionGateError::PayloadSha3Mismatch {
                    expected: input.content_sha3_256.to_string(),
                    computed,
                });
            }
        }

        // Layer 2: CriticGate decision + negative-result quota.
        let critic_outcome = evaluate_gate(input.critic_inputs);
        if !critic_outcome.cleared {
            return Err(EmissionGateError::CriticGateDenied {
                reason: critic_outcome.reason,
                diagnostics: critic_outcome.diagnostics,
            });
        }
        if input.negative_result_quota > 0
            && !input.is_negative_result
            && input.null_results_in_window < input.negative_result_quota
        {
            return Err(EmissionGateError::NegativeResultQuotaUnmet {
                required: input.negative_result_quota,
                observed: input.null_results_in_window,
            });
        }

        // Layer 3: NoveltyVerdict clearance (Novel | PossiblyNovel).
        let novelty_label = match input.novelty_verdict {
            NoveltyVerdict::Novel => "novel".to_string(),
            NoveltyVerdict::PossiblyNovel { .. } => "possibly_novel".to_string(),
            NoveltyVerdict::NotNovel {
                closest_hit_uri,
                similarity,
            } => {
                return Err(EmissionGateError::NoveltyDenied {
                    reason: format!("not_novel (similarity {similarity:.2} vs {closest_hit_uri})"),
                });
            }
            NoveltyVerdict::InsufficientEvidence { reason } => {
                return Err(EmissionGateError::NoveltyDenied {
                    reason: format!("insufficient_evidence ({reason})"),
                });
            }
            NoveltyVerdict::Contradicted { conflicting_uri } => {
                return Err(EmissionGateError::NoveltyDenied {
                    reason: format!("contradicted by {conflicting_uri}"),
                });
            }
        };

        // Layer 4: Scholarly trust / OfflineAttested fallback clearance.
        let trust_mode = match input.scholarly_trust {
            ScholarlyTrustMode::VerifiedOnline {
                trust_score,
                min_trust_score,
            } => {
                if !trust_score.is_finite() || *trust_score < *min_trust_score {
                    return Err(EmissionGateError::ScholarlyTrustDenied {
                        reason: format!(
                            "online trust_score {trust_score:.3} below floor {min_trust_score:.3}"
                        ),
                    });
                }
                "verified_online".to_string()
            }
            ScholarlyTrustMode::OfflineAttested {
                attestation_digest,
                signer_orcid,
            } => {
                if attestation_digest.trim().is_empty() {
                    return Err(EmissionGateError::ScholarlyTrustDenied {
                        reason: "offline attestation_digest is empty".to_string(),
                    });
                }
                if signer_orcid.as_deref().is_none_or(|o| o.trim().is_empty()) {
                    return Err(EmissionGateError::ScholarlyTrustDenied {
                        reason: "offline attestation requires a non-empty signer ORCID".to_string(),
                    });
                }
                "offline_attested".to_string()
            }
            ScholarlyTrustMode::Unverified => {
                return Err(EmissionGateError::ScholarlyTrustDenied {
                    reason: "unverified scholarly trust state".to_string(),
                });
            }
        };

        Ok(EmissionClearance {
            publication_id: input.publication_id.to_string(),
            content_sha3_256: input.content_sha3_256.to_string(),
            critic_reason: critic_outcome.reason,
            novelty_label,
            trust_mode,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::critic_gate::{ApproverRecord, ApproverRole, VenueCriticPolicy};
    use crate::review::{ReviewDecisionLike, mint_from_decision, sealed};

    struct TestDecision {
        claim_id: i64,
        publication_id: String,
        bound_digest: String,
        decision: String,
    }
    impl sealed::Sealed for TestDecision {}
    impl ReviewDecisionLike for TestDecision {
        fn claim_id(&self) -> i64 {
            self.claim_id
        }
        fn publication_id(&self) -> &str {
            &self.publication_id
        }
        fn bound_digest(&self) -> &str {
            &self.bound_digest
        }
        fn decision(&self) -> &str {
            &self.decision
        }
    }

    fn minted_token(pub_id: &str, payload: &[u8]) -> (ApprovalToken, String) {
        let digest = sha3_256_hex(payload);
        let d = TestDecision {
            claim_id: 1,
            publication_id: pub_id.to_string(),
            bound_digest: digest.clone(),
            decision: "approved".to_string(),
        };
        (mint_from_decision(&d).expect("mint"), digest)
    }

    fn two_humans() -> Vec<ApproverRecord> {
        vec![
            ApproverRecord {
                approver_id: "0000-0001".into(),
                role: ApproverRole::Human,
                critic_fingerprint: None,
                critic_recommendation: None,
            },
            ApproverRecord {
                approver_id: "0000-0002".into(),
                role: ApproverRole::Human,
                critic_fingerprint: None,
                critic_recommendation: None,
            },
        ]
    }

    #[test]
    fn passes_all_four_layers_with_offline_attested_fallback() {
        let payload = b"canonical publication payload";
        let (token, digest) = minted_token("pub-1", payload);
        let approvers = two_humans();
        let critic_inputs = GateInputs {
            approvers: &approvers,
            artifact_model_fingerprints: &[],
            venue_policy: VenueCriticPolicy::Forbidden,
        };
        let trust = ScholarlyTrustMode::OfflineAttested {
            attestation_digest: digest.clone(),
            signer_orcid: Some("https://orcid.org/0000-0002-1825-0097".into()),
        };
        let clearance = ExternalEmissionGate::verify(&EmissionGateInput {
            publication_id: "pub-1",
            token: &token,
            content_sha3_256: &digest,
            canonical_payload: Some(payload),
            critic_inputs: &critic_inputs,
            negative_result_quota: 1,
            null_results_in_window: 1,
            is_negative_result: false,
            novelty_verdict: &NoveltyVerdict::PossiblyNovel {
                closest_score: 0.62,
            },
            scholarly_trust: &trust,
        })
        .expect("4-layer clearance");
        assert_eq!(clearance.novelty_label, "possibly_novel");
        assert_eq!(clearance.trust_mode, "offline_attested");
    }

    #[test]
    fn rejects_stale_digest_quota_shortfall_not_novel_and_unverified_trust() {
        let payload = b"v1";
        let (token, digest) = minted_token("pub-2", payload);
        let approvers = two_humans();
        let critic_inputs = GateInputs {
            approvers: &approvers,
            artifact_model_fingerprints: &[],
            venue_policy: VenueCriticPolicy::Allowed,
        };
        let trust_ok = ScholarlyTrustMode::VerifiedOnline {
            trust_score: 0.9,
            min_trust_score: 0.7,
        };

        let stale = ExternalEmissionGate::verify(&EmissionGateInput {
            publication_id: "pub-2",
            token: &token,
            content_sha3_256: "stale-other",
            canonical_payload: None,
            critic_inputs: &critic_inputs,
            negative_result_quota: 0,
            null_results_in_window: 0,
            is_negative_result: false,
            novelty_verdict: &NoveltyVerdict::Novel,
            scholarly_trust: &trust_ok,
        })
        .unwrap_err();
        assert!(matches!(
            stale,
            EmissionGateError::ApprovalTokenStaleDigest { .. }
        ));

        let quota = ExternalEmissionGate::verify(&EmissionGateInput {
            publication_id: "pub-2",
            token: &token,
            content_sha3_256: &digest,
            canonical_payload: Some(payload),
            critic_inputs: &critic_inputs,
            negative_result_quota: 2,
            null_results_in_window: 1,
            is_negative_result: false,
            novelty_verdict: &NoveltyVerdict::Novel,
            scholarly_trust: &trust_ok,
        })
        .unwrap_err();
        assert!(matches!(
            quota,
            EmissionGateError::NegativeResultQuotaUnmet { .. }
        ));

        let not_novel = ExternalEmissionGate::verify(&EmissionGateInput {
            publication_id: "pub-2",
            token: &token,
            content_sha3_256: &digest,
            canonical_payload: Some(payload),
            critic_inputs: &critic_inputs,
            negative_result_quota: 0,
            null_results_in_window: 0,
            is_negative_result: false,
            novelty_verdict: &NoveltyVerdict::NotNovel {
                closest_hit_uri: "doi:10.1/x".into(),
                similarity: 0.91,
            },
            scholarly_trust: &trust_ok,
        })
        .unwrap_err();
        assert!(matches!(not_novel, EmissionGateError::NoveltyDenied { .. }));

        let untrusted = ExternalEmissionGate::verify(&EmissionGateInput {
            publication_id: "pub-2",
            token: &token,
            content_sha3_256: &digest,
            canonical_payload: Some(payload),
            critic_inputs: &critic_inputs,
            negative_result_quota: 0,
            null_results_in_window: 0,
            is_negative_result: false,
            novelty_verdict: &NoveltyVerdict::Novel,
            scholarly_trust: &ScholarlyTrustMode::Unverified,
        })
        .unwrap_err();
        assert!(matches!(
            untrusted,
            EmissionGateError::ScholarlyTrustDenied { .. }
        ));
    }
}
