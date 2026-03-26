use super::enums::{CostPreference, ScalingProfile};
use super::orchestrator_fields::OrchestratorConfig;

impl OrchestratorConfig {
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
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_MAX_TOESTUB_DEBUG_ITERATIONS") {
            self.max_toestub_debug_iterations = parse_or_warn(
                "VOX_ORCHESTRATOR_MAX_TOESTUB_DEBUG_ITERATIONS",
                &val,
                self.max_toestub_debug_iterations,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_MAX_SOCRATES_DEBUG_ITERATIONS") {
            self.max_socrates_debug_iterations = parse_or_warn(
                "VOX_ORCHESTRATOR_MAX_SOCRATES_DEBUG_ITERATIONS",
                &val,
                self.max_socrates_debug_iterations,
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
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_MESH_TRAINING_ROUTING_EXPERIMENTAL") {
            self.populi_training_routing_experimental = parse_or_warn(
                "VOX_ORCHESTRATOR_MESH_TRAINING_ROUTING_EXPERIMENTAL",
                &val,
                self.populi_training_routing_experimental,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_MESH_TRAINING_BUDGET_PRESSURE") {
            self.populi_training_budget_pressure = parse_or_warn(
                "VOX_ORCHESTRATOR_MESH_TRAINING_BUDGET_PRESSURE",
                &val,
                self.populi_training_budget_pressure,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_MESH_REMOTE_EXECUTE_EXPERIMENTAL") {
            self.populi_remote_execute_experimental = parse_or_warn(
                "VOX_ORCHESTRATOR_MESH_REMOTE_EXECUTE_EXPERIMENTAL",
                &val,
                self.populi_remote_execute_experimental,
            );
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_MESH_REMOTE_EXECUTE_RECEIVER_AGENT") {
            let t = val.trim();
            if t.is_empty() {
                self.populi_remote_execute_receiver_agent = None;
            } else {
                self.populi_remote_execute_receiver_agent = Some(t.to_string());
            }
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_MESH_REMOTE_EXECUTE_SENDER_AGENT") {
            let t = val.trim();
            if t.is_empty() {
                self.populi_remote_execute_sender_agent = None;
            } else {
                self.populi_remote_execute_sender_agent = Some(t.to_string());
            }
        }
        if let Ok(val) = std::env::var("VOX_ORCHESTRATOR_MESH_REMOTE_RESULT_POLL_INTERVAL_SECS") {
            self.populi_remote_result_poll_interval_secs = parse_or_warn(
                "VOX_ORCHESTRATOR_MESH_REMOTE_RESULT_POLL_INTERVAL_SECS",
                &val,
                self.populi_remote_result_poll_interval_secs,
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
        if let Ok(v) = std::env::var("VOX_ORCHESTRATOR_REPO_SHARD_SPECIALIZATION_WEIGHT") {
            self.repo_shard_specialization_weight = parse_or_warn(
                "VOX_ORCHESTRATOR_REPO_SHARD_SPECIALIZATION_WEIGHT",
                &v,
                self.repo_shard_specialization_weight,
            );
        }
        if let Ok(v) = std::env::var("VOX_ORCHESTRATOR_REPO_SHARD_VALIDATION_FAILURE_PENALTY") {
            self.repo_shard_validation_failure_penalty = parse_or_warn(
                "VOX_ORCHESTRATOR_REPO_SHARD_VALIDATION_FAILURE_PENALTY",
                &v,
                self.repo_shard_validation_failure_penalty,
            );
        }
        if let Ok(v) = std::env::var("VOX_ORCHESTRATOR_REPO_REDUCE_CONFLICT_COOLDOWN_PENALTY") {
            self.repo_reduce_conflict_cooldown_penalty = parse_or_warn(
                "VOX_ORCHESTRATOR_REPO_REDUCE_CONFLICT_COOLDOWN_PENALTY",
                &v,
                self.repo_reduce_conflict_cooldown_penalty,
            );
        }
        if let Ok(v) = std::env::var("VOX_ORCHESTRATOR_REPO_REDUCE_CONFLICT_COOLDOWN_MS") {
            self.repo_reduce_conflict_cooldown_ms = parse_or_warn(
                "VOX_ORCHESTRATOR_REPO_REDUCE_CONFLICT_COOLDOWN_MS",
                &v,
                self.repo_reduce_conflict_cooldown_ms,
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
        if let Ok(v) = std::env::var("VOX_SOCIAL_REDDIT_CLIENT_ID") {
            let t = v.trim();
            self.news.reddit_client_id = if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            };
        }
        if let Ok(v) = std::env::var("VOX_SOCIAL_REDDIT_CLIENT_SECRET") {
            let t = v.trim();
            self.news.reddit_client_secret = if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            };
        }
        if let Ok(v) = std::env::var("VOX_SOCIAL_REDDIT_REFRESH_TOKEN") {
            let t = v.trim();
            self.news.reddit_refresh_token = if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            };
        }
        if let Ok(v) = std::env::var("VOX_SOCIAL_REDDIT_USER_AGENT") {
            let t = v.trim();
            self.news.reddit_user_agent = if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            };
        }
        if let Ok(v) = std::env::var("VOX_SOCIAL_YOUTUBE_CLIENT_ID") {
            let t = v.trim();
            self.news.youtube_client_id = if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            };
        }
        if let Ok(v) = std::env::var("VOX_SOCIAL_YOUTUBE_CLIENT_SECRET") {
            let t = v.trim();
            self.news.youtube_client_secret = if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            };
        }
        if let Ok(v) = std::env::var("VOX_SOCIAL_YOUTUBE_REFRESH_TOKEN") {
            let t = v.trim();
            self.news.youtube_refresh_token = if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            };
        }
        if let Ok(v) = std::env::var("VOX_SOCIAL_HN_MODE") {
            let t = v.trim();
            self.news.hacker_news_mode = if t.is_empty() {
                None
            } else {
                Some(t.to_string())
            };
        }
        if let Ok(v) = std::env::var("VOX_SOCIAL_WORTHINESS_ENFORCE") {
            self.news.worthiness_enforce = parse_or_warn(
                "VOX_SOCIAL_WORTHINESS_ENFORCE",
                &v,
                self.news.worthiness_enforce,
            );
        }
        if let Ok(v) = std::env::var("VOX_SOCIAL_WORTHINESS_SCORE_MIN") {
            self.news.worthiness_score_min = Some(parse_or_warn(
                "VOX_SOCIAL_WORTHINESS_SCORE_MIN",
                &v,
                self.news.worthiness_score_min.unwrap_or(0.85),
            ));
        }
        if let Ok(v) = std::env::var("VOX_SOCIAL_CHANNEL_WORTHINESS_FLOORS") {
            for pair in v.split(',') {
                let trimmed = pair.trim();
                if trimmed.is_empty() {
                    continue;
                }
                let mut parts = trimmed.splitn(2, '=');
                let key = parts.next().unwrap_or("").trim().to_lowercase();
                let val = parts.next().unwrap_or("").trim();
                if key.is_empty() || val.is_empty() {
                    continue;
                }
                let floor = parse_or_warn("VOX_SOCIAL_CHANNEL_WORTHINESS_FLOORS", val, 0.85_f64);
                self.news.channel_worthiness_floors.insert(key, floor);
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
