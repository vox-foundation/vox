use crate::types::VerifierOutput;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub enum MiniCheckBackend {
    Mock,
    Http { endpoint: String },
}

pub struct MiniCheckVerifier {
    pub backend: MiniCheckBackend,
    pub abstain_threshold: f64,
}

#[allow(dead_code)]
#[derive(Serialize)]
struct MiniCheckRequest<'a> {
    claim: &'a str,
    context: &'a str,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct MiniCheckResponse {
    support_score: f64,
}

impl MiniCheckVerifier {
    pub fn mock() -> Self {
        Self {
            backend: MiniCheckBackend::Mock,
            abstain_threshold: 0.3,
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

    pub fn from_env() -> Self {
        if let Ok(url) = std::env::var("VOX_MINICHECK_ENDPOINT") {
            Self::http(url)
        } else {
            Self::mock()
        }
    }

    pub async fn verify_claim(
        &self,
        claim: &str,
        context: &str,
    ) -> Result<VerifierOutput, Box<dyn std::error::Error + Send + Sync>> {
        let claim_id = fnv1a_hash(claim);
        match &self.backend {
            MiniCheckBackend::Mock => {
                let claim_words: Vec<&str> = claim.split_whitespace().collect();
                let overlap = claim_words
                    .iter()
                    .filter(|w| {
                        context
                            .to_ascii_lowercase()
                            .contains(&w.to_ascii_lowercase())
                    })
                    .count();
                let score = if claim_words.is_empty() {
                    0.5
                } else {
                    0.5 + 0.5 * (overlap as f64 / claim_words.len() as f64)
                };
                Ok(VerifierOutput {
                    claim_id,
                    support_score: score,
                    abstained: score < self.abstain_threshold,
                    verifier_model: "mock".to_string(),
                })
            }
            MiniCheckBackend::Http { endpoint } => {
                // Phase 5: replace with actual reqwest call
                let _ = endpoint;
                Err("HTTP MiniCheck backend not yet wired (Phase 5)".into())
            }
        }
    }
}

fn fnv1a_hash(s: &str) -> u64 {
    const FNV_OFFSET: u64 = 14695981039346656037;
    const FNV_PRIME: u64 = 1099511628211;
    s.bytes()
        .fold(FNV_OFFSET, |h, b| (h ^ b as u64).wrapping_mul(FNV_PRIME))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_verifier_returns_result() {
        let verifier = MiniCheckVerifier::mock();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt
            .block_on(verifier.verify_claim(
                "latency increased",
                "The provider's p95 latency rose by 12ms in April 2026.",
            ))
            .unwrap();
        assert!(result.support_score >= 0.0 && result.support_score <= 1.0);
    }
}
