use super::enums::CostPreference;

pub(super) fn default_heartbeat_interval() -> u64 {
    5_000
}
pub(super) fn default_stale_threshold() -> u64 {
    60_000
}
pub(super) fn default_true() -> bool {
    true
}
pub(super) fn default_continuation_cooldown() -> u64 {
    30_000
}
pub(super) fn default_max_auto_continuations() -> u32 {
    5
}
pub(super) fn default_event_capacity() -> usize {
    1024
}
pub(super) fn default_min_agents() -> usize {
    1
}
pub(super) fn default_scaling_threshold() -> usize {
    5
}
pub(super) fn default_idle_retirement() -> u64 {
    300_000
}
pub(super) fn default_false() -> bool {
    false
}
pub(super) fn default_cost_preference() -> CostPreference {
    CostPreference::Performance
}
pub(super) fn default_lookback_ticks() -> usize {
    5
}
pub(super) fn default_resource_weight() -> f64 {
    0.3
}
pub(super) fn default_cpu_multiplier() -> f64 {
    0.7
}
pub(super) fn default_mem_multiplier() -> f64 {
    0.3
}
pub(super) fn default_resource_exponent() -> f64 {
    1.0
}
pub(super) fn default_max_spawn_per_tick() -> usize {
    1
}
pub(super) fn default_scaling_cooldown_ms() -> u64 {
    5_000
}
pub(super) fn default_urgent_rebalance_threshold() -> usize {
    3
}
pub(super) fn default_max_toestub_debug_iterations() -> u8 {
    3
}
pub(super) fn default_max_socrates_debug_iterations() -> u8 {
    3
}

pub(super) fn default_populi_poll_interval_secs() -> u64 {
    30
}

pub(super) fn default_populi_remote_result_poll_interval_secs() -> u64 {
    5
}

pub(super) fn default_populi_http_timeout_ms() -> u64 {
    10_000
}

pub(super) fn default_populi_training_budget_pressure() -> f64 {
    0.0
}

pub(super) fn default_socrates_reputation_weight() -> f64 {
    1.0
}

pub(super) fn default_idle_timeout() -> u64 {
    600_000
}

pub(super) fn default_task_timeout() -> u64 {
    1_800_000
}

// Phase 15: Attention budget defaults
pub(super) fn default_attention_budget_ms() -> u64 {
    3_600_000
}
pub(super) fn default_attention_alert_threshold() -> f64 {
    0.7
}
pub(super) fn default_attention_interrupt_cost_ms() -> u64 {
    23_250
}
pub(super) fn default_trust_ewma_alpha() -> f64 {
    0.1
}
pub(super) fn default_trust_provisional_threshold() -> u32 {
    5
}
pub(super) fn default_trust_trusted_threshold() -> u32 {
    20
}
pub(super) fn default_trust_auto_approve_min() -> f64 {
    0.85
}
pub(super) fn default_attention_trust_routing_weight() -> f64 {
    2.0
}

/// Routing bonus for shard-role specialization.
///
/// Chosen to sit between baseline reliability blending (1.0) and attention-trust
/// influence (2.0), so specialization influences ties without dominating trust.
pub(super) fn default_repo_shard_specialization_weight() -> f64 {
    1.5
}

/// Penalty multiplier applied for each recent shard validation failure.
///
/// Kept below baseline reliability weight to avoid overreacting to single failures
/// while still steering validation tasks away from unstable agents.
pub(super) fn default_repo_shard_validation_failure_penalty() -> f64 {
    0.8
}

/// Penalty applied to reducer placement when agent is in reducer conflict cooldown.
///
/// Set above trust weight to strongly discourage immediate repeat reducer assignment
/// after merge-conflict churn.
pub(super) fn default_repo_reduce_conflict_cooldown_penalty() -> f64 {
    2.5
}

/// Reducer conflict cooldown window in milliseconds.
///
/// Defaults to the same horizon as idle dynamic retirement so conflict cooling and
/// ephemeral worker retirement operate on consistent time scales.
pub(super) fn default_repo_reduce_conflict_cooldown_ms() -> u64 {
    default_idle_retirement()
}
