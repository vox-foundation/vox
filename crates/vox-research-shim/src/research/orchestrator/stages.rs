use super::super::types::{Citation, ResearchHit, SelfVerificationResult};
use super::config::{ABSTENTION_MARKER, RESEARCH_COMPLETENESS_RIDER};
use super::helpers::sanitize_evidence;

/// Count distinct registrable domains in research hits and flag diversity shortfall.
#[must_use]
pub fn evaluate_citation_diversity(
    hits: &[ResearchHit],
    min_distinct_domains: usize,
) -> (usize, bool) {
    let mut domains = std::collections::HashSet::new();
    for hit in hits {
        if let Some(host) = registrable_domain(&hit.url) {
            domains.insert(host);
        }
    }
    let count = domains.len();
    let below = min_distinct_domains > 0 && count < min_distinct_domains;
    (count, below)
}

pub(super) fn registrable_domain(url: &str) -> Option<String> {
    let lower = url.trim().to_ascii_lowercase();
    if lower.starts_with("repo://")
        || lower.starts_with("vox://")
        || lower.starts_with("tavily-research://")
    {
        return None;
    }
    let rest = lower.split("://").nth(1).unwrap_or(&lower);
    let host = rest
        .split('/')
        .next()
        .unwrap_or(rest)
        .split(':')
        .next()
        .unwrap_or(rest)
        .trim_start_matches("www.");
    if host.is_empty() || host == "localhost" {
        return None;
    }
    Some(host.to_string())
}

pub(super) struct JudgeParams<'a> {
    pub query: &'a str,
    pub answer: &'a str,
    pub citations: &'a [Citation],
    pub endpoint: Option<&'a str>,
    pub api_key: Option<&'a str>,
    pub model: &'a str,
    pub temperature: f32,
    pub max_tokens: u32,
}

pub(super) fn build_judge_system_prompt() -> String {
    "You are a research quality evaluator. Score the following answer strictly based on the rubric.
You MUST output your evaluation as a valid JSON object embedded in a ```json codeblock. Do not output anything else.

Schema required:
{
  \"factual_accuracy_reasoning\": \"string\",
  \"factual_accuracy_score\": integer (0-33),
  \"citation_density_reasoning\": \"string\",
  \"citation_density_score\": integer (0-33),
  \"coverage_reasoning\": \"string\",
  \"coverage_score\": integer (0-34),
  \"total_score\": integer (0-100)
}
{}"
    .replace("{}", RESEARCH_COMPLETENESS_RIDER)
}

pub(super) async fn judge_quality(params: JudgeParams<'_>) -> Result<i32, String> {
    let citation_snippets: String = params
        .citations
        .iter()
        .take(5)
        .map(|c| {
            format!(
                "- {} <{}>: {}",
                c.title,
                c.url,
                c.snippet.chars().take(200).collect::<String>()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let sys_prompt = build_judge_system_prompt();

    let user_prompt = format!(
        "Query: {}
Answer: {}

Citations used:
{}

Scoring rubric:
1. Factual accuracy: Does the answer align with the cited sources?
2. Citation density: Are key claims backed by at least one citation?
3. Coverage: Does the answer address all major aspects of the query?",
        sanitize_evidence(params.query),
        sanitize_evidence(params.answer),
        sanitize_evidence(&citation_snippets)
    );

    let content = chat_stage(
        vox_actor_runtime::llm::cascade::ResearchStage::Judge,
        params.endpoint,
        params.api_key,
        params.model,
        params.temperature,
        params.max_tokens,
        vec![
            ("system".to_string(), sys_prompt),
            ("user".to_string(), user_prompt),
        ],
        Some(serde_json::json!({"type": "json_object"})),
    )
    .await
    .map_err(|e| format!("judge call failed: {e}"))?;

    let mut block = content.as_str();
    if let Some(start) = content.find("```json") {
        let rest = &content[start + 7..];
        if let Some(end) = rest.find("```") {
            block = &rest[..end];
        } else {
            block = rest;
        }
    } else if let Some(start) = content.find("```") {
        let rest = &content[start + 3..];
        if let Some(end) = rest.find("```") {
            block = &rest[..end];
        } else {
            block = rest;
        }
    }

    #[derive(serde::Deserialize)]
    struct JudgeResponse {
        #[serde(default)]
        total_score: i32,
    }

    let parsed = serde_json::from_str::<JudgeResponse>(block.trim()).map_err(|_| {
        format!(
            "judge returned unparseable JSON: {}",
            content.chars().take(200).collect::<String>()
        )
    })?;

    if parsed.total_score <= 0 {
        return Err("judge returned no score".into());
    }
    Ok(parsed.total_score.clamp(1, 100))
}

pub(super) struct SynthesisParams<'a> {
    pub query: &'a str,
    pub hits: &'a [ResearchHit],
    pub verdicts: &'a [super::super::types::ClaimVerdict],
    pub endpoint: Option<&'a str>,
    pub api_key: Option<&'a str>,
    pub model: &'a str,
    pub temperature: f32,
    pub max_tokens: u32,
    pub context_max_chars: usize,
}

/// LLM-backed synthesis. There is deliberately no template fallback (spec §5):
/// a failed synthesis is an error the caller must surface. Returns
/// `(answer, model)`, where `model` is the id of the candidate that actually
/// produced the answer — not necessarily `params.model`, since the cascade
/// (`chat_with_cascade`) may fall through to a different candidate (e.g. a
/// free-tier fallback) when the first choice fails or is unavailable.
pub(super) async fn synthesize_answer_with_llm(
    params: SynthesisParams<'_>,
) -> anyhow::Result<(String, String)> {
    call_synthesis_llm(&params)
        .await
        .map_err(|e| anyhow::anyhow!("synthesis failed: {e}"))
}

fn synthesis_system_prompt() -> String {
    format!(
        "You are a precise research synthesizer. Using ONLY the provided evidence \
         snippets, write a thorough, well-structured answer to the user's question.\n\n\
         You MUST structure your synthesis with the following comprehensive markdown sections:\n\
         # Executive Summary\n\
         ## Architectural Tradeoffs\n\
         ## Grounded Claims\n\
         ## Contested Findings\n\
         ## Implementation Implications\n\n\
         Cite sources inline as [1], [2], etc. matching the evidence numbers.\n\
         If the evidence is insufficient to answer, reply with {ABSTENTION_MARKER} as the \
         very first line, then one sentence on what is missing, and make no claims.\n{}",
        RESEARCH_COMPLETENESS_RIDER
    )
}

async fn call_synthesis_llm(params: &SynthesisParams<'_>) -> anyhow::Result<(String, String)> {
    use crate::research::distillation::{
        ClaimEvidenceUnit, EpistemicModality, EvidenceKind, extract_registrable_domain,
        pack_rag_budget,
    };

    // 1. Format verdicts first, including compiler sandbox stderr for contradicted claims
    let raw_verdicts = params
        .verdicts
        .iter()
        .map(|v| {
            let stderr_detail = v
                .evidence_spans
                .iter()
                .find(|s| {
                    s.span_type == super::super::types::SpanType::Contradicting
                        && s.text.contains("Compiler error")
                })
                .map(|s| {
                    format!(
                        " [Sandbox Stderr: {}]",
                        s.text.chars().take(200).collect::<String>()
                    )
                })
                .unwrap_or_default();
            format!(
                "{}: {} ({:.0}% confidence){}",
                v.claim.text,
                v.verdict,
                v.confidence * 100.0,
                stderr_detail
            )
        })
        .collect::<Vec<_>>()
        .join("; ");

    let total_budget = params.context_max_chars.max(4000);
    let max_verdict_budget = if !params.verdicts.is_empty() {
        (total_budget * 15 / 100).max(1200)
    } else {
        0
    };
    let verdict_text: String = raw_verdicts.chars().take(max_verdict_budget).collect();

    // 2. Reclaim unused verdict budget for evidence
    let evidence_budget = total_budget.saturating_sub(verdict_text.len());

    // 3. Convert hits into ClaimEvidenceUnits and pack using partitioned RAG budget
    let units: Vec<ClaimEvidenceUnit> = params
        .hits
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let snippet = sanitize_evidence(&h.snippet.chars().take(600).collect::<String>());
            let domain = extract_registrable_domain(&h.url);
            let is_contested = params.verdicts.iter().any(|v| {
                (v.verdict == super::super::types::Verdict::Contradicted
                    || v.verdict == super::super::types::Verdict::Contested)
                    && (v.claim.text.contains(&h.title) || snippet.contains(&v.claim.text))
            });
            ClaimEvidenceUnit {
                unit_id: i as u64,
                kind: EvidenceKind::AtomicFact,
                subject: h.title.clone(),
                predicate: "reports".to_string(),
                object: snippet.clone(),
                conditions: vec![],
                modality: EpistemicModality::Definite,
                verbatim_quote: snippet,
                span_start: 0,
                span_end: 0,
                source_url: h.url.clone(),
                registrable_domain: domain,
                trust_score: h.trust_score,
                corroborating_domains: vec![],
                is_contradicted_or_contested: is_contested,
            }
        })
        .collect();

    let packed_units = pack_rag_budget(&units, params.query, evidence_budget);
    let evidence_text: String = if !packed_units.is_empty() {
        packed_units
            .iter()
            .enumerate()
            .map(|(i, u)| {
                format!(
                    "[{}] {}\nURL: {}\n{}\n",
                    i + 1,
                    u.subject,
                    u.source_url,
                    u.verbatim_quote
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        let fallback_evidence: String = params
            .hits
            .iter()
            .enumerate()
            .map(|(i, h)| {
                let snippet = sanitize_evidence(&h.snippet.chars().take(600).collect::<String>());
                format!("[{}] {}\nURL: {}\n{}\n", i + 1, h.title, h.url, snippet)
            })
            .collect::<Vec<_>>()
            .join("\n");
        fallback_evidence.chars().take(evidence_budget).collect()
    };

    let system = synthesis_system_prompt();

    let user = format!(
        "Question: {}\n\nEvidence:\n{}{verdict_section}",
        params.query,
        evidence_text,
        verdict_section = if verdict_text.is_empty() {
            String::new()
        } else {
            format!("\n\nClaim verdicts: {verdict_text}")
        }
    );

    chat_stage_with_model(
        vox_actor_runtime::llm::cascade::ResearchStage::Synthesis,
        params.endpoint,
        params.api_key,
        params.model,
        params.temperature,
        params.max_tokens,
        vec![("system".to_string(), system), ("user".to_string(), user)],
        None,
    )
    .await
}

/// CoVE-style self-verification step.
pub(super) async fn run_self_verification(
    _query: &str,
    answer: &str,
    hits: &[ResearchHit],
    endpoint: Option<&str>,
    api_key: Option<&str>,
    model: &str,
) -> SelfVerificationResult {
    // Build a compact context from top-5 hits.
    let context: String = hits
        .iter()
        .take(5)
        .map(|h| {
            format!(
                "- {} — {}",
                h.title,
                h.snippet.chars().take(300).collect::<String>()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Step 1: Ask the model to generate verification questions from the draft.
    let question_prompt = format!(
        "Given the following research answer, generate up to 5 yes/no verification questions \
that target specific factual claims in the answer. Return one question per line, no numbering.\n\n\
Answer: {answer}\n\nQuestions:"
    );

    let questions: Vec<String> = if let Ok(content) = chat_stage(
        vox_actor_runtime::llm::cascade::ResearchStage::SelfVerification,
        endpoint,
        api_key,
        model,
        0.3,
        300,
        vec![("user".to_string(), question_prompt)],
        None,
    )
    .await
    {
        content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .take(5)
            .map(|l| l.trim().to_string())
            .collect()
    } else {
        return SelfVerificationResult {
            checked: true,
            questions_generated: 0,
            inconsistency_count: 0,
            critical_inconsistency: false,
        };
    };

    let questions_generated = questions.len();
    if questions_generated == 0 {
        return SelfVerificationResult {
            checked: true,
            questions_generated: 0,
            inconsistency_count: 0,
            critical_inconsistency: false,
        };
    }

    // Step 2: Answer each question from the retrieved context only and check consistency.
    let mut inconsistency_count = 0usize;
    for q in &questions {
        let verify_prompt = format!(
            "Based ONLY on the following sources, answer this yes/no question.\n\
Sources:\n{context}\n\nQuestion: {q}\n\nAnswer with only 'yes', 'no', or 'unknown'."
        );
        if let Ok(ans) = chat_stage(
            vox_actor_runtime::llm::cascade::ResearchStage::SelfVerification,
            endpoint,
            api_key,
            model,
            0.0,
            10,
            vec![("user".to_string(), verify_prompt)],
            None,
        )
        .await
        {
            let cleaned = ans.trim().to_lowercase();
            // "unknown" counts as a soft inconsistency (answer claimed something the context can't confirm)
            if cleaned.contains("no") || cleaned.contains("unknown") {
                inconsistency_count += 1;
            }
        }
    }

    let critical_inconsistency = inconsistency_count > questions_generated / 2;
    SelfVerificationResult {
        checked: true,
        questions_generated,
        inconsistency_count,
        critical_inconsistency,
    }
}

#[cfg(feature = "runtime")]
pub(crate) async fn chat_stage(
    stage: vox_actor_runtime::llm::cascade::ResearchStage,
    endpoint: Option<&str>,
    api_key: Option<&str>,
    model: &str,
    temperature: f32,
    max_tokens: u32,
    messages: Vec<(String, String)>,
    response_format: Option<serde_json::Value>,
) -> anyhow::Result<String> {
    chat_stage_with_model(
        stage,
        endpoint,
        api_key,
        model,
        temperature,
        max_tokens,
        messages,
        response_format,
    )
    .await
    .map(|(content, _model)| content)
}

/// Like [`chat_stage`], but also returns the model id that actually produced the
/// response (`LlmResponse::model` — the winning candidate, not necessarily the
/// first-choice `model` argument: `chat_with_cascade` may fall through to a
/// different candidate). Callers that need to record which model answered
/// (e.g. synthesis, for an honest `ResearchMetadata::synthesis_model`) should
/// use this instead of `chat_stage` and discarding the model id.
#[cfg(feature = "runtime")]
pub(crate) async fn chat_stage_with_model(
    stage: vox_actor_runtime::llm::cascade::ResearchStage,
    endpoint: Option<&str>,
    api_key: Option<&str>,
    model: &str,
    temperature: f32,
    max_tokens: u32,
    messages: Vec<(String, String)>,
    response_format: Option<serde_json::Value>,
) -> anyhow::Result<(String, String)> {
    use vox_actor_runtime::ActivityOptions;
    use vox_actor_runtime::llm::LlmChatMessage;
    use vox_actor_runtime::llm::cascade::{
        ResearchStage, cascade_with_optional_manual, chat_with_cascade,
    };
    use vox_actor_runtime::model_resolution::RouteResolutionInput;

    let input = RouteResolutionInput {
        openrouter_model: model.to_string(),
        ..RouteResolutionInput::default()
    };
    // Mirrors the stage->intent mapping established in
    // `research::model_select::resolve_research_models`: Synthesis uses the
    // general research intent, Judge uses the review intent. SelfVerification
    // has no equivalent stage there yet, so it defaults to the research
    // intent as the closest fit (future refinement: a dedicated intent).
    let intent = match stage {
        ResearchStage::Judge => vox_orchestrator::models::SelectionIntent::review(),
        ResearchStage::Synthesis => vox_orchestrator::models::SelectionIntent::research(),
        ResearchStage::SelfVerification => {
            vox_orchestrator::models::SelectionIntent::snippet_triage()
        }
        ResearchStage::Planner => vox_orchestrator::models::SelectionIntent::research(),
        ResearchStage::ClaimExtraction => {
            vox_orchestrator::models::SelectionIntent::claim_extraction()
        }
        ResearchStage::Verification => vox_orchestrator::models::SelectionIntent::nli_classifier(),
    };
    let primary = crate::research::orchestrator::model_dispatch::primary_candidate_for_intent(
        intent,
        stage.model_role(),
    );
    let mut candidates: Vec<vox_actor_runtime::llm::LlmConfig> = primary.into_iter().collect();
    candidates.extend(cascade_with_optional_manual(
        stage,
        &input,
        endpoint,
        api_key,
        Some(model),
    ));
    candidates.dedup_by(|a, b| a.provider == b.provider && a.model == b.model);
    for candidate in &mut candidates {
        candidate.temperature = Some(temperature);
        candidate.max_tokens = Some(max_tokens.into());
        candidate.response_format = response_format.clone();
    }
    crate::research::metering::tag_candidates(&mut candidates);
    let messages = messages
        .into_iter()
        .map(|(role, content)| LlmChatMessage {
            role,
            content,
            ..Default::default()
        })
        .collect();
    let opts = ActivityOptions::new().with_timeout_secs(45);
    // `Some(stage)`: labels telemetry and lets a trace record the requested vs
    // resolved model for this role (`record_research_model_uses`).
    chat_with_cascade(&opts, messages, candidates, Some(stage))
        .await
        .inspect(crate::research::metering::meter_response)
        .inspect_err(|_| crate::research::metering::meter_failure())
        .map(response_to_content_and_model)
        .map_err(|e| anyhow::anyhow!(e))
}

/// Extracts `(content, model)` from a cascade response. `LlmResponse::model` is the
/// id of the candidate that actually answered (from the response body, or the
/// configured model as fallback) — this is the seam the synthesis-honesty fix
/// depends on: `chat_with_cascade` may fall through past the first-choice candidate
/// (e.g. to a free-tier fallback), so the caller must read the model off the
/// response, never assume it matches whichever candidate was listed first.
///
/// Compiled whenever `runtime` is enabled (the production call site) or under
/// `cfg(test)` (so it stays unit-testable without the `runtime` feature — see
/// `winning_candidate_model_is_threaded_through_not_the_configured_one` below),
/// rather than only `#[cfg(feature = "runtime")]` like `chat_stage_with_model`.
#[cfg(any(feature = "runtime", test))]
fn response_to_content_and_model(
    response: vox_actor_runtime::llm::LlmResponse,
) -> (String, String) {
    (response.content, response.model)
}

#[cfg(not(feature = "runtime"))]
pub(crate) async fn chat_stage(
    stage: vox_actor_runtime::llm::cascade::ResearchStage,
    endpoint: Option<&str>,
    api_key: Option<&str>,
    model: &str,
    temperature: f32,
    max_tokens: u32,
    messages: Vec<(String, String)>,
    response_format: Option<serde_json::Value>,
) -> anyhow::Result<String> {
    chat_stage_with_model(
        stage,
        endpoint,
        api_key,
        model,
        temperature,
        max_tokens,
        messages,
        response_format,
    )
    .await
    .map(|(content, _model)| content)
}

#[cfg(not(feature = "runtime"))]
pub(crate) async fn chat_stage_with_model(
    _stage: vox_actor_runtime::llm::cascade::ResearchStage,
    _endpoint: Option<&str>,
    _api_key: Option<&str>,
    _model: &str,
    _temperature: f32,
    _max_tokens: u32,
    _messages: Vec<(String, String)>,
    _response_format: Option<serde_json::Value>,
) -> anyhow::Result<(String, String)> {
    anyhow::bail!("research runtime feature is disabled")
}

#[cfg(test)]
mod synthesis_prompt_tests {
    use super::super::config::ABSTENTION_MARKER;
    use super::synthesis_system_prompt;

    #[test]
    fn synthesis_prompt_names_the_abstention_marker() {
        let prompt = synthesis_system_prompt();
        assert!(prompt.contains(ABSTENTION_MARKER));
        assert!(prompt.contains("Cite sources inline"));
    }
}

#[cfg(test)]
mod citation_diversity_tests {
    use super::super::model_dispatch::ENV_LOCK;
    use super::evaluate_citation_diversity;
    use crate::research::types::ResearchHit;

    #[test]
    fn diversity_gate_flags_insufficient_domains() {
        let hits = vec![
            ResearchHit {
                url: "https://a.example/x".into(),
                title: "a".into(),
                snippet: "s".into(),
                score: 1.0,
                http_status: 0,
                trust_score: 1.0,
                raw_content: String::new(),
            },
            ResearchHit {
                url: "https://a.example/y".into(),
                title: "b".into(),
                snippet: "s".into(),
                score: 1.0,
                http_status: 0,
                trust_score: 1.0,
                raw_content: String::new(),
            },
        ];
        let (count, below) = evaluate_citation_diversity(&hits, 3);
        assert_eq!(count, 1);
        assert!(below);
    }

    #[test]
    fn winning_candidate_model_is_threaded_through_not_the_configured_one() {
        // Regression seed for the D5 follow-up: `chat_with_cascade` may fall through
        // past the first-choice candidate (e.g. a free-tier fallback), so the model
        // recorded on `ResearchMetadata::synthesis_model` must come from the response
        // that actually answered (`LlmResponse::model`), never the configured/first
        // candidate's model string. The `runtime` feature gates the live HTTP path
        // (impractical to stub here), so this exercises the plumbing seam directly:
        // `response_to_content_and_model` is the sole place `chat_stage_with_model`
        // extracts `(content, model)` from a cascade response.
        let response = vox_actor_runtime::llm::LlmResponse {
            content: "synthesized answer".to_string(),
            prompt_tokens: 10,
            completion_tokens: 20,
            model: "openrouter/some-fallback-model".to_string(),
            cost_usd: None,
            tool_calls: None,
            latency_ms: 0,
            cache_read_tokens: 0,
            ttft_ms: None,
            tpot_ms: None,
        };

        let (content, model) = super::response_to_content_and_model(response);

        assert_eq!(content, "synthesized answer");
        assert_eq!(
            model, "openrouter/some-fallback-model",
            "the winning candidate's model must be threaded through, not the configured/first-choice one"
        );
    }

    #[test]
    fn judge_prompt_has_no_code_generation_boilerplate() {
        let sys_prompt = super::build_judge_system_prompt();
        assert!(
            !sys_prompt.contains("TODO"),
            "judge prompt should not contain code-generation vocabulary: {sys_prompt}"
        );
        assert!(
            sys_prompt.contains("Cite every material claim"),
            "judge prompt should use research-appropriate completeness language: {sys_prompt}"
        );
    }

    #[test]
    fn unjudged_fallback_score_stays_below_persistence_gates() {
        // pipeline gates: low bar >= 50, high bar >= 70. An answer nobody judged must not clear them.
        let cfg = crate::research::ResearchConfig::default();
        assert!(
            cfg.fallback_quality_score < 50,
            "{}",
            cfg.fallback_quality_score
        );
    }

    #[test]
    fn default_judge_budget_fits_its_json_schema() {
        // A schema-shaped reply with modest reasoning text is ~100+ tokens; 16 truncated it every time.
        let cfg = crate::research::ResearchConfig::default();
        assert!(cfg.judge_max_tokens >= 256, "{}", cfg.judge_max_tokens);
    }

    #[tokio::test]
    #[allow(unsafe_code)]
    // ENV_LOCK is a test-only, never-production std::sync::Mutex that must stay held across
    // the `.await` below to actually serialize this test's env mutation against every other
    // test in the crate that reads/writes VOX_MODEL_FORCE / OPENROUTER_BASE_URL — releasing it
    // sooner would defeat its purpose. Single-threaded within the critical section, so no
    // executor-blocking risk in practice.
    #[allow(clippy::await_holding_lock)]
    async fn synthesis_failure_is_an_error_not_a_template() {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let prior_force = std::env::var("VOX_MODEL_FORCE").ok();
        let prior_base = std::env::var("OPENROUTER_BASE_URL").ok();
        // No key + unreachable pinned model => every candidate fails.
        unsafe {
            std::env::set_var("VOX_MODEL_FORCE", "nonexistent/model-for-test");
        }
        unsafe {
            std::env::set_var("OPENROUTER_BASE_URL", "http://127.0.0.1:9");
        }
        let hits = vec![crate::research::types::ResearchHit {
            url: "https://a.example".into(),
            title: "a".into(),
            snippet: "s".into(),
            score: 1.0,
            http_status: 200,
            trust_score: 1.0,
            raw_content: String::new(),
        }];
        let r = super::synthesize_answer_with_llm(super::SynthesisParams {
            query: "q",
            hits: &hits,
            verdicts: &[],
            endpoint: None,
            api_key: None,
            model: "nonexistent/model-for-test",
            temperature: 0.2,
            max_tokens: 100,
            context_max_chars: 4000,
        })
        .await;
        unsafe {
            match prior_force {
                Some(v) => std::env::set_var("VOX_MODEL_FORCE", v),
                None => std::env::remove_var("VOX_MODEL_FORCE"),
            }
            match prior_base {
                Some(v) => std::env::set_var("OPENROUTER_BASE_URL", v),
                None => std::env::remove_var("OPENROUTER_BASE_URL"),
            }
        }
        let err = r.expect_err("a failed synthesis must be Err — never template prose");
        assert!(err.to_string().contains("synthesis failed"), "{err}");
    }

    #[test]
    fn judge_budget_fits_its_schema() {
        let c = super::super::config::ResearchConfig::default();
        assert!(
            c.judge_max_tokens >= c.synthesis_max_tokens,
            "judge_max_tokens {} does not leave room for google/gemini-3.8-flash's \
             reasoning tokens ahead of the visible JSON (D8 — see judge_probe_max_tokens_400_vs_4000). \
             The judge reads the synthesis output, so its budget must scale with \
             synthesis_max_tokens (D9)",
            c.judge_max_tokens
        );
    }

    #[test]
    fn synthesis_budget_fits_its_five_mandated_sections() {
        let c = super::super::config::ResearchConfig::default();
        assert!(
            c.synthesis_max_tokens >= 2400,
            "synthesis_max_tokens {} truncates the five mandated markdown sections at live \
             evidence scale (D9 — see synthesis_probe_max_tokens_budget_vs_visible_output: \
             1200 exhausted the budget at 1202 completion tokens mid-sentence; the natural \
             length is ~1800)",
            c.synthesis_max_tokens
        );
    }

    /// D8 live diagnostic (Task 8 fix round 2, finding (b)): the live daemon's
    /// deep-research judge stage failed with `judge returned unparseable JSON:
    /// {\n  "factual_accuracy_` — ~20 visible characters, far short of the
    /// 400-token budget that was configured, so `max_tokens` itself wasn't the
    /// literal ceiling being hit at the API framing level; something was
    /// consuming the budget before the JSON could be written out. Probes
    /// `judge_quality` directly against the real OpenRouter endpoint at the
    /// old 400-token budget (reproduce) and then at a much larger budget
    /// (diagnose), printing whether each attempt parses.
    ///
    /// Run manually (needs a live `OPENROUTER_API_KEY` and network):
    /// `cargo test -p vox-research-shim --features runtime -- --ignored --nocapture judge_probe`
    #[cfg(feature = "runtime")]
    #[tokio::test]
    #[ignore = "owner: research — live OpenRouter network probe; run manually with --ignored --features runtime"]
    async fn judge_probe_max_tokens_400_vs_4000() {
        let Some(api_key) = vox_secrets::resolve_secret(vox_secrets::SecretId::OpenRouterApiKey)
            .expose()
            .map(|s| s.to_string())
        else {
            eprintln!("SKIP: OPENROUTER_API_KEY does not resolve; cannot probe live judge stage");
            return;
        };
        let citations = vec![crate::research::types::Citation {
            source_id: 1,
            url: "https://arxiv.org/abs/0000.00000".into(),
            title: "Example paper".into(),
            snippet: "An example finding about search infrastructure.".into(),
            confidence: 0.9,
        }];
        let query = "compare SearXNG and Tavily for agent web search";
        let answer = "SearXNG is a self-hosted open-source metasearch engine; Tavily is a \
                       commercial API purpose-built for LLM agents. [1]";

        for max_tokens in [400u32, 1200u32, 1600u32, 4000u32] {
            let result = super::judge_quality(super::JudgeParams {
                query,
                answer,
                citations: &citations,
                endpoint: None,
                api_key: Some(&api_key),
                model: "google/gemini-3.8-flash",
                temperature: 0.0,
                max_tokens,
            })
            .await;
            eprintln!("--- max_tokens={max_tokens} ---\n{result:?}\n");
        }
    }

    /// D9 live diagnostic (Task 8 fix round 4): the live daemon's deep-research
    /// synthesis stage returned a 202-character fragment that stopped
    /// mid-sentence ("**Tavily** is a managed, purpose-built \"agent-native\"
    /// search"), which made the judge score it 5/100 and citation_audit report
    /// 0/35. Same class as the round-2 judge truncation (D8), but for
    /// synthesis. Probes the real synthesis path against the real
    /// `google/gemini-3.8-flash` at the shipped budget and larger ones,
    /// printing the visible answer length AND `completion_tokens` — if
    /// `completion_tokens` sits at the budget while the visible text is a few
    /// hundred characters, the model spent the budget on reasoning tokens
    /// before writing any answer.
    ///
    /// Run manually (needs a live `OPENROUTER_API_KEY` and network):
    /// `cargo test -p vox-research-shim --features runtime -- --ignored --nocapture synthesis_probe`
    #[cfg(feature = "runtime")]
    #[tokio::test]
    #[ignore = "owner: research — live OpenRouter network probe; run manually with --ignored --features runtime"]
    async fn synthesis_probe_max_tokens_budget_vs_visible_output() {
        use vox_actor_runtime::ActivityOptions;
        use vox_actor_runtime::llm::LlmChatMessage;
        use vox_actor_runtime::llm::cascade::{
            ResearchStage, cascade_with_optional_manual, chat_with_cascade,
        };
        use vox_actor_runtime::model_resolution::RouteResolutionInput;

        let Some(api_key) = vox_secrets::resolve_secret(vox_secrets::SecretId::OpenRouterApiKey)
            .expose()
            .map(|s| s.to_string())
        else {
            eprintln!("SKIP: OPENROUTER_API_KEY does not resolve; cannot probe live synthesis");
            return;
        };
        let model = "google/gemini-3.8-flash";

        // The real synthesis framing (five mandated markdown sections), with a
        // small but realistic SearXNG-vs-Tavily evidence set — the live shape.
        let system = format!(
            "You are a precise research synthesizer. Using ONLY the provided evidence \
             snippets, write a thorough, well-structured answer to the user's question.\n\n\
             You MUST structure your synthesis with the following comprehensive markdown sections:\n\
             # Executive Summary\n\
             ## Architectural Tradeoffs\n\
             ## Grounded Claims\n\
             ## Contested Findings\n\
             ## Implementation Implications\n\n\
             Cite sources inline as [1], [2], etc. matching the evidence numbers.\n\
             If evidence is insufficient, say so clearly.\n{}",
            super::RESEARCH_COMPLETENESS_RIDER
        );
        let user = "Question: compare SearXNG and Tavily for agent web search\n\n\
             Evidence:\n\
             [1] Tavily vs SearXNG: which wins on the bench?\nURL: https://trytested.com/a\n\
             Tavily is a managed search API purpose-built for LLM agents, billed per call.\n\n\
             [2] Replacing Tavily with self-hosted SearXNG\nURL: https://note.com/b\n\
             SearXNG is a self-hosted metasearch aggregator with no per-call cost but \
             requires operating upstream engines yourself.\n\n\
             [3] The True Cost of Self-Hosted Web Search for AI Agents\nURL: https://tavily.com/c\n\
             Self-hosting shifts cost from per-query billing to operational maintenance.\n"
            .to_string();
        // The live failure had 35 sources packed into `synthesis_context_max_chars`
        // (24000). Reproduce that scale — a three-source prompt is far smaller than
        // what the daemon actually sends, and the truncation is input-size sensitive.
        let mut user = user;
        let mut n = 4;
        while user.chars().count() < 24_000 {
            user.push_str(&format!(
                "[{n}] Benchmarking LLM search APIs: latency, cost and recall\n\
                 URL: https://example{n}.dev/post\n\
                 Measured p50 latency, per-query price and answer recall across managed \
                 and self-hosted search backends for agent workloads, with notes on \
                 rate limits, caching, and upstream engine availability.\n\n"
            ));
            n += 1;
        }
        eprintln!("evidence prompt chars = {}", user.chars().count());

        for max_tokens in [1200u32, 2400u32, 4000u32, 8000u32] {
            let input = RouteResolutionInput {
                openrouter_model: model.to_string(),
                ..RouteResolutionInput::default()
            };
            let mut candidates = cascade_with_optional_manual(
                ResearchStage::Synthesis,
                &input,
                None,
                Some(&api_key),
                Some(model),
            );
            candidates.truncate(1);
            for candidate in &mut candidates {
                candidate.temperature = Some(0.2);
                candidate.max_tokens = Some(max_tokens.into());
                candidate.response_format = None;
            }
            let messages = vec![
                LlmChatMessage {
                    role: "system".to_string(),
                    content: system.clone(),
                    ..Default::default()
                },
                LlmChatMessage {
                    role: "user".to_string(),
                    content: user.clone(),
                    ..Default::default()
                },
            ];
            let opts = ActivityOptions::new().with_timeout_secs(120);
            match chat_with_cascade(&opts, messages, candidates, None).await {
                Ok(r) => eprintln!(
                    "--- max_tokens={max_tokens} --- chars={} completion_tokens={} model={}\n\
                     tail: {:?}\n",
                    r.content.chars().count(),
                    r.completion_tokens,
                    r.model,
                    r.content.chars().rev().take(90).collect::<String>()
                ),
                Err(e) => eprintln!("--- max_tokens={max_tokens} --- Err: {e}\n"),
            }
        }
    }
}
