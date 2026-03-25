//! Typed orchestrator settings: scaling, queues, compaction, sessions, and env overlay.
//!
//! [`OrchestratorConfig`] loads from `Vox.toml` / environment and is validated before use.

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::compaction::CompactionConfig;
use crate::contract::{OrchestrationMigrationFlags, TaskCapabilityHints};
use crate::memory::MemoryConfig;
use crate::scope::ScopeEnforcement;
use crate::session::SessionConfig;
use crate::types::TaskPriority;
use vox_socrates_policy::{ConfidencePolicy, ConfidencePolicyOverride};

/// Strategy for handling queue overflow when max tasks is reached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverflowStrategy {
    /// Block the request until space is available.
    Block,
    /// Drop the lowest-priority task to make room.
    DropLowest,
    /// Spawn a new agent to handle overflow.
    SpawnNewAgent,
}

/// Preference for balancing model quality vs operational cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CostPreference {
    /// Prioritize model performance/quality over cost.
    Performance,
    /// Prioritize lower cost models even if quality is slightly reduced.
    Economy,
}

/// User-governable scaling profile: when to scale up and how aggressively to scale down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScalingProfile {
    /// Scale up only when load is high; retire idle agents quickly.
    Conservative,
    /// Default balance of scale-up threshold and retirement time.
    #[default]
    Balanced,
    /// Scale up earlier; keep idle agents longer.
    Aggressive,
}

impl ScalingProfile {
    /// Multiplier for scaling_threshold (higher = scale up later).
    pub fn threshold_multiplier(self) -> f64 {
        match self {
            ScalingProfile::Conservative => 1.5,
            ScalingProfile::Balanced => 1.0,
            ScalingProfile::Aggressive => 0.7,
        }
    }

    /// Multiplier for idle_retirement_ms (higher = retire later).
    pub fn retirement_multiplier(self) -> f64 {
        match self {
            ScalingProfile::Conservative => 0.6,
            ScalingProfile::Balanced => 1.0,
            ScalingProfile::Aggressive => 1.5,
        }
    }
}

/// Configuration for the orchestrator system.
///
/// Can be loaded from the `[orchestrator]` section in `Vox.toml`,
/// overridden by `VOX_ORCHESTRATOR_*` environment variables,
/// or constructed programmatically.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct OrchestratorConfig {
    /// Whether the orchestrator is enabled (default: true).
    pub enabled: bool,
    /// Maximum number of concurrent agents (default: 8).
    pub max_agents: usize,
    /// Default priority for new tasks (default: Normal).
    pub default_priority: TaskPriority,
    /// How to handle queue overflow (default: SpawnNewAgent).
    pub queue_overflow_strategy: OverflowStrategy,
    /// Lock timeout in milliseconds (default: 30000).
    pub lock_timeout_ms: u64,
    /// Bulletin board broadcast channel capacity (default: 256).
    pub bulletin_capacity: usize,
    /// Whether to fall back to a single agent when routing is ambiguous (default: true).
    pub fallback_to_single_agent: bool,
    /// Whether to run TOESTUB validation after each completed task (default: true).
    pub toestub_gate: bool,
    /// Maximum number of times a task can be re-routed due to validation failures (default: 3).
    pub max_debug_iterations: u8,
    /// Emit Socrates gate decisions to logs without blocking completion (default: false).
    #[serde(default = "default_false")]
    pub socrates_gate_shadow: bool,
    /// When true, a non-answer Socrates risk decision requeues the task for remediation (default: false).
    #[serde(default = "default_false")]
    pub socrates_gate_enforce: bool,
    /// Blend `agent_reliability` (Arca V10) into routing when a VoxDb is attached (default: false).
    #[serde(default = "default_false")]
    pub socrates_reputation_routing: bool,
    /// Optional Socrates confidence thresholds merged onto [`ConfidencePolicy::workspace_default`].
    #[serde(default)]
    pub socrates_policy: Option<ConfidencePolicyOverride>,
    /// Weight applied to Arca `agent_reliability` when blending into routing scores (default: 1.0).
    #[serde(default = "default_socrates_reputation_weight")]
    pub socrates_reputation_weight: f64,
    /// Log level for orchestrator events (default: "info").
    pub log_level: String,
    /// Global system idle timeout in milliseconds (default: 600000 / 10min).
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout_ms: u64,
    /// Default task execution timeout in milliseconds (default: 1800000 / 30min).
    #[serde(default = "default_task_timeout")]
    pub task_timeout_ms: u64,

    // ── Phase 1: New fields ──────────────────────────────────
    /// Heartbeat check interval in milliseconds (default: 5000).
    #[serde(default = "default_heartbeat_interval")]
    pub heartbeat_interval_ms: u64,
    /// Threshold in milliseconds before an agent is considered stale (default: 60000).
    #[serde(default = "default_stale_threshold")]
    pub stale_threshold_ms: u64,
    /// Whether auto-continuation is enabled (default: true).
    #[serde(default = "default_true")]
    pub auto_continue_enabled: bool,
    /// Cooldown between auto-continuations per agent in ms (default: 30000).
    #[serde(default = "default_continuation_cooldown")]
    pub continuation_cooldown_ms: u64,
    /// Maximum auto-continuations before requiring manual intervention (default: 5).
    #[serde(default = "default_max_auto_continuations")]
    pub max_auto_continuations: u32,
    /// How strictly to enforce agent scope boundaries (default: Warn).
    #[serde(default)]
    pub scope_enforcement: ScopeEnforcement,
    /// Event bus capacity (default: 1024).
    #[serde(default = "default_event_capacity")]
    pub event_bus_capacity: usize,
    /// Default GPU / capability hints for newly spawned agent queues.
    #[serde(default)]
    pub default_agent_capabilities: TaskCapabilityHints,
    /// MCP/CLI wire migration toggles (v2 contract hints, legacy fallback).
    #[serde(default)]
    pub orchestration_migration: OrchestrationMigrationFlags,

    // ── Phase 12: Scaling & Cost ─────────────────────────────
    /// Minimum number of concurrent agents (default: 1).
    #[serde(default = "default_min_agents")]
    pub min_agents: usize,
    /// Number of queued tasks per agent to trigger scaling (default: 5).
    #[serde(default = "default_scaling_threshold")]
    pub scaling_threshold: usize,
    /// Time an idle dynamic agent lives before retirement in ms (default: 300000 / 5min).
    #[serde(default = "default_idle_retirement")]
    pub idle_retirement_ms: u64,
    /// Whether dynamic scaling is enabled (default: false).
    #[serde(default = "default_false")]
    pub scaling_enabled: bool,
    /// Preference for cost vs performance (default: Performance).
    #[serde(default = "default_cost_preference")]
    pub cost_preference: CostPreference,
    /// Number of ticks to look back for predictive scaling (default: 5).
    #[serde(default = "default_lookback_ticks")]
    pub scaling_lookback_ticks: usize,
    /// Weight of system resource usage in load calculation (0.0 to 1.0, default: 0.3).
    #[serde(default = "default_resource_weight")]
    pub resource_weight: f64,
    /// Baseline multiplier for CPU usage in the load calculation (default: 0.7).
    #[serde(default = "default_cpu_multiplier")]
    pub resource_cpu_multiplier: f64,
    /// Baseline multiplier for Memory usage in the load calculation (default: 0.3).
    #[serde(default = "default_mem_multiplier")]
    pub resource_mem_multiplier: f64,
    /// Exponent to apply to the final resource factor, allowing exponential scaling (default: 1.0).
    #[serde(default = "default_resource_exponent")]
    pub resource_exponent: f64,
    /// User-governable scaling profile (conservative / balanced / aggressive).
    #[serde(default)]
    pub scaling_profile: ScalingProfile,
    /// Max number of agents to spawn in one scaling tick (default: 1).
    #[serde(default = "default_max_spawn_per_tick")]
    pub max_spawn_per_tick: usize,
    /// Cooldown in ms between scale-up actions (default: 5000).
    #[serde(default = "default_scaling_cooldown_ms")]
    pub scaling_cooldown_ms: u64,
    /// Number of Urgent tasks on a single agent that triggers an automatic rebalance (default: 3).
    /// Set to 0 to disable urgent auto-rebalance.
    #[serde(default = "default_urgent_rebalance_threshold")]
    pub urgent_rebalance_threshold: usize,

    // ── OpenClaw-Inspired Features ───────────────────────────────────────
    /// Configuration for the context compaction engine.
    #[serde(default)]
    pub compaction: CompactionConfig,
    /// Configuration for the persistent memory system.
    #[serde(default)]
    pub memory: MemoryConfig,
    /// Configuration for the session lifecycle manager.
    #[serde(default)]
    pub session: SessionConfig,
    /// Optional mens HTTP control plane base URL (`GET /v1/populi/nodes`) for read-only status federation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub populi_control_url: Option<String>,
    /// Optional mens cluster / tenancy id from `Vox.toml` `[mens].scope_id` or `VOX_MESH_SCOPE_ID` (env wins).
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "mesh_scope_id")]
    pub populi_scope_id: Option<String>,
    /// Background poll interval (seconds) for MCP populi federation cache; `0` disables the poller.
    #[serde(default = "default_populi_poll_interval_secs", alias = "mesh_poll_interval_secs")]
    pub populi_poll_interval_secs: u64,
    /// HTTP client timeout (milliseconds) for populi control plane `GET /v1/populi/nodes`.
    #[serde(default = "default_populi_http_timeout_ms", alias = "mesh_http_timeout_ms")]
    pub populi_http_timeout_ms: u64,
    /// Experimental: use remote populi node labels when scoring routes (no remote task execution).
    #[serde(default = "default_false", alias = "mesh_routing_experimental")]
    pub populi_routing_experimental: bool,
    /// Experimental: allow remote task-envelope dispatch over populi A2A relay with local fallback.
    #[serde(default = "default_false", alias = "mesh_remote_execute_experimental")]
    pub populi_remote_execute_experimental: bool,
    /// When true, MCP tool LLM calls collapse system/user turns into a single string
    /// formatted with `<|im_start|>` markers instead of JSON message arrays.
    #[serde(default = "default_false")]
    pub chatml_strict: bool,
    /// Enable dynamic planning mode (router + plan execution bridge).
    #[serde(default = "default_false")]
    pub planning_enabled: bool,
    /// Enable intake router classification at ingress.
    #[serde(default = "default_false")]
    pub planning_router_enabled: bool,
    /// Enable branch-based replanning after qualifying failures.
    #[serde(default = "default_false")]
    pub planning_replan_enabled: bool,
    /// Allow workflow runtime handoff path from planner.
    #[serde(default = "default_false")]
    pub planning_workflow_handoff_enabled: bool,
    /// Compute planning decisions but keep direct execution path.
    #[serde(default = "default_false")]
    pub planning_shadow_mode: bool,
    /// Enable `planning_mode=auto` behavior for goal ingress.
    #[serde(default = "default_false")]
    pub planning_auto_mode_enabled: bool,
    /// Rollout percentage for auto planning (0-100).
    #[serde(default)]
    pub planning_rollout_percent: u8,

    // ── Phase 15: Attention Budget ─────────────────────────────────────────────
    /// Enable attention budget tracking. Default: false (shadow/observe mode).
    #[serde(default = "default_false")]
    pub attention_enabled: bool,
    /// Pilot attention budget per session period in ms. Default: 3_600_000 (1 hr).
    #[serde(default = "default_attention_budget_ms")]
    pub attention_budget_ms: u64,
    /// Ratio of budget that triggers AttentionHigh signal. Default: 0.7.
    #[serde(default = "default_attention_alert_threshold")]
    pub attention_alert_threshold: f64,
    /// Baseline interrupt recovery cost in ms. Default: 23_250 (Gloria Mark).
    #[serde(default = "default_attention_interrupt_cost_ms")]
    pub attention_interrupt_cost_ms: u64,
    /// EWMA alpha for trust score updates. Default: 0.1.
    #[serde(default = "default_trust_ewma_alpha")]
    pub trust_ewma_alpha: f64,
    /// Minimum outcomes for Untrusted → Provisional. Default: 5.
    #[serde(default = "default_trust_provisional_threshold")]
    pub trust_provisional_threshold: u32,
    /// Minimum outcomes for Provisional → Trusted. Default: 20.
    #[serde(default = "default_trust_trusted_threshold")]
    pub trust_trusted_threshold: u32,
    /// Minimum trust score for auto-approve eligibility. Default: 0.85.
    #[serde(default = "default_trust_auto_approve_min")]
    pub trust_auto_approve_min: f64,
    /// Routing weight applied to trust scores in step 3d. Default: 2.0.
    #[serde(default = "default_attention_trust_routing_weight")]
    pub attention_trust_routing_weight: f64,
    /// NASA TLX subscale weights for attention cost computation.
    /// Defaults to validated pilot-study values (mental=0.35, temporal=0.25, etc.).
    #[serde(default)]
    pub attention_tlx_weights: crate::attention::NasaTlxWeights,
    /// Approval tier gate thresholds. Override to tune auto-approve graduation.
    #[serde(default)]
    pub tier_gate: crate::attention::TierGateConfig,
    /// Configuration for the unified news publisher (docs/news/ → RSS/X/GitHub).
    #[serde(default)]
    pub news: NewsConfig,
}

/// Unified news syndication configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NewsConfig {
    /// Whether the background news monitor is active (default: false).
    pub enabled: bool,
    /// Relative path to watch for new Markdown news items (default: "docs/news").
    pub news_dir: String,
    /// When true, walk `news_dir` recursively (includes `drafts/` subfolders).
    #[serde(default = "default_true")]
    pub scan_recursive: bool,
    /// Personal access token for GitHub Releases (Octocrab).
    pub github_token: Option<String>,
    /// Bearer token for Twitter X API v2 (reqwest).
    pub twitter_token: Option<String>,
    /// API Key for Open Collective GraphQL v2 (reqwest).
    pub opencollective_token: Option<String>,
    /// Global flag to force local testing only without actually calling external publish endpoints.
    pub dry_run: bool,
    /// Must be true (or `VOX_NEWS_PUBLISH_ARMED=1`) before any **live** syndication attempt.
    #[serde(default)]
    pub publish_armed: bool,
    /// Override public site URL for RSS links (default: vox-publisher contract default).
    #[serde(default)]
    pub site_base_url: Option<String>,
    /// Path to `feed.xml` relative to repo root.
    #[serde(default)]
    pub rss_feed_path: Option<String>,
    #[serde(default)]
    pub opencollective_graphql_url: Option<String>,
    #[serde(default)]
    pub github_graphql_url: Option<String>,
    #[serde(default)]
    pub github_rest_base: Option<String>,
    #[serde(default)]
    pub twitter_api_base: Option<String>,
    /// Optional override for tweet chunk max chars (defaults to publisher contract constant).
    #[serde(default)]
    pub twitter_text_chunk_max: Option<usize>,
    /// Optional truncation suffix for non-thread tweet shortening (default "...").
    #[serde(default)]
    pub twitter_truncation_suffix: Option<String>,
}

impl Default for NewsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            news_dir: "docs/news".to_string(),
            scan_recursive: true,
            github_token: None,
            twitter_token: None,
            opencollective_token: None,
            dry_run: true,
            publish_armed: false,
            site_base_url: None,
            rss_feed_path: None,
            opencollective_graphql_url: None,
            github_graphql_url: None,
            github_rest_base: None,
            twitter_api_base: None,
            twitter_text_chunk_max: None,
            twitter_truncation_suffix: None,
        }
    }
}

fn default_heartbeat_interval() -> u64 {
    5_000
}
fn default_stale_threshold() -> u64 {
    60_000
}
fn default_true() -> bool {
    true
}
fn default_continuation_cooldown() -> u64 {
    30_000
}
fn default_max_auto_continuations() -> u32 {
    5
}
fn default_event_capacity() -> usize {
    1024
}
fn default_min_agents() -> usize {
    1
}
fn default_scaling_threshold() -> usize {
    5
}
fn default_idle_retirement() -> u64 {
    300_000
}
fn default_false() -> bool {
    false
}
fn default_cost_preference() -> CostPreference {
    CostPreference::Performance
}
fn default_lookback_ticks() -> usize {
    5
}
fn default_resource_weight() -> f64 {
    0.3
}
fn default_cpu_multiplier() -> f64 {
    0.7
}
fn default_mem_multiplier() -> f64 {
    0.3
}
fn default_resource_exponent() -> f64 {
    1.0
}
fn default_max_spawn_per_tick() -> usize {
    1
}
fn default_scaling_cooldown_ms() -> u64 {
    5_000
}
fn default_urgent_rebalance_threshold() -> usize {
    3
}

fn default_populi_poll_interval_secs() -> u64 {
    30
}

fn default_populi_http_timeout_ms() -> u64 {
    10_000
}

fn default_socrates_reputation_weight() -> f64 {
    1.0
}

fn default_idle_timeout() -> u64 {
    600_000
}

fn default_task_timeout() -> u64 {
    1_800_000
}

// Phase 15: Attention budget defaults
fn default_attention_budget_ms() -> u64 {
    3_600_000
}
fn default_attention_alert_threshold() -> f64 {
    0.7
}
fn default_attention_interrupt_cost_ms() -> u64 {
    23_250
}
fn default_trust_ewma_alpha() -> f64 {
    0.1
}
fn default_trust_provisional_threshold() -> u32 {
    5
}
fn default_trust_trusted_threshold() -> u32 {
    20
}
fn default_trust_auto_approve_min() -> f64 {
    0.85
}
fn default_attention_trust_routing_weight() -> f64 {
    2.0
}

fn apply_vox_populi_toml(config: &mut OrchestratorConfig, mens: &vox_repository::VoxMeshToml) {
    if let Some(url) = mens
        .control_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        config.populi_control_url = Some(url.to_string());
    }
    if let Some(sid) = mens
        .scope_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        config.populi_scope_id = Some(sid.to_string());
    }
    if let Some(labels) = mens.labels.as_ref() {
        for lab in labels {
            let lab = lab.trim();
            if lab.is_empty() {
                continue;
            }
            let s = lab.to_string();
            if !config.default_agent_capabilities.labels.contains(&s) {
                config.default_agent_capabilities.labels.push(s);
            }
        }
    }
    if mens.advertise_gpu == Some(true) {
        config.default_agent_capabilities.gpu_cuda = true;
    }
}

impl Default for OrchestratorConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_agents: 8,
            default_priority: TaskPriority::Normal,
            queue_overflow_strategy: OverflowStrategy::SpawnNewAgent,
            lock_timeout_ms: 30_000,
            bulletin_capacity: 256,
            fallback_to_single_agent: true,
            toestub_gate: true,
            max_debug_iterations: 3,
            socrates_gate_shadow: default_false(),
            socrates_gate_enforce: default_false(),
            socrates_reputation_routing: default_false(),
            log_level: "info".to_string(),
            idle_timeout_ms: default_idle_timeout(),
            task_timeout_ms: default_task_timeout(),
            heartbeat_interval_ms: default_heartbeat_interval(),
            stale_threshold_ms: default_stale_threshold(),
            auto_continue_enabled: default_true(),
            continuation_cooldown_ms: default_continuation_cooldown(),
            max_auto_continuations: default_max_auto_continuations(),
            scope_enforcement: ScopeEnforcement::default(),
            event_bus_capacity: default_event_capacity(),
            default_agent_capabilities: TaskCapabilityHints::default(),
            orchestration_migration: OrchestrationMigrationFlags::default(),
            min_agents: default_min_agents(),
            scaling_threshold: default_scaling_threshold(),
            idle_retirement_ms: default_idle_retirement(),
            scaling_enabled: default_false(),
            cost_preference: default_cost_preference(),
            scaling_lookback_ticks: default_lookback_ticks(),
            resource_weight: default_resource_weight(),
            resource_cpu_multiplier: default_cpu_multiplier(),
            resource_mem_multiplier: default_mem_multiplier(),
            resource_exponent: default_resource_exponent(),
            scaling_profile: ScalingProfile::default(),
            max_spawn_per_tick: default_max_spawn_per_tick(),
            scaling_cooldown_ms: default_scaling_cooldown_ms(),
            urgent_rebalance_threshold: default_urgent_rebalance_threshold(),
            compaction: CompactionConfig::default(),
            memory: MemoryConfig::default(),
            session: SessionConfig::default(),
            socrates_policy: None,
            socrates_reputation_weight: default_socrates_reputation_weight(),
            populi_control_url: None,
            populi_scope_id: None,
            populi_poll_interval_secs: default_populi_poll_interval_secs(),
            populi_http_timeout_ms: default_populi_http_timeout_ms(),
            populi_routing_experimental: default_false(),
            populi_remote_execute_experimental: default_false(),
            chatml_strict: default_false(),
            planning_enabled: default_false(),
            planning_router_enabled: default_false(),
            planning_replan_enabled: default_false(),
            planning_workflow_handoff_enabled: default_false(),
            planning_shadow_mode: default_false(),
            planning_auto_mode_enabled: default_false(),
            planning_rollout_percent: 0,
            // Phase 15: Attention budget
            attention_enabled: false,
            attention_budget_ms: default_attention_budget_ms(),
            attention_alert_threshold: default_attention_alert_threshold(),
            attention_interrupt_cost_ms: default_attention_interrupt_cost_ms(),
            trust_ewma_alpha: default_trust_ewma_alpha(),
            trust_provisional_threshold: default_trust_provisional_threshold(),
            trust_trusted_threshold: default_trust_trusted_threshold(),
            trust_auto_approve_min: default_trust_auto_approve_min(),
            attention_trust_routing_weight: default_attention_trust_routing_weight(),
            attention_tlx_weights: crate::attention::NasaTlxWeights::default(),
            tier_gate: crate::attention::TierGateConfig::default(),
            news: NewsConfig::default(),
        }
    }
}

impl OrchestratorConfig {
    /// Effective Socrates policy for gates and MCP tools (workspace default + optional overrides).
    #[must_use]
    pub fn effective_socrates_policy(&self) -> ConfidencePolicy {
        let base = ConfidencePolicy::workspace_default();
        match &self.socrates_policy {
            Some(o) => base.with_overrides(o),
            None => base,
        }
    }

    /// Load configuration from a TOML file.
    ///
    /// Looks for an `[orchestrator]` section in the given file.
    /// Returns the default config if the section is missing.
    pub fn load_from_toml(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path).map_err(ConfigError::Io)?;
        let table: toml::Table = toml::from_str(&content).map_err(ConfigError::Parse)?;

        let mut config = if let Some(section) = table.get("orchestrator") {
            let section_str = toml::to_string(section).map_err(ConfigError::Serialize)?;
            toml::from_str(&section_str).map_err(ConfigError::Parse)?
        } else {
            Self::default()
        };

        match vox_repository::read_vox_populi_toml(path) {
            Ok(Some(mens)) => apply_vox_populi_toml(&mut config, &mens),
            Ok(None) => {}
            Err(e) => tracing::warn!("Vox.toml [mens] ignored (parse error): {e}"),
        }

        Ok(config)
    }

    /// Override configuration values from `VOX_ORCHESTRATOR_*` environment variables.
    /// Logs a warning when an env value fails to parse; invalid values are ignored.
    pub fn merge_env_overrides(&mut self) {
        fn parse_or_warn<T: std::str::FromStr>(key: &str, val: &str, default: T) -> T {
            val.parse().unwrap_or_else(|_| {
                tracing::warn!("{}: invalid value '{}', using default", key, val);
                default
            })
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_ENABLED") {
            self.enabled = parse_or_warn("VOX_ORCHESTRATOR_ENABLED", &val, self.enabled);
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_MAX_AGENTS") {
            self.max_agents = parse_or_warn("VOX_ORCHESTRATOR_MAX_AGENTS", &val, self.max_agents);
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_LOCK_TIMEOUT_MS") {
            self.lock_timeout_ms = parse_or_warn(
                "VOX_ORCHESTRATOR_LOCK_TIMEOUT_MS",
                &val,
                self.lock_timeout_ms,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_TOESTUB_GATE") {
            self.toestub_gate =
                parse_or_warn("VOX_ORCHESTRATOR_TOESTUB_GATE", &val, self.toestub_gate);
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_MAX_DEBUG_ITERATIONS") {
            self.max_debug_iterations = parse_or_warn(
                "VOX_ORCHESTRATOR_MAX_DEBUG_ITERATIONS",
                &val,
                self.max_debug_iterations,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_SOCRATES_GATE_SHADOW") {
            self.socrates_gate_shadow = parse_or_warn(
                "VOX_ORCHESTRATOR_SOCRATES_GATE_SHADOW",
                &val,
                self.socrates_gate_shadow,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_SOCRATES_GATE_ENFORCE") {
            self.socrates_gate_enforce = parse_or_warn(
                "VOX_ORCHESTRATOR_SOCRATES_GATE_ENFORCE",
                &val,
                self.socrates_gate_enforce,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_SOCRATES_REPUTATION_ROUTING") {
            self.socrates_reputation_routing = parse_or_warn(
                "VOX_ORCHESTRATOR_SOCRATES_REPUTATION_ROUTING",
                &val,
                self.socrates_reputation_routing,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_SOCRATES_REPUTATION_WEIGHT") {
            self.socrates_reputation_weight = parse_or_warn(
                "VOX_ORCHESTRATOR_SOCRATES_REPUTATION_WEIGHT",
                &val,
                self.socrates_reputation_weight,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_LOG_LEVEL") {
            self.log_level = val;
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_FALLBACK_SINGLE") {
            self.fallback_to_single_agent = parse_or_warn(
                "VOX_ORCHESTRATOR_FALLBACK_SINGLE",
                &val,
                self.fallback_to_single_agent,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_MIN_AGENTS") {
            self.min_agents = parse_or_warn("VOX_ORCHESTRATOR_MIN_AGENTS", &val, self.min_agents);
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_SCALING_THRESHOLD") {
            self.scaling_threshold = parse_or_warn(
                "VOX_ORCHESTRATOR_SCALING_THRESHOLD",
                &val,
                self.scaling_threshold,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_IDLE_RETIREMENT_MS") {
            self.idle_retirement_ms = parse_or_warn(
                "VOX_ORCHESTRATOR_IDLE_RETIREMENT_MS",
                &val,
                self.idle_retirement_ms,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_SCALING_ENABLED") {
            self.scaling_enabled = parse_or_warn(
                "VOX_ORCHESTRATOR_SCALING_ENABLED",
                &val,
                self.scaling_enabled,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_COST_PREFERENCE") {
            match val.to_lowercase().as_str() {
                "performance" => self.cost_preference = CostPreference::Performance,
                "economy" => self.cost_preference = CostPreference::Economy,
                _ => tracing::warn!(
                    "VOX_ORCHESTRATOR_COST_PREFERENCE: invalid value '{}', expected 'performance' or 'economy'",
                    val
                ),
            }
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_SCALING_LOOKBACK") {
            self.scaling_lookback_ticks = parse_or_warn(
                "VOX_ORCHESTRATOR_SCALING_LOOKBACK",
                &val,
                self.scaling_lookback_ticks,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_RESOURCE_WEIGHT") {
            self.resource_weight = parse_or_warn(
                "VOX_ORCHESTRATOR_RESOURCE_WEIGHT",
                &val,
                self.resource_weight,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_RESOURCE_CPU_MULT") {
            self.resource_cpu_multiplier = parse_or_warn(
                "VOX_ORCHESTRATOR_RESOURCE_CPU_MULT",
                &val,
                self.resource_cpu_multiplier,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_RESOURCE_MEM_MULT") {
            self.resource_mem_multiplier = parse_or_warn(
                "VOX_ORCHESTRATOR_RESOURCE_MEM_MULT",
                &val,
                self.resource_mem_multiplier,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_RESOURCE_EXPONENT") {
            self.resource_exponent = parse_or_warn(
                "VOX_ORCHESTRATOR_RESOURCE_EXPONENT",
                &val,
                self.resource_exponent,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_SCALING_PROFILE") {
            match val.to_lowercase().as_str() {
                "conservative" => self.scaling_profile = ScalingProfile::Conservative,
                "balanced" => self.scaling_profile = ScalingProfile::Balanced,
                "aggressive" => self.scaling_profile = ScalingProfile::Aggressive,
                _ => tracing::warn!(
                    "VOX_ORCHESTRATOR_SCALING_PROFILE: invalid value '{}', expected conservative|balanced|aggressive",
                    val
                ),
            }
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_MAX_SPAWN_PER_TICK") {
            self.max_spawn_per_tick = parse_or_warn(
                "VOX_ORCHESTRATOR_MAX_SPAWN_PER_TICK",
                &val,
                self.max_spawn_per_tick,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_SCALING_COOLDOWN_MS") {
            self.scaling_cooldown_ms = parse_or_warn(
                "VOX_ORCHESTRATOR_SCALING_COOLDOWN_MS",
                &val,
                self.scaling_cooldown_ms,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_URGENT_REBALANCE_THRESHOLD") {
            self.urgent_rebalance_threshold = parse_or_warn(
                "VOX_ORCHESTRATOR_URGENT_REBALANCE_THRESHOLD",
                &val,
                self.urgent_rebalance_threshold,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_MIGRATION_V2_ENABLED") {
            self.orchestration_migration.orchestration_v2_enabled = parse_or_warn(
                "VOX_ORCHESTRATOR_MIGRATION_V2_ENABLED",
                &val,
                self.orchestration_migration.orchestration_v2_enabled,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_MIGRATION_LEGACY_FALLBACK") {
            self.orchestration_migration.legacy_orchestration_fallback = parse_or_warn(
                "VOX_ORCHESTRATOR_MIGRATION_LEGACY_FALLBACK",
                &val,
                self.orchestration_migration.legacy_orchestration_fallback,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_MESH_CONTROL_URL") {
            let v = val.trim();
            if v.is_empty() {
                self.populi_control_url = None;
            } else {
                self.populi_control_url = Some(v.to_string());
            }
        } else if let Ok(val) = std::env::var("VOX_MESH_CONTROL_ADDR") {
            let v = val.trim();
            if v.is_empty() {
                self.populi_control_url = None;
            } else {
                self.populi_control_url = Some(v.to_string());
            }
        }
        if let Ok(val) = std::env::var("VOX_MESH_SCOPE_ID") {
            let v = val.trim();
            if v.is_empty() {
                self.populi_scope_id = None;
            } else {
                self.populi_scope_id = Some(v.to_string());
            }
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_MESH_POLL_INTERVAL_SECS") {
            self.populi_poll_interval_secs = parse_or_warn(
                "VOX_ORCHESTRATOR_MESH_POLL_INTERVAL_SECS",
                &val,
                self.populi_poll_interval_secs,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_MESH_HTTP_TIMEOUT_MS") {
            self.populi_http_timeout_ms = parse_or_warn(
                "VOX_ORCHESTRATOR_MESH_HTTP_TIMEOUT_MS",
                &val,
                self.populi_http_timeout_ms,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_MESH_ROUTING_EXPERIMENTAL") {
            self.populi_routing_experimental = parse_or_warn(
                "VOX_ORCHESTRATOR_MESH_ROUTING_EXPERIMENTAL",
                &val,
                self.populi_routing_experimental,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_MESH_REMOTE_EXECUTE_EXPERIMENTAL") {
            self.populi_remote_execute_experimental = parse_or_warn(
                "VOX_ORCHESTRATOR_MESH_REMOTE_EXECUTE_EXPERIMENTAL",
                &val,
                self.populi_remote_execute_experimental,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_CHATML_STRICT") {
            self.chatml_strict =
                parse_or_warn("VOX_ORCHESTRATOR_CHATML_STRICT", &val, self.chatml_strict);
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_PLANNING_ENABLED") {
            self.planning_enabled = parse_or_warn(
                "VOX_ORCHESTRATOR_PLANNING_ENABLED",
                &val,
                self.planning_enabled,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_PLANNING_ROUTER_ENABLED") {
            self.planning_router_enabled = parse_or_warn(
                "VOX_ORCHESTRATOR_PLANNING_ROUTER_ENABLED",
                &val,
                self.planning_router_enabled,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_PLANNING_REPLAN_ENABLED") {
            self.planning_replan_enabled = parse_or_warn(
                "VOX_ORCHESTRATOR_PLANNING_REPLAN_ENABLED",
                &val,
                self.planning_replan_enabled,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_PLANNING_WORKFLOW_HANDOFF_ENABLED") {
            self.planning_workflow_handoff_enabled = parse_or_warn(
                "VOX_ORCHESTRATOR_PLANNING_WORKFLOW_HANDOFF_ENABLED",
                &val,
                self.planning_workflow_handoff_enabled,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_PLANNING_SHADOW_MODE") {
            self.planning_shadow_mode = parse_or_warn(
                "VOX_ORCHESTRATOR_PLANNING_SHADOW_MODE",
                &val,
                self.planning_shadow_mode,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_PLANNING_AUTO_MODE_ENABLED") {
            self.planning_auto_mode_enabled = parse_or_warn(
                "VOX_ORCHESTRATOR_PLANNING_AUTO_MODE_ENABLED",
                &val,
                self.planning_auto_mode_enabled,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_PLANNING_ROLLOUT_PERCENT") {
            self.planning_rollout_percent = parse_or_warn(
                "VOX_ORCHESTRATOR_PLANNING_ROLLOUT_PERCENT",
                &val,
                self.planning_rollout_percent,
            );
        }
        // Phase 15: Attention Budget env overrides
        if let Ok(v) = std::env::var("VOX_ORCHESTRATOR_ATTENTION_ENABLED") {
            self.attention_enabled = parse_or_warn(
                "VOX_ORCHESTRATOR_ATTENTION_ENABLED",
                &v,
                self.attention_enabled,
            );
        }
        if let Ok(v) = std::env::var("VOX_ORCHESTRATOR_ATTENTION_BUDGET_MS") {
            self.attention_budget_ms = parse_or_warn(
                "VOX_ORCHESTRATOR_ATTENTION_BUDGET_MS",
                &v,
                self.attention_budget_ms,
            );
        }
        if let Ok(v) = std::env::var("VOX_ORCHESTRATOR_ATTENTION_ALERT_THRESHOLD") {
            self.attention_alert_threshold = parse_or_warn(
                "VOX_ORCHESTRATOR_ATTENTION_ALERT_THRESHOLD",
                &v,
                self.attention_alert_threshold,
            );
        }
        if let Ok(v) = std::env::var("VOX_ORCHESTRATOR_ATTENTION_INTERRUPT_COST_MS") {
            self.attention_interrupt_cost_ms = parse_or_warn(
                "VOX_ORCHESTRATOR_ATTENTION_INTERRUPT_COST_MS",
                &v,
                self.attention_interrupt_cost_ms,
            );
        }
        if let Ok(v) = std::env::var("VOX_ORCHESTRATOR_TRUST_EWMA_ALPHA") {
            self.trust_ewma_alpha = parse_or_warn(
                "VOX_ORCHESTRATOR_TRUST_EWMA_ALPHA",
                &v,
                self.trust_ewma_alpha,
            );
        }
        if let Ok(v) = std::env::var("VOX_ORCHESTRATOR_TRUST_PROVISIONAL_THRESHOLD") {
            self.trust_provisional_threshold = parse_or_warn(
                "VOX_ORCHESTRATOR_TRUST_PROVISIONAL_THRESHOLD",
                &v,
                self.trust_provisional_threshold,
            );
        }
        if let Ok(v) = std::env::var("VOX_ORCHESTRATOR_TRUST_TRUSTED_THRESHOLD") {
            self.trust_trusted_threshold = parse_or_warn(
                "VOX_ORCHESTRATOR_TRUST_TRUSTED_THRESHOLD",
                &v,
                self.trust_trusted_threshold,
            );
        }
        if let Ok(v) = std::env::var("VOX_ORCHESTRATOR_TRUST_AUTO_APPROVE_MIN") {
            self.trust_auto_approve_min = parse_or_warn(
                "VOX_ORCHESTRATOR_TRUST_AUTO_APPROVE_MIN",
                &v,
                self.trust_auto_approve_min,
            );
        }
        if let Ok(v) = std::env::var("VOX_ORCHESTRATOR_ATTENTION_TRUST_ROUTING_WEIGHT") {
            self.attention_trust_routing_weight = parse_or_warn(
                "VOX_ORCHESTRATOR_ATTENTION_TRUST_ROUTING_WEIGHT",
                &v,
                self.attention_trust_routing_weight,
            );
        }
        // News syndication (see docs/architecture/news_syndication_security.md)
        if let Ok(v) = std::env::var("VOX_NEWS_PUBLISH_ARMED") {
            self.news.publish_armed =
                parse_or_warn("VOX_NEWS_PUBLISH_ARMED", &v, self.news.publish_armed);
        }
        if let Ok(v) = std::env::var("VOX_NEWS_SITE_BASE_URL") {
            let t = v.trim();
            if t.is_empty() {
                self.news.site_base_url = None;
            } else {
                self.news.site_base_url = Some(t.to_string());
            }
        }
        if let Ok(v) = std::env::var("VOX_NEWS_RSS_FEED_PATH") {
            let t = v.trim();
            if t.is_empty() {
                self.news.rss_feed_path = None;
            } else {
                self.news.rss_feed_path = Some(t.to_string());
            }
        }
        if let Ok(v) = std::env::var("VOX_NEWS_SCAN_RECURSIVE") {
            self.news.scan_recursive =
                parse_or_warn("VOX_NEWS_SCAN_RECURSIVE", &v, self.news.scan_recursive);
        }
        if let Ok(v) = std::env::var("VOX_NEWS_TWITTER_TEXT_CHUNK_MAX") {
            self.news.twitter_text_chunk_max = Some(parse_or_warn(
                "VOX_NEWS_TWITTER_TEXT_CHUNK_MAX",
                &v,
                self.news.twitter_text_chunk_max.unwrap_or(280),
            ));
        }
        if let Ok(v) = std::env::var("VOX_NEWS_TWITTER_TRUNCATION_SUFFIX") {
            let t = v.trim();
            if t.is_empty() {
                self.news.twitter_truncation_suffix = None;
            } else {
                self.news.twitter_truncation_suffix = Some(t.to_string());
            }
        }
    }

    /// Create a config suitable for testing (small limits, fast timeouts).
    pub fn for_testing() -> Self {
        Self {
            max_agents: 4,
            lock_timeout_ms: 1000,
            bulletin_capacity: 16,
            toestub_gate: false,
            ..Default::default()
        }
    }
}

/// A validation error encountered when checking an orchestrator configuration.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigValidationError {
    /// `max_agents` was below one.
    #[error("max_agents must be >= 1 (got {0})")]
    InvalidMaxAgents(usize),
    /// File lock TTL was shorter than the minimum safe window.
    #[error("lock_timeout_ms must be >= 100 (got {0})")]
    InvalidLockTimeout(u64),
    /// Broadcast channel capacity was zero.
    #[error("bulletin_capacity must be >= 1 (got {0})")]
    InvalidBulletinCapacity(usize),
    /// Scaling bounds were inconsistent (`min_agents` > `max_agents`).
    #[error("min_agents ({0}) cannot be greater than max_agents ({1})")]
    InvalidScalingLimits(usize, usize),
    /// Planning toggles are inconsistent.
    #[error("invalid planning configuration: {0}")]
    PlanningInvalid(String),
}

impl OrchestratorConfig {
    /// Validates the configuration against required invariants.
    pub fn validate(&self) -> Result<(), Vec<ConfigValidationError>> {
        let mut errors = Vec::new();

        if self.max_agents < 1 {
            errors.push(ConfigValidationError::InvalidMaxAgents(self.max_agents));
        }
        if self.lock_timeout_ms < 100 {
            errors.push(ConfigValidationError::InvalidLockTimeout(
                self.lock_timeout_ms,
            ));
        }
        if self.bulletin_capacity < 1 {
            errors.push(ConfigValidationError::InvalidBulletinCapacity(
                self.bulletin_capacity,
            ));
        }
        if self.min_agents > self.max_agents {
            errors.push(ConfigValidationError::InvalidScalingLimits(
                self.min_agents,
                self.max_agents,
            ));
        }
        if self.planning_router_enabled && !self.planning_enabled {
            errors.push(ConfigValidationError::PlanningInvalid(
                "planning_router_enabled requires planning_enabled".to_string(),
            ));
        }
        if self.planning_replan_enabled && !self.planning_enabled {
            errors.push(ConfigValidationError::PlanningInvalid(
                "planning_replan_enabled requires planning_enabled".to_string(),
            ));
        }
        if self.planning_workflow_handoff_enabled && !self.planning_enabled {
            errors.push(ConfigValidationError::PlanningInvalid(
                "planning_workflow_handoff_enabled requires planning_enabled".to_string(),
            ));
        }
        if self.planning_rollout_percent > 100 {
            errors.push(ConfigValidationError::PlanningInvalid(
                "planning_rollout_percent must be <= 100".to_string(),
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Errors that can occur loading orchestrator configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Underlying filesystem error while reading or writing config files.
    #[error("I/O error reading config: {0}")]
    Io(#[from] std::io::Error),
    /// TOML syntax or schema mismatch on deserialize.
    #[error("TOML parse error: {0}")]
    Parse(#[from] toml::de::Error),
    /// TOML serialization failed (e.g., when persisting overrides).
    #[error("TOML serialize error: {0}")]
    Serialize(#[from] toml::ser::Error),
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Serializes tests that mutate process environment variables.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn default_config_values() {
        let cfg = OrchestratorConfig::default();
        assert!(cfg.enabled);
        assert_eq!(cfg.max_agents, 8);
        assert_eq!(cfg.default_priority, TaskPriority::Normal);
        assert_eq!(cfg.queue_overflow_strategy, OverflowStrategy::SpawnNewAgent);
        assert_eq!(cfg.lock_timeout_ms, 30_000);
        assert!(cfg.toestub_gate);
        assert!(cfg.fallback_to_single_agent);
        assert_eq!(cfg.min_agents, 1);
        assert!(!cfg.scaling_enabled);
        assert_eq!(cfg.cost_preference, CostPreference::Performance);
    }

    #[test]
    fn config_serialization_roundtrip() {
        let cfg = OrchestratorConfig::default();
        let json = serde_json::to_string(&cfg).expect("serialize");
        let back: OrchestratorConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back.max_agents, cfg.max_agents);
        assert_eq!(back.enabled, cfg.enabled);
    }

    #[test]
    fn test_config_values() {
        let cfg = OrchestratorConfig::for_testing();
        assert_eq!(cfg.max_agents, 4);
        assert_eq!(cfg.lock_timeout_ms, 1000);
        assert!(!cfg.toestub_gate);
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn test_validation_errors() {
        let cfg = OrchestratorConfig {
            max_agents: 0,
            lock_timeout_ms: 50,
            bulletin_capacity: 0,
            ..Default::default()
        };

        let errs = cfg.validate().unwrap_err();
        // max_agents=0, lock_timeout=50, bulletin_capacity=0, AND min_agents(1) > max_agents(0)
        assert_eq!(errs.len(), 4);
        assert!(errs.contains(&ConfigValidationError::InvalidMaxAgents(0)));
        assert!(errs.contains(&ConfigValidationError::InvalidLockTimeout(50)));
        assert!(errs.contains(&ConfigValidationError::InvalidBulletinCapacity(0)));
        assert!(errs.contains(&ConfigValidationError::InvalidScalingLimits(1, 0)));
    }

    #[test]
    fn missing_toml_section_returns_default() {
        // Write a temp TOML without [orchestrator]
        let dir = std::env::temp_dir().join("vox_orch_test");
        std::fs::create_dir_all(&dir).ok();
        let toml_path = dir.join("no_orch.toml");
        std::fs::write(&toml_path, "[package]\nname = \"test\"\n").ok();

        let cfg = OrchestratorConfig::load_from_toml(&toml_path).expect("should load");
        assert_eq!(cfg.max_agents, 8); // default
    }

    #[test]
    fn orchestration_migration_defaults_match_contract() {
        let c = OrchestratorConfig::default();
        assert!(!c.orchestration_migration.orchestration_v2_enabled);
        assert!(c.orchestration_migration.legacy_orchestration_fallback);
    }

    #[test]
    fn orchestration_migration_deserializes_from_toml_fragment() {
        let flags: OrchestrationMigrationFlags = toml::from_str(
            "orchestration_v2_enabled = true\nlegacy_orchestration_fallback = false\n",
        )
        .expect("parse nested [orchestrator.orchestration_migration]-shaped keys");
        assert!(flags.orchestration_v2_enabled);
        assert!(!flags.legacy_orchestration_fallback);
    }

    #[test]
    fn populi_toml_section_merges_into_config() {
        let dir = tempfile::tempdir().expect("tempdir");
        let toml_path = dir.path().join("Vox.toml");
        std::fs::write(
            &toml_path,
            r#"
[orchestrator]
max_agents = 3

[mens]
control_url = "http://mens.example:9847"
scope_id = "unit-scope"
advertise_gpu = true
labels = ["from=toml"]
"#,
        )
        .expect("write");
        let cfg = OrchestratorConfig::load_from_toml(&toml_path).expect("load");
        assert_eq!(cfg.max_agents, 3);
        assert_eq!(
            cfg.populi_control_url.as_deref(),
            Some("http://mens.example:9847")
        );
        assert_eq!(cfg.populi_scope_id.as_deref(), Some("unit-scope"));
        assert!(cfg.default_agent_capabilities.gpu_cuda);
        assert!(
            cfg.default_agent_capabilities
                .labels
                .contains(&"from=toml".to_string())
        );
    }

    #[test]
    #[allow(unsafe_code)] // Rust 2024 requires `unsafe` for process-global env mutation; serialized via `ENV_MUTEX`.
    fn populi_env_overrides_toml_control_url() {
        let _guard = ENV_MUTEX.lock().expect("env test lock");
        const KEY: &str = "VOX_ORCHESTRATOR_MESH_CONTROL_URL";
        let prev = std::env::var(KEY).ok();
        // SAFETY: tests are serialized on `ENV_MUTEX`; we restore `KEY` before releasing the lock.
        unsafe {
            std::env::set_var(KEY, "http://env-wins:7777");
        }

        let dir = tempfile::tempdir().expect("tempdir");
        let toml_path = dir.path().join("Vox.toml");
        std::fs::write(
            &toml_path,
            r#"
[mens]
control_url = "http://toml-loses:8888"
"#,
        )
        .expect("write");

        let mut cfg = OrchestratorConfig::load_from_toml(&toml_path).expect("load");
        assert_eq!(
            cfg.populi_control_url.as_deref(),
            Some("http://toml-loses:8888")
        );
        cfg.merge_env_overrides();
        assert_eq!(
            cfg.populi_control_url.as_deref(),
            Some("http://env-wins:7777")
        );

        unsafe {
            match prev {
                None => std::env::remove_var(KEY),
                Some(v) => std::env::set_var(KEY, v),
            }
        }
    }
}
