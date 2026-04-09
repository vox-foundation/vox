//! Canonical context envelope shared across MCP, orchestrator, and Populi-adjacent flows.

use serde::{Deserialize, Serialize};

/// High-level payload category carried by a [`ContextEnvelope`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextEnvelopeType {
    ChatTurn,
    SessionSummary,
    RetrievalEvidence,
    TaskContext,
    HandoffContext,
    AgentNote,
    PolicyHint,
    ExecutionContext,
}

/// Context source plane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextSourcePlane {
    Mcp,
    Orchestrator,
    Search,
    Populi,
    Codex,
    Manual,
    External,
}

/// Capture mode used to produce a context payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextCaptureMode {
    Inline,
    Derived,
    Retrieved,
    Compacted,
    HandedOff,
    Imported,
}

/// Trust tier used by context conflict and injection policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextTrustTier {
    Untrusted,
    Advisory,
    Trusted,
    SystemVerified,
}

/// Freshness class for context payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextFreshnessTier {
    Volatile,
    Recent,
    Stable,
    Archival,
}

/// Merge strategy for conflicting context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextMergeStrategy {
    AppendOnly,
    LastWriteWins,
    ConfidenceWeighted,
    AuthorityPrecedence,
    CrdtMerge,
    ManualReview,
}

/// Conflict class for merge policy and telemetry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextConflictClass {
    Temporal,
    Semantic,
    Authority,
    SourceTrust,
    Policy,
}

/// Injection mode for downstream context assembly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextInjectionMode {
    Inline,
    SummaryOnly,
    ReferenceOnly,
    ToolRequired,
}

/// Priority used by context budget policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextPriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Retrieval-cost class used by policy layers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextRetrievalCostClass {
    Cheap,
    Moderate,
    Expensive,
}

/// Source-level provenance metadata for the envelope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextProvenance {
    pub source_plane: ContextSourcePlane,
    pub source_system: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_tool: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub producer_agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub producer_node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub producer_session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub producer_thread_id: Option<String>,
    pub capture_mode: ContextCaptureMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub policy_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub observed_via: Vec<String>,
    /// End-to-end trace id (e.g. MCP → orchestrator → retrieval); optional for backward compat.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    /// Correlation id for this tool call or session thread (may equal `trace_id`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
}

/// Trust and confidence metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextTrust {
    pub trust_tier: ContextTrustTier,
    pub authority_rank: u32,
    pub freshness_tier: ContextFreshnessTier,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contradiction_ratio: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires_citation: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub may_override_lower_authority: Option<bool>,
}

/// Derivation parent reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextDerivedRef {
    pub kind: String,
    pub r#ref: String,
}

/// Optional derivation lineage metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ContextLineage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_envelope_id: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parent_envelope_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_conversation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hop_count: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compaction_generation: Option<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub derived_from: Vec<ContextDerivedRef>,
}

/// Subject scope metadata for anti-bleed boundaries.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextSubject {
    pub repository_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receiver_agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub populi_scope_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface: Option<String>,
}

/// Structured fact inside envelope content.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextFact {
    pub fact_id: String,
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supersedes_fact_ids: Vec<String>,
}

/// Emitted when a context section is cut short by the character budget.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextTruncatedWarning {
    pub section: String,
    pub chars_included: usize,
    pub chars_dropped: usize,
    pub session_id: String,
}

/// Core content payload.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextContent {
    pub summary_text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub facts: Vec<ContextFact>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repo_paths: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifact_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub citations: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structured_payload: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub truncated_warnings: Vec<ContextTruncatedWarning>,
}

/// Conflict handling policy.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextConflictPolicy {
    pub merge_strategy: ContextMergeStrategy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stale_after_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dedupe_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overwrite_requires_evidence: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conflict_class: Option<ContextConflictClass>,
}

/// Budget and injection strategy metadata.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextBudget {
    pub priority: ContextPriority,
    pub injection_mode: ContextInjectionMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_estimate: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens_for_injection: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retrieval_cost_class: Option<ContextRetrievalCostClass>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub must_refresh_before_use: Option<bool>,
}

/// Socrates-facing safety hints that may be mirrored from task context.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ContextSafety {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk_budget: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub factual_mode: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_citations: Option<u32>,
}

/// Canonical context artifact.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ContextEnvelope {
    pub schema_version: u32,
    pub envelope_type: ContextEnvelopeType,
    pub envelope_id: String,
    pub created_at_unix_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at_unix_ms: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ttl_seconds: Option<u64>,
    pub provenance: ContextProvenance,
    pub trust: ContextTrust,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lineage: Option<ContextLineage>,
    pub subject: ContextSubject,
    pub content: ContextContent,
    pub conflict_policy: ContextConflictPolicy,
    pub budget: ContextBudget,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safety: Option<ContextSafety>,
}

impl ContextEnvelope {
    /// Build a retrieval envelope projection from the orchestrator session retrieval bridge shape.
    #[must_use]
    pub fn from_session_retrieval(
        repository_id: impl Into<String>,
        session_id: impl Into<String>,
        retrieval: &crate::socrates::SessionRetrievalEnvelope,
    ) -> Self {
        let now = crate::types::now_unix_ms();
        let repo_id = repository_id.into();
        let sid = session_id.into();
        let hit_count = retrieval
            .memory_hit_count
            .saturating_add(retrieval.knowledge_hit_count)
            .saturating_add(retrieval.chunk_hit_count)
            .saturating_add(retrieval.repo_hit_count)
            .saturating_add(retrieval.rrf_fused_hit_count);
        let contradiction_ratio = if hit_count == 0 {
            None
        } else {
            Some((retrieval.contradiction_count as f64 / hit_count as f64).clamp(0.0, 1.0))
        };
        Self {
            schema_version: 1,
            envelope_type: ContextEnvelopeType::RetrievalEvidence,
            envelope_id: format!("retrieval-envelope-{sid}-{now}"),
            created_at_unix_ms: now,
            expires_at_unix_ms: Some(now.saturating_add(3_600_000)),
            ttl_seconds: Some(3600),
            provenance: ContextProvenance {
                source_plane: ContextSourcePlane::Orchestrator,
                source_system: "vox-orchestrator".to_string(),
                source_tool: Some("session_retrieval_envelope".to_string()),
                source_path: Some("context_store".to_string()),
                producer_agent_id: Some("0".to_string()),
                producer_node_id: None,
                producer_session_id: Some(sid.clone()),
                producer_thread_id: None,
                capture_mode: ContextCaptureMode::Retrieved,
                policy_version: None,
                observed_via: vec!["context_store_key:retrieval_envelope".to_string()],
                trace_id: None,
                correlation_id: None,
            },
            trust: ContextTrust {
                trust_tier: ContextTrustTier::Trusted,
                authority_rank: 70,
                freshness_tier: ContextFreshnessTier::Recent,
                confidence: Some(retrieval.evidence_quality.clamp(0.0, 1.0)),
                contradiction_ratio,
                requires_citation: Some(hit_count == 0),
                may_override_lower_authority: Some(true),
            },
            lineage: None,
            subject: ContextSubject {
                repository_id: repo_id.clone(),
                workspace_id: None,
                session_id: Some(sid.clone()),
                thread_id: None,
                task_id: None,
                goal_id: None,
                agent_id: Some("0".to_string()),
                receiver_agent_id: None,
                node_id: None,
                populi_scope_id: None,
                surface: Some("retrieval".to_string()),
            },
            content: ContextContent {
                summary_text: format!(
                    "retrieval_tier={} memory_hits={} knowledge_hits={} chunk_hits={} repo_hits={} contradictions={}",
                    retrieval.retrieval_tier,
                    retrieval.memory_hit_count,
                    retrieval.knowledge_hit_count,
                    retrieval.chunk_hit_count,
                    retrieval.repo_hit_count,
                    retrieval.contradiction_count
                ),
                facts: Vec::new(),
                repo_paths: Vec::new(),
                artifact_refs: Vec::new(),
                citations: Vec::new(),
                tags: vec![
                    "retrieval".to_string(),
                    retrieval.retrieval_tier.clone(),
                    if retrieval.used_vector {
                        "used_vector".to_string()
                    } else {
                        "no_vector".to_string()
                    },
                    if retrieval.used_bm25 {
                        "used_bm25".to_string()
                    } else {
                        "no_bm25".to_string()
                    },
                ],
                structured_payload: serde_json::to_value(retrieval).ok(),
                truncated_warnings: Vec::new(),
            },
            conflict_policy: ContextConflictPolicy {
                merge_strategy: ContextMergeStrategy::ConfidenceWeighted,
                stale_after_ms: Some(3_600_000),
                dedupe_key: Some(format!("retrieval:{repo_id}:{sid}")),
                overwrite_requires_evidence: Some(true),
                conflict_class: Some(ContextConflictClass::SourceTrust),
            },
            budget: ContextBudget {
                priority: ContextPriority::Normal,
                injection_mode: ContextInjectionMode::Inline,
                token_estimate: None,
                max_tokens_for_injection: None,
                retrieval_cost_class: Some(ContextRetrievalCostClass::Moderate),
                must_refresh_before_use: Some(false),
            },
            safety: Some(ContextSafety {
                risk_budget: Some("normal".to_string()),
                factual_mode: Some(true),
                required_citations: Some(if hit_count == 0 { 1 } else { 0 }),
            }),
        }
    }

    /// Build a task-context envelope projection from Socrates task context.
    #[must_use]
    pub fn from_task_socrates_context(
        repository_id: impl Into<String>,
        task_id: crate::types::TaskId,
        session_id: Option<String>,
        ctx: &crate::socrates::SocratesTaskContext,
    ) -> Self {
        let now = crate::types::now_unix_ms();
        let repo_id = repository_id.into();
        Self {
            schema_version: 1,
            envelope_type: ContextEnvelopeType::TaskContext,
            envelope_id: format!("task-context-{}-{now}", task_id.0),
            created_at_unix_ms: now,
            expires_at_unix_ms: None,
            ttl_seconds: None,
            provenance: ContextProvenance {
                source_plane: ContextSourcePlane::Orchestrator,
                source_system: "vox-orchestrator".to_string(),
                source_tool: Some("attach_socrates_context".to_string()),
                source_path: Some("agent_queue".to_string()),
                producer_agent_id: None,
                producer_node_id: None,
                producer_session_id: session_id.clone(),
                producer_thread_id: None,
                capture_mode: ContextCaptureMode::Derived,
                policy_version: None,
                observed_via: vec!["socrates".to_string()],
                trace_id: None,
                correlation_id: None,
            },
            trust: ContextTrust {
                trust_tier: ContextTrustTier::Trusted,
                authority_rank: 80,
                freshness_tier: ContextFreshnessTier::Recent,
                confidence: Some(ctx.evidence_quality.clamp(0.0, 1.0)),
                contradiction_ratio: Some((ctx.contradiction_hints as f64 / 10.0).clamp(0.0, 1.0)),
                requires_citation: Some(ctx.required_citations > 0),
                may_override_lower_authority: Some(true),
            },
            lineage: None,
            subject: ContextSubject {
                repository_id: repo_id,
                workspace_id: None,
                session_id,
                thread_id: None,
                task_id: Some(task_id.0.to_string()),
                goal_id: None,
                agent_id: None,
                receiver_agent_id: None,
                node_id: None,
                populi_scope_id: None,
                surface: Some("task_submit".to_string()),
            },
            content: ContextContent {
                summary_text: format!(
                    "risk_budget={} factual_mode={} required_citations={} evidence_count={} contradiction_hints={}",
                    ctx.risk_budget,
                    ctx.factual_mode,
                    ctx.required_citations,
                    ctx.evidence_count,
                    ctx.contradiction_hints
                ),
                facts: Vec::new(),
                repo_paths: Vec::new(),
                artifact_refs: Vec::new(),
                citations: Vec::new(),
                tags: vec![
                    "task_context".to_string(),
                    ctx.retrieval_tier
                        .clone()
                        .unwrap_or_else(|| "none".to_string()),
                ],
                structured_payload: serde_json::to_value(ctx).ok(),
                truncated_warnings: Vec::new(),
            },
            conflict_policy: ContextConflictPolicy {
                merge_strategy: ContextMergeStrategy::AuthorityPrecedence,
                stale_after_ms: None,
                dedupe_key: Some(format!("task:{}", task_id.0)),
                overwrite_requires_evidence: Some(true),
                conflict_class: Some(ContextConflictClass::Authority),
            },
            budget: ContextBudget {
                priority: ContextPriority::High,
                injection_mode: ContextInjectionMode::Inline,
                token_estimate: None,
                max_tokens_for_injection: None,
                retrieval_cost_class: None,
                must_refresh_before_use: Some(false),
            },
            safety: Some(ContextSafety {
                risk_budget: Some(ctx.risk_budget.clone()),
                factual_mode: Some(ctx.factual_mode),
                required_citations: Some(ctx.required_citations as u32),
            }),
        }
    }
}
