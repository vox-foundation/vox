//! Research-oriented LLM cascade helpers.

use crate::model_resolution::{RouteResolutionInput, chat_route_to_llm_config};
use crate::{ActivityOptions, ActivityResult};

use super::{LlmChatMessage, LlmConfig, LlmResponse, infer_with_retry};
use vox_telemetry::{AiFixtureEvent, PromptDispatchTelemetryEvent, TelemetryEvent};

/// Research pipeline stage requesting an LLM call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResearchStage {
    Planner,
    ClaimExtraction,
    Verification,
    Synthesis,
    Judge,
    SelfVerification,
}

impl ResearchStage {
    /// The model role whose pin governs this stage (Task 13). Claim extraction
    /// and self-verification belong to verification.
    #[must_use]
    pub const fn model_role(self) -> vox_config::inference::ModelRole {
        use vox_config::inference::ModelRole;
        match self {
            Self::Planner => ModelRole::Planner,
            Self::Synthesis => ModelRole::Synthesis,
            Self::Judge => ModelRole::Judge,
            Self::ClaimExtraction | Self::Verification | Self::SelfVerification => {
                ModelRole::Verifier
            }
        }
    }
}

/// One successful research-stage LLM call: the id that was requested (the
/// candidate's `model` — a pin, possibly a `~vendor/…-latest` alias) and the
/// concrete id the provider reported answering (`LlmResponse::model`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResearchModelUse {
    pub stage: ResearchStage,
    pub requested: String,
    pub resolved: String,
}

tokio::task_local! {
    static RESEARCH_MODEL_USES: std::cell::RefCell<Vec<ResearchModelUse>>;
}

/// Run `f`, collecting every successful research-stage call made on this task
/// ([`chat_with_cascade`] / [`chat_with_cascade_parsed`] with a stage). Calls
/// made outside such a scope are simply not recorded.
pub async fn record_research_model_uses<F: std::future::Future>(
    f: F,
) -> (F::Output, Vec<ResearchModelUse>) {
    RESEARCH_MODEL_USES
        .scope(std::cell::RefCell::new(Vec::new()), async move {
            let out = f.await;
            let uses = RESEARCH_MODEL_USES.with(|u| u.take());
            (out, uses)
        })
        .await
}

fn note_research_model_use(stage: Option<ResearchStage>, cfg: &LlmConfig, resp: &LlmResponse) {
    if let Some(stage) = stage {
        let _ = RESEARCH_MODEL_USES.try_with(|u| {
            u.borrow_mut().push(ResearchModelUse {
                stage,
                requested: cfg.model.clone(),
                resolved: resp.model.clone(),
            });
        });
    }
}

fn research_stage_label(stage: Option<ResearchStage>) -> String {
    stage
        .map(|s| format!("{s:?}"))
        .unwrap_or_else(|| "unspecified".to_string())
}

/// Run chat completion over an explicit candidate cascade.
///
/// When `research_stage` is `Some`, emits [`TelemetryEvent::AiFixture`] prompt-dispatch telemetry.
pub async fn chat_with_cascade(
    opts: &ActivityOptions,
    messages: Vec<LlmChatMessage>,
    candidates: Vec<LlmConfig>,
    research_stage: Option<ResearchStage>,
) -> Result<LlmResponse, String> {
    if candidates.is_empty() {
        let stage_lbl = research_stage_label(research_stage);
        vox_telemetry::record_event!(&TelemetryEvent::AiFixture(AiFixtureEvent::PromptDispatch(
            PromptDispatchTelemetryEvent {
                stage: stage_lbl,
                outcome: "error".into(),
                error: Some("no LLM candidates available for research cascade".into()),
                redact_count: 0,
            }
        )));
        return Err("no LLM candidates available for research cascade".to_string());
    }

    let res = infer_with_retry(opts, messages, candidates).await;
    let stage_lbl = research_stage_label(research_stage);
    let (outcome, err) = match &res {
        ActivityResult::Ok(Ok(_)) => ("ok", None),
        ActivityResult::Ok(Err(e)) => ("error", Some(e.clone())),
        ActivityResult::Failed(e) => (
            "error",
            Some(format!("research cascade activity failed: {e:?}")),
        ),
        ActivityResult::Cancelled => ("cancelled", Some("research cascade cancelled".into())),
    };
    vox_telemetry::record_event!(&TelemetryEvent::AiFixture(AiFixtureEvent::PromptDispatch(
        PromptDispatchTelemetryEvent {
            stage: stage_lbl,
            outcome: outcome.into(),
            error: err,
            redact_count: 0,
        }
    )));

    match res {
        ActivityResult::Ok(Ok((response, cfg))) => {
            note_research_model_use(research_stage, &cfg, &response);
            Ok(response)
        }
        ActivityResult::Ok(Err(e)) => Err(e),
        ActivityResult::Failed(e) => Err(format!("research cascade activity failed: {e:?}")),
        ActivityResult::Cancelled => Err("research cascade cancelled".to_string()),
    }
}

/// Run chat completion over an explicit candidate cascade with schema parsing and fallback.
///
/// Iterates through `candidates`, executing each candidate in order with `infer_with_retry`.
/// If a candidate succeeds and `parser` successfully parses the content, returns `Ok((parsed, response))`.
/// If `parser` fails or the candidate execution fails, logs a warning/debug and falls back to the next candidate in the cascade.
pub async fn chat_with_cascade_parsed<T, F>(
    opts: &ActivityOptions,
    messages: Vec<LlmChatMessage>,
    candidates: Vec<LlmConfig>,
    research_stage: Option<ResearchStage>,
    parser: F,
) -> Result<(T, LlmResponse), String>
where
    F: Fn(&str) -> Result<T, String>,
{
    if candidates.is_empty() {
        let stage_lbl = research_stage_label(research_stage);
        vox_telemetry::record_event!(&TelemetryEvent::AiFixture(AiFixtureEvent::PromptDispatch(
            PromptDispatchTelemetryEvent {
                stage: stage_lbl,
                outcome: "error".into(),
                error: Some("no LLM candidates available for research cascade".into()),
                redact_count: 0,
            }
        )));
        return Err("no LLM candidates available for research cascade".to_string());
    }

    let mut last_err = String::from("no candidates succeeded");
    for candidate in candidates {
        let res = infer_with_retry(opts, messages.clone(), vec![candidate.clone()]).await;
        match res {
            ActivityResult::Ok(Ok((response, cfg))) => match parser(&response.content) {
                Ok(parsed) => {
                    note_research_model_use(research_stage, &cfg, &response);
                    let stage_lbl = research_stage_label(research_stage);
                    vox_telemetry::record_event!(&TelemetryEvent::AiFixture(
                        AiFixtureEvent::PromptDispatch(PromptDispatchTelemetryEvent {
                            stage: stage_lbl,
                            outcome: "ok".into(),
                            error: None,
                            redact_count: 0,
                        })
                    ));
                    return Ok((parsed, response));
                }
                Err(parse_err) => {
                    tracing::warn!(
                        candidate_model = %candidate.model,
                        error = %parse_err,
                        "cascade candidate response failed to parse; trying next candidate"
                    );
                    last_err = format!("parse error on model {}: {}", candidate.model, parse_err);
                }
            },
            ActivityResult::Ok(Err(e)) => {
                tracing::debug!(
                    candidate_model = %candidate.model,
                    error = %e,
                    "cascade candidate inference error; trying next candidate"
                );
                last_err = e;
            }
            ActivityResult::Failed(e) => {
                tracing::debug!(
                    candidate_model = %candidate.model,
                    error = ?e,
                    "cascade candidate activity failed; trying next candidate"
                );
                last_err = format!("activity failed: {e:?}");
            }
            ActivityResult::Cancelled => {
                let stage_lbl = research_stage_label(research_stage);
                vox_telemetry::record_event!(&TelemetryEvent::AiFixture(
                    AiFixtureEvent::PromptDispatch(PromptDispatchTelemetryEvent {
                        stage: stage_lbl,
                        outcome: "cancelled".into(),
                        error: Some("research cascade cancelled".into()),
                        redact_count: 0,
                    })
                ));
                return Err("research cascade cancelled".to_string());
            }
        }
    }

    let stage_lbl = research_stage_label(research_stage);
    vox_telemetry::record_event!(&TelemetryEvent::AiFixture(AiFixtureEvent::PromptDispatch(
        PromptDispatchTelemetryEvent {
            stage: stage_lbl,
            outcome: "error".into(),
            error: Some(last_err.clone()),
            redact_count: 0,
        }
    )));
    Err(format!(
        "all cascade candidates failed or failed to parse: {last_err}"
    ))
}

/// Ordered, dispatchable OpenRouter model ids for a research call.
///
/// Concrete `:free` slugs from [`vox_config::OPENROUTER_FREE_FALLBACK_MODELS`] are
/// ALWAYS appended as a zero-cost fallback floor so research degrades to free instead
/// of failing. The virtual `openrouter/free` route is intentionally NOT used here: it
/// is a registry-only id that the OpenRouter API rejects when dispatched raw, so real
/// `:free` model ids are used instead. `prefer_free` moves the free slugs ahead of the
/// caller-configured model; a configured model that is already a free slug is not
/// duplicated.
#[must_use]
fn research_openrouter_model_ids(configured: &str, prefer_free: bool) -> Vec<String> {
    let free: Vec<String> = vox_config::OPENROUTER_FREE_FALLBACK_MODELS
        .iter()
        .map(|s| (*s).to_string())
        .collect();
    let configured_is_free = free.iter().any(|f| f == configured);

    let mut ordered = Vec::with_capacity(free.len() + 1);
    if prefer_free {
        ordered.extend(free.iter().cloned());
        if !configured_is_free {
            ordered.push(configured.to_string());
        }
    } else {
        if !configured_is_free {
            ordered.push(configured.to_string());
        }
        ordered.extend(free.iter().cloned());
    }
    ordered
}

/// Build the default research cascade: local Mens/Ollama first, then OpenRouter.
#[must_use]
pub fn cascade_for_research_stage(
    stage: ResearchStage,
    input: &RouteResolutionInput,
) -> Vec<LlmConfig> {
    // Per-role pin (Task 13): the stage's role key, then VOX_MODEL_FORCE_RESEARCH,
    // then VOX_MODEL_FORCE.
    if let Some(forced) = vox_config::inference::forced_model_for(stage.model_role()) {
        return pinned_research_candidates(stage, &forced);
    }

    let mut candidates = Vec::new();

    if vox_config::inference::inference_profile_allows_local_ollama_http() {
        let base = vox_config::inference::local_ollama_populi_base_url();
        let mut local = chat_route_to_llm_config(
            &vox_orchestrator_types::ChatProviderRouteKind::PopuliLocal {
                base_url: base,
                model: input.mens_chat_model.clone(),
            },
        );
        apply_stage_defaults(stage, &mut local);
        candidates.push(local);
    }

    if vox_config::inference::openrouter_api_key().is_some() {
        let prefer_free = vox_config::inference::research_prefer_free_tier();
        for model_id in research_openrouter_model_ids(&input.openrouter_model, prefer_free) {
            let mut openrouter = LlmConfig::openrouter(model_id);
            apply_stage_defaults(stage, &mut openrouter);
            candidates.push(openrouter);
        }
    }

    candidates
}

/// Exactly one OpenRouter candidate for a pinned model: no local lane, no free floor.
#[must_use]
pub fn pinned_research_candidates(stage: ResearchStage, model: &str) -> Vec<LlmConfig> {
    let mut c = LlmConfig::openrouter(model.to_string());
    apply_stage_defaults(stage, &mut c);
    vec![c]
}

/// Add a manual OpenAI-compatible candidate before the default cascade.
#[must_use]
pub fn cascade_with_optional_manual(
    stage: ResearchStage,
    input: &RouteResolutionInput,
    endpoint: Option<&str>,
    api_key: Option<&str>,
    model: Option<&str>,
) -> Vec<LlmConfig> {
    let mut candidates = Vec::new();
    if let (Some(endpoint), Some(model)) = (endpoint, model) {
        let mut manual = LlmConfig {
            provider: "openai_compatible".to_string(),
            model: model.to_string(),
            cost_per_1k: None,
            base_url: Some(format!(
                "{}/v1/chat/completions",
                endpoint.trim_end_matches('/')
            )),
            api_key: api_key.map(str::to_string),
            temperature: None,
            top_p: None,
            max_tokens: None,
            response_format: None,
            tools: None,
            tool_choice: None,
            timeout_ms: Some(30_000),
            telemetry_session_id: None,
            telemetry_user_id: None,
            telemetry_task_category: Some("research".to_string()),
            telemetry_strength_tag: Some(format!("{stage:?}").to_ascii_lowercase()),
            telemetry_trace_id: None,
            telemetry_attempt_number: None,
            telemetry_skip_interaction: false,
        };
        apply_stage_defaults(stage, &mut manual);
        candidates.push(manual);
    }
    candidates.extend(cascade_for_research_stage(stage, input));
    candidates
}

fn apply_stage_defaults(stage: ResearchStage, cfg: &mut LlmConfig) {
    cfg.telemetry_task_category = Some("research".to_string());
    cfg.telemetry_strength_tag = Some(format!("{stage:?}").to_ascii_lowercase());
    cfg.temperature = Some(match stage {
        ResearchStage::Planner => 0.2,
        ResearchStage::ClaimExtraction | ResearchStage::Judge => 0.0,
        // Nonzero so SelfCheckGPT-style resampling (see
        // `verify_claims_with_config` in vox-research-shim) produces
        // genuine variation across samples instead of near-identical
        // deterministic output.
        ResearchStage::Verification => 0.3,
        ResearchStage::Synthesis => 0.2,
        ResearchStage::SelfVerification => 0.0,
    });
    // Synthesis max_tokens is NOT set here — controlled by ResearchConfig::synthesis_max_tokens.
    if stage != ResearchStage::Synthesis {
        cfg.max_tokens = Some(match stage {
            ResearchStage::Planner => 700,
            ResearchStage::ClaimExtraction => 900,
            ResearchStage::Verification => 500,
            // D8 (Task 8 fix round 2): kept in sync with
            // `ResearchConfig::judge_max_tokens` in vox-research-shim — 400 was
            // too tight for the judge's own JSON schema (3 free-text
            // `*_reasoning` fields plus 4 integer scores), live-probed truncating
            // every time against google/gemini-3.8-flash.
            ResearchStage::Judge => 4000,
            ResearchStage::SelfVerification => 700,
            ResearchStage::Synthesis => unreachable!("guarded by outer if"),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial(model_pin_env)]
    fn cascade_includes_local_candidate_when_profile_allows_it() {
        let candidates =
            cascade_for_research_stage(ResearchStage::Planner, &RouteResolutionInput::default());

        assert!(
            candidates
                .iter()
                .any(|candidate| candidate.provider == "ollama")
        );
    }

    #[test]
    fn pinned_research_candidates_is_exactly_the_pin() {
        let c = pinned_research_candidates(ResearchStage::Synthesis, "google/gemini-3.8-flash");
        assert_eq!(
            c.len(),
            1,
            "a pin must not append free-floor or local candidates"
        );
        assert_eq!(c[0].model, "google/gemini-3.8-flash");
        assert_eq!(c[0].provider, LlmConfig::openrouter("x").provider);
    }

    #[test]
    #[serial_test::serial(model_pin_env)]
    fn manual_candidate_is_first_when_endpoint_and_model_are_supplied() {
        let candidates = cascade_with_optional_manual(
            ResearchStage::Verification,
            &RouteResolutionInput::default(),
            Some("http://localhost:9999"),
            None,
            Some("local-test-model"),
        );

        assert_eq!(candidates[0].provider, "openai_compatible");
        assert_eq!(candidates[0].model, "local-test-model");
        assert_eq!(
            candidates[0].base_url.as_deref(),
            Some("http://localhost:9999/v1/chat/completions")
        );
    }

    #[test]
    #[serial_test::serial(model_pin_env)]
    fn synthesis_stage_does_not_force_1800_max_tokens() {
        use crate::model_resolution::RouteResolutionInput;
        let candidates = cascade_with_optional_manual(
            ResearchStage::Synthesis,
            &RouteResolutionInput::default(),
            None,
            None,
            None,
        );
        if let Some(c) = candidates.first() {
            assert_ne!(
                c.max_tokens,
                Some(1_800),
                "Synthesis max_tokens must not be hard-coded; got {:?}",
                c.max_tokens
            );
        }
    }

    #[test]
    #[serial_test::serial(model_pin_env)]
    fn verification_stage_uses_nonzero_temperature() {
        let candidates = cascade_with_optional_manual(
            ResearchStage::Verification,
            &RouteResolutionInput::default(),
            None,
            None,
            None,
        );
        assert!(
            candidates.iter().all(|c| c.temperature == Some(0.3)),
            "Verification stage must use nonzero temperature for self-consistency resampling to work"
        );
    }

    #[test]
    #[serial_test::serial(model_pin_env)]
    fn claim_extraction_and_judge_stages_stay_deterministic() {
        let claim_extraction = cascade_with_optional_manual(
            ResearchStage::ClaimExtraction,
            &RouteResolutionInput::default(),
            None,
            None,
            None,
        );
        assert!(claim_extraction.iter().all(|c| c.temperature == Some(0.0)));

        let judge = cascade_with_optional_manual(
            ResearchStage::Judge,
            &RouteResolutionInput::default(),
            None,
            None,
            None,
        );
        assert!(judge.iter().all(|c| c.temperature == Some(0.0)));
    }

    fn expected_free() -> Vec<String> {
        vox_config::OPENROUTER_FREE_FALLBACK_MODELS
            .iter()
            .map(|s| (*s).to_string())
            .collect()
    }

    #[test]
    fn research_models_append_free_floor_by_default() {
        let v = research_openrouter_model_ids("anthropic/claude-sonnet-4.6", false);
        // configured model first, then the concrete dispatchable :free floor.
        assert_eq!(v[0], "anthropic/claude-sonnet-4.6");
        assert_eq!(v[1..].to_vec(), expected_free());
        // every floor entry is a real, dispatchable :free slug (not the virtual route).
        assert!(v[1..].iter().all(|m| m.ends_with(":free")));
        assert!(!v.iter().any(|m| m == vox_config::OPENROUTER_FREE));
    }

    #[test]
    fn research_models_prefer_free_moves_it_first() {
        let v = research_openrouter_model_ids("anthropic/claude-sonnet-4.6", true);
        let n = expected_free().len();
        assert_eq!(v[..n].to_vec(), expected_free());
        assert_eq!(v.last().unwrap(), "anthropic/claude-sonnet-4.6");
        assert!(v[..n].iter().all(|m| m.ends_with(":free")));
    }

    #[test]
    fn research_models_no_duplicate_when_configured_is_already_free() {
        let slug = vox_config::OPENROUTER_FREE_FALLBACK_MODELS[0];
        let v = research_openrouter_model_ids(slug, false);
        // a configured model that is already a free slug appears exactly once.
        assert_eq!(v.iter().filter(|m| m.as_str() == slug).count(), 1);
        assert_eq!(v, expected_free());
    }

    /// Task 13: each research stage is pinned through its model role.
    #[test]
    fn research_stages_map_to_model_roles() {
        use vox_config::inference::ModelRole;
        assert_eq!(ResearchStage::Planner.model_role(), ModelRole::Planner);
        assert_eq!(ResearchStage::Synthesis.model_role(), ModelRole::Synthesis);
        assert_eq!(ResearchStage::Judge.model_role(), ModelRole::Judge);
        assert_eq!(
            ResearchStage::Verification.model_role(),
            ModelRole::Verifier
        );
        assert_eq!(
            ResearchStage::ClaimExtraction.model_role(),
            ModelRole::Verifier
        );
        assert_eq!(
            ResearchStage::SelfVerification.model_role(),
            ModelRole::Verifier
        );
    }

    /// Task 13: the research cascade honours the stage's own role pin. Every
    /// test in this module that builds a cascade (and so reads the pin env) is
    /// `#[serial(model_pin_env)]`, so this env mutation cannot race them.
    #[test]
    #[serial_test::serial(model_pin_env)]
    #[allow(unsafe_code)]
    fn research_cascade_uses_the_stage_role_pin() {
        let prev = std::env::var("VOX_MODEL_FORCE_JUDGE").ok();
        // SAFETY: serialized with every pin-env reader in this module (above).
        unsafe { std::env::set_var("VOX_MODEL_FORCE_JUDGE", "vendor/judge-pin") };
        vox_config::snapshot::bump(&["VOX_MODEL_FORCE_JUDGE"]);
        let judge =
            cascade_for_research_stage(ResearchStage::Judge, &RouteResolutionInput::default());
        unsafe {
            match prev {
                Some(v) => std::env::set_var("VOX_MODEL_FORCE_JUDGE", v),
                None => std::env::remove_var("VOX_MODEL_FORCE_JUDGE"),
            }
        }
        vox_config::snapshot::bump(&["VOX_MODEL_FORCE_JUDGE"]);
        assert_eq!(judge.len(), 1, "a pin is exactly one candidate");
        assert_eq!(judge[0].model, "vendor/judge-pin");
    }

    /// Task 13: a successful research-stage call records the requested id (a pin
    /// or `~…-latest` alias) next to the concrete id the provider reported, so a
    /// trace can show both. Outside a recording scope nothing is recorded.
    #[tokio::test]
    async fn research_calls_record_requested_and_resolved_model_ids() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "model": "vendor/concrete-model-1",
                "choices": [{"message": {"role": "assistant", "content": "{\"ok\":true}"}}],
                "usage": {"prompt_tokens": 3, "completion_tokens": 1}
            })))
            .mount(&server)
            .await;
        let candidate = || {
            let mut c = LlmConfig::openrouter("~vendor/model-latest".to_string());
            c.base_url = Some(format!("{}/chat/completions", server.uri()));
            c.api_key = Some("test-key".into());
            c.telemetry_skip_interaction = true;
            vec![c]
        };

        let (res, uses) = record_research_model_uses(async {
            let a = chat_with_cascade(
                &ActivityOptions::default(),
                vec![],
                candidate(),
                Some(ResearchStage::Judge),
            )
            .await;
            let b = chat_with_cascade_parsed(
                &ActivityOptions::default(),
                vec![],
                candidate(),
                Some(ResearchStage::Verification),
                |s| Ok::<_, String>(s.to_string()),
            )
            .await;
            (a.is_ok(), b.is_ok())
        })
        .await;
        assert_eq!(res, (true, true));
        assert_eq!(
            uses,
            vec![
                ResearchModelUse {
                    stage: ResearchStage::Judge,
                    requested: "~vendor/model-latest".into(),
                    resolved: "vendor/concrete-model-1".into(),
                },
                ResearchModelUse {
                    stage: ResearchStage::Verification,
                    requested: "~vendor/model-latest".into(),
                    resolved: "vendor/concrete-model-1".into(),
                },
            ]
        );

        // No recording scope: the call still works and nothing panics.
        assert!(
            chat_with_cascade(
                &ActivityOptions::default(),
                vec![],
                candidate(),
                Some(ResearchStage::Judge)
            )
            .await
            .is_ok()
        );
    }

    #[tokio::test]
    async fn chat_with_cascade_parsed_empty_candidates_errors() {
        let res = chat_with_cascade_parsed::<String, _>(
            &ActivityOptions::default(),
            vec![],
            vec![],
            None,
            |s| Ok(s.to_string()),
        )
        .await;
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("no LLM candidates available"));
    }
}
