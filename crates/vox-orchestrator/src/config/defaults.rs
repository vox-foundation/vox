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

pub(super) fn default_populi_http_timeout_ms() -> u64 {
    10_000
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
