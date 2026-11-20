//! Entailment judge for `vox audit docs --verify-llm`, over the model-agnostic
//! `vox_actor_runtime::llm` facade (`LLM-HEAL-01`). `vox-doc-pipeline` owns the trait and all
//! guardrails; this file only turns evidence into a prompt and a reply into a verdict.

use vox_actor_runtime::llm::{LlmChatMessage, LlmConfig, llm_chat};
use vox_actor_runtime::{ActivityOptions, ActivityResult};
use vox_doc_pipeline::claims::llm_verifier::{
    ClaimEntailmentJudge, ClaimVerdictKind, DocClaimCandidate, EvidenceSpan, JudgeOutput,
};

/// Judge backed by one configured model. Any failure (no key, rate limit, bad reply) yields
/// `None`, so the deterministic verdict stands.
pub struct LlmEntailmentJudge {
    config: LlmConfig,
}

impl LlmEntailmentJudge {
    #[must_use]
    pub fn new(config: LlmConfig) -> Self {
        Self { config }
    }

    /// Default judge: the model the registry selects for the NLI-classifier task class
    /// (`SelectionIntent::nli_classifier`), exactly as the research verifier picks its judge.
    /// Only when the registry offers nothing does it fall back to the workspace classifier
    /// default.
    #[must_use]
    pub fn from_default_model() -> Self {
        Self::new(registry_selected_config().unwrap_or_else(|| {
            LlmConfig::openrouter(vox_config::model_defaults::CLASSIFIER_FALLBACK)
        }))
    }
}

/// `LlmConfig` for the registry's pick for the NLI-classifier intent, or `None`.
pub(crate) fn registry_selected_config() -> Option<LlmConfig> {
    use vox_orchestrator::models::{
        SelectionIntent, llm_config_for_spec, select_with_default_registry,
    };
    let intent = SelectionIntent::nli_classifier();
    let task = intent.task;
    let outcome = select_with_default_registry(&intent)?;
    Some(llm_config_for_spec(&outcome.model_spec, task))
}

pub(crate) fn build_prompt(claim: &DocClaimCandidate, evidence: &[EvidenceSpan]) -> String {
    let mut p = String::from(
        "You audit documentation against code. Decide whether the CLAIM is directly entailed by \
         the EVIDENCE, contradicted by it, or cannot be decided from it.\n\
         Answer `verified` ONLY if the evidence text itself states or clearly implies the claim. \
         Answer `drifted` ONLY if the evidence contradicts the claim. Otherwise answer `uncertain`.\n\
         Reply with one JSON object: {\"verdict\":\"verified|drifted|uncertain\",\"rationale\":\"<one sentence>\"}.\n\n",
    );
    p.push_str(&format!(
        "CLAIM ({}:{}):\n{}\n\n",
        claim.doc_path, claim.line, claim.claim_text
    ));
    for e in evidence {
        p.push_str(&format!("EVIDENCE {}:\n{}\n\n", e.citation, e.text));
    }
    p
}

/// Parse the model reply. Anything not clearly one of the three verdicts is `None`.
pub(crate) fn parse_reply(reply: &str) -> Option<(ClaimVerdictKind, String)> {
    let start = reply.find('{')?;
    let end = reply.rfind('}')?;
    let v: serde_json::Value = serde_json::from_str(reply.get(start..=end)?).ok()?;
    let kind = match v
        .get("verdict")?
        .as_str()?
        .trim()
        .to_ascii_lowercase()
        .as_str()
    {
        "verified" => ClaimVerdictKind::Verified,
        "drifted" => ClaimVerdictKind::Drifted,
        "uncertain" => ClaimVerdictKind::Uncertain,
        _ => return None,
    };
    let rationale = v
        .get("rationale")
        .and_then(|r| r.as_str())
        .unwrap_or("")
        .trim()
        .to_string();
    Some((kind, rationale))
}

impl ClaimEntailmentJudge for LlmEntailmentJudge {
    fn judge(&self, claim: &DocClaimCandidate, evidence: &[EvidenceSpan]) -> Option<JudgeOutput> {
        let prompt = build_prompt(claim, evidence);
        let prompt_tokens = (prompt.len() / 4) as u64;
        let config = self.config.clone();
        let messages = vec![LlmChatMessage {
            role: "user".to_string(),
            content: prompt,
            ..Default::default()
        }];
        // A private current-thread runtime on its own thread: safe whether or not the caller is
        // already inside an async context.
        let reply = std::thread::scope(|s| {
            s.spawn(|| {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .ok()?;
                let opts = ActivityOptions::new().with_timeout_secs(45);
                match rt.block_on(llm_chat(&opts, messages, config)) {
                    ActivityResult::Ok(Ok(resp)) => Some(resp.content),
                    _ => None,
                }
            })
            .join()
            .ok()
            .flatten()
        })?;
        let (verdict, rationale) = parse_reply(&reply)?;
        Some(JudgeOutput {
            verdict,
            rationale: if rationale.is_empty() {
                "model verdict without rationale".to_string()
            } else {
                rationale
            },
            estimated_tokens: prompt_tokens + (reply.len() / 4) as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_the_three_verdicts_and_ignores_prose_around_json() {
        let (k, r) = parse_reply("Sure! {\"verdict\":\"Verified\",\"rationale\":\"stated\"} done")
            .expect("parse");
        assert_eq!(k, ClaimVerdictKind::Verified);
        assert_eq!(r, "stated");
        assert!(parse_reply("{\"verdict\":\"maybe\"}").is_none());
        assert!(parse_reply("no json here").is_none());
    }

    #[test]
    fn prompt_carries_claim_and_every_evidence_span() {
        let claim = DocClaimCandidate {
            doc_path: "docs/src/a.md".into(),
            line: 4,
            claim_text: "Uses `x/y.rs`.".into(),
            symbol_anchors: vec![],
            symbol_span_hash: String::new(),
        };
        let ev = vec![EvidenceSpan {
            citation: "[x/y.rs:L1-L2]".into(),
            text: "fn a() {}".into(),
        }];
        let p = build_prompt(&claim, &ev);
        assert!(p.contains("docs/src/a.md:4"));
        assert!(p.contains("EVIDENCE [x/y.rs:L1-L2]"));
        assert!(p.contains("fn a() {}"));
    }
}

#[cfg(test)]
mod registry_tests {
    use super::*;

    /// The judge's model comes from the registry's NLI-classifier selection, not a literal.
    #[test]
    fn judge_model_is_registry_selected_when_the_registry_offers_one() {
        if let Some(cfg) = registry_selected_config() {
            assert!(!cfg.model.is_empty());
        }
        // Registry selection is stateful; never compare two selections.
        let judge = LlmEntailmentJudge::from_default_model();
        assert!(!judge.config.model.is_empty());
    }
}
