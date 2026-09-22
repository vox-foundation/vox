use std::sync::Arc;

use vox_orchestrator_types::socrates_policy::ConfidencePolicy;

use vox_orchestrator::services::embeddings::EmbeddingService;

/// Progress reporting callback for research operations.
pub type ProgressCallback = dyn Fn(String, Option<f32>) + Send + Sync + 'static;

/// Completeness rider for all research LLM prompts.
pub(super) const RESEARCH_COMPLETENESS_RIDER: &str = "
<research_completeness_rider>
Cite every material claim. Do not omit contradicting evidence. Do not pad the summary with unsupported filler.
</research_completeness_rider>
";

/// Configuration for a single research run.
#[derive(Clone)]
pub struct ResearchConfig {
    /// LLM endpoint base URL (e.g. `https://api.openai.com`).
    pub llm_endpoint: Option<String>,
    /// Bearer API key for the LLM endpoint.
    pub api_key: Option<String>,
    /// Model used for query decomposition / planning.
    pub planner_model: String,
    /// Sampling temperature for the planner (lower = more deterministic).
    pub planner_temperature: f32,
    /// Maximum number of subqueries the planner may emit.
    pub planner_max_subqueries: usize,
    /// Model used for claim extraction.
    pub claim_model: String,
    /// Max tokens for a single claim-extraction response.
    pub claim_max_tokens: u32,
    /// Model used for answer synthesis.
    pub synthesis_model: String,
    /// Sampling temperature for synthesis.
    pub synthesis_temperature: f32,
    /// Max tokens for synthesis response.
    pub synthesis_max_tokens: u32,
    /// Model used for the LLM-as-judge quality scorer.
    pub judge_model: String,
    /// Sampling temperature for the judge.
    pub judge_temperature: f32,
    /// Max tokens for the judge response.
    pub judge_max_tokens: u32,
    /// Max chars for the synthesis LLM context (hits + verdict text).
    pub synthesis_context_max_chars: usize,
    /// Maximum characters per extracted chunk.
    pub chunk_max_chars: usize,
    /// Chars of overlap between consecutive chunks.
    pub chunk_overlap_chars: usize,
    /// Multiplier applied to the provider score for high-trust domains.
    pub trust_multiplier: f64,
    /// Minimum confidence before a doc is persisted to `docs/src/research/`.
    pub persist_min_confidence: f64,
    /// Confidence gate configuration.
    pub gate: super::super::config::GateConfig,
    /// Claim verifier configuration.
    pub verifier: super::super::config::VerifierConfig,
    /// Routing tier thresholds.
    pub routing_thresholds: super::super::config::RoutingThresholds,
    /// Fusion weights: (vector_weight, bm25_weight, kb_chunk_weight).
    pub fusion_weights: (f64, f64, f64),
    /// Minimum confidence to write a Mens training pair.
    pub training_pair_min_confidence: f64,
    /// Minimum citation count to write a Mens training pair.
    pub training_pair_min_citations: usize,
    /// Max age in seconds for a cached research result.
    pub cache_ttl_secs: u64,
    /// Provider configuration (high-trust domains, timeout, etc.).
    pub provider: super::super::config::ProviderConfig,
    /// Minimum distinct source domains required before synthesis (citation diversity gate).
    pub min_distinct_domains: usize,
    /// Optional embedding service for indexing chunks.
    pub embedder: Option<Arc<EmbeddingService>>,
    /// Whether claim detection and verification is enabled for this run.
    ///
    /// Can be overridden at runtime by the `rollout.claim_detection` config key.
    pub claim_detection_enabled: bool,
    /// Optional callback for progress reporting.
    pub progress_callback: Option<Arc<ProgressCallback>>,
    /// Optional SCIENTIA event sink for mesh signal emission.
    pub event_emitter: Option<Arc<dyn vox_research_events::ResearchEventEmitter>>,
    /// Optional retrieval feedback (caller-supplied). When unset and a DB is attached, the pipeline
    /// loads a short rolling window from `research_metrics` before applying [`vox_search::SearchPolicy::with_scientia_feedback`].
    pub search_policy_feedback: Option<vox_search::SearchPolicyFeedback>,
    /// Optional snapshot of workspace inference policy for registry stage picks.
    ///
    /// Phase 0a STUB: uses `super::super::model_select::InferenceConfig` (static fallbacks).
    /// Phase 1 replaces with `vox_orchestrator::mode::InferenceConfig` when that module is activated.
    pub model_pick_inference: Option<super::super::model_select::InferenceConfig>,
    /// Tunable retrieval search policy (provider toggles, timeouts, weights).
    pub search_policy: vox_search::policy::SearchPolicy,
}

impl std::fmt::Debug for ResearchConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResearchConfig")
            .field("llm_endpoint", &self.llm_endpoint)
            .field("planner_model", &self.planner_model)
            .field("claim_model", &self.claim_model)
            .field("synthesis_model", &self.synthesis_model)
            .field("judge_model", &self.judge_model)
            .field("chunk_max_chars", &self.chunk_max_chars)
            .field("fusion_weights", &self.fusion_weights)
            .field("event_emitter", &self.event_emitter.is_some())
            .field(
                "search_policy_feedback",
                &self.search_policy_feedback.is_some(),
            )
            .field("search_policy", &self.search_policy)
            .finish_non_exhaustive()
    }
}

impl Default for ResearchConfig {
    /// Phase 0a STUB: uses static fallback model IDs from vox-config constants.
    /// Phase 1 wires to live ModelRegistry resolution via InferenceConfig.
    fn default() -> Self {
        let reg = vox_orchestrator::models::ModelRegistry::new();
        let base = super::super::model_select::InferenceConfig::default();
        let r = super::super::model_select::resolve_research_models(&reg, &base);
        let verifier = super::super::config::VerifierConfig {
            nli_model_id: r.claim_model.clone(),
            ..super::super::config::VerifierConfig::default()
        };
        Self {
            llm_endpoint: None,
            api_key: None,
            planner_model: r.planner_model,
            planner_temperature: 0.3,
            planner_max_subqueries: 6,
            claim_model: r.claim_model,
            claim_max_tokens: 512,
            synthesis_model: r.synthesis_model,
            synthesis_temperature: 0.2,
            // D9 (Task 8 fix round 4): 1200 truncated the answer mid-sentence at
            // live scale. Live-probed against google/gemini-3.8-flash with a
            // 24k-char evidence prompt (the shipped `synthesis_context_max_chars`):
            // 1200 => completion_tokens 1202, cut off mid-sentence; 2400 => 1803
            // tokens, complete; 4000 => 1562; 8000 => 962. The natural length for
            // the five mandated markdown sections tops out near 1800 tokens, so
            // 2400 is marginal — 4000 keeps headroom without being unbounded, and
            // stays under the chat path's 8192 output cap. See
            // `synthesis_probe_max_tokens_budget_vs_visible_output`.
            synthesis_max_tokens: 4000,
            judge_model: r.judge_model,
            judge_temperature: 0.0,
            // D8 (Task 8 fix round 2): 400 was too tight for the judge's own JSON
            // schema (3 free-text `*_reasoning` fields plus 4 integer scores) —
            // live-probed against google/gemini-3.8-flash: 400 => unparseable
            // truncated JSON every time; 1200/1600/4000 all returned a parseable
            // score. 1600 keeps headroom above the observed 1200 floor without
            // being unbounded.
            //
            // D9 (Task 8 fix round 4): 1600 was sized against a *truncated*
            // 201-char answer. Once the synthesis fix below let the answer reach
            // its natural ~3800 chars, the judge's own `*_reasoning` strings grew
            // with it and 1600 truncated again live ("judge returned unparseable
            // JSON: {\n  \"factual_accuracy_reasoning\": \"The provided
            // bibliography lists five unrelated arXiv papers…"). Raised to 4000,
            // matching `synthesis_max_tokens` — the judge reads the synthesis
            // output, so its budget must scale with it.
            judge_max_tokens: 4000,
            synthesis_context_max_chars: 24000,
            chunk_max_chars: 1200,
            chunk_overlap_chars: 150,
            trust_multiplier: 1.2,
            persist_min_confidence: ConfidencePolicy::DEFAULT_MIN_PERSIST_CONFIDENCE,
            gate: super::super::config::GateConfig::default(),
            verifier,
            routing_thresholds: super::super::config::RoutingThresholds::default(),
            fusion_weights: (0.65, 0.50, 0.80),
            training_pair_min_confidence: ConfidencePolicy::DEFAULT_MIN_TRAINING_PAIR_CONFIDENCE,
            training_pair_min_citations: 2,
            min_distinct_domains: 3,
            cache_ttl_secs: 3600,
            provider: super::super::config::ProviderConfig::default(),
            embedder: None,
            claim_detection_enabled: true,
            progress_callback: None,
            event_emitter: None,
            search_policy_feedback: None,
            model_pick_inference: None,
            search_policy: vox_search::policy::SearchPolicy::from_env(),
        }
    }
}
