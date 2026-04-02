use vox_config::AutoRoutingPriority;
use vox_orchestrator::config::CostPreference;
use vox_orchestrator::models::ModelSpec;
use vox_orchestrator::usage::RemainingBudget;

use super::types::McpChatModelResolution;

const QUALITY_FREE_PAID_COMPONENT: f64 = 0.35;
const QUALITY_PAID_COMPONENT: f64 = 0.95;
const QUALITY_TOKEN_WEIGHT: f64 = 0.6;
const QUALITY_PAID_WEIGHT: f64 = 0.4;
const EFFICIENCY_COST_SCALER: f64 = 100.0;
const COMPLEXITY_HIGH_CUTOFF: u8 = 8;
const COMPLEXITY_LOW_CUTOFF: u8 = 3;
const COMPLEXITY_PRECISION_BONUS: u8 = 10;
const COMPLEXITY_EFFICIENCY_BONUS: u8 = 10;
const COMPLEXITY_LATENCY_BONUS: u8 = 5;
const FIM_CODE_SIGNAL_BONUS: f64 = 0.08;
const FIM_NON_CODE_SIGNAL_PENALTY: f64 = -0.02;
const ECONOMY_EFFICIENCY_BONUS: u8 = 15;
const PERFORMANCE_PRECISION_BONUS: u8 = 12;
const RATE_LIMITED_SCORE_FLOOR: f64 = -10_000.0;
const EMPTY_BUDGET_AVAILABILITY_SCORE: f64 = 0.35;
const BUDGET_LOG10_DIVISOR: f64 = 3.0;
const BUDGET_AVAILABILITY_MIN: f64 = 0.4;

#[must_use]
pub(super) fn budget_match(limit_model: &str, model: &str) -> bool {
    limit_model == model
        || limit_model == "*"
        || (limit_model == ":free" && model.ends_with(":free"))
}

#[must_use]
pub(super) fn model_budget_hint(
    model: &ModelSpec,
    hints: Option<&[RemainingBudget]>,
) -> (u32, bool) {
    let usage = model.llm_usage_key();
    let mut remaining_max = 0u32;
    let mut any_rate_limited = false;
    for b in hints.unwrap_or(&[]) {
        if b.provider == usage.provider && budget_match(&b.model, &usage.model) {
            remaining_max = remaining_max.max(b.remaining);
            any_rate_limited |= b.rate_limited;
        }
    }
    (remaining_max, any_rate_limited)
}

#[must_use]
pub(super) fn quality_score(m: &ModelSpec) -> f64 {
    let token_component = (m.max_tokens as f64).log10().clamp(1.0, 7.0) / 7.0;
    let paid_component = if m.is_free {
        QUALITY_FREE_PAID_COMPONENT
    } else {
        QUALITY_PAID_COMPONENT
    };
    ((token_component * QUALITY_TOKEN_WEIGHT) + (paid_component * QUALITY_PAID_WEIGHT))
        .clamp(0.0, 1.0)
}

#[must_use]
pub(super) fn efficiency_score(m: &ModelSpec) -> f64 {
    let blended = if m.cost_per_1k_input > 0.0 || m.cost_per_1k_output > 0.0 {
        (m.cost_per_1k_input + m.cost_per_1k_output) / 2.0
    } else {
        m.cost_per_1k
    };
    if blended <= 0.0 {
        return 1.0;
    }
    (1.0 / (1.0 + blended * EFFICIENCY_COST_SCALER)).clamp(0.0, 1.0)
}

#[must_use]
pub(super) fn latency_score(m: &ModelSpec) -> f64 {
    use vox_orchestrator::models::ProviderType;
    match m.provider_type {
        ProviderType::Ollama => 0.95,
        ProviderType::GoogleDirect => 0.8,
        ProviderType::OpenRouter => 0.7,
        _ => 0.65,
    }
}

#[must_use]
pub(super) fn mobile_score(m: &ModelSpec) -> f64 {
    use vox_orchestrator::models::ProviderType;
    match vox_config::inference_profile_from_env() {
        vox_config::InferenceProfile::MobileLitert | vox_config::InferenceProfile::MobileCoreml => {
            if matches!(m.provider_type, ProviderType::Ollama) {
                0.0
            } else {
                1.0
            }
        }
        _ => 0.7,
    }
}

#[must_use]
pub(super) fn auto_score_model(
    m: &ModelSpec,
    res: &McpChatModelResolution,
    preference: CostPreference,
    hints: Option<&[RemainingBudget]>,
) -> f64 {
    let mut w = AutoRoutingPriority::from_env();
    if res.complexity >= COMPLEXITY_HIGH_CUTOFF {
        w.precision = w.precision.saturating_add(COMPLEXITY_PRECISION_BONUS);
    } else if res.complexity <= COMPLEXITY_LOW_CUTOFF {
        w.efficiency = w.efficiency.saturating_add(COMPLEXITY_EFFICIENCY_BONUS);
        w.latency = w.latency.saturating_add(COMPLEXITY_LATENCY_BONUS);
    }
    let fim_bias = if res.free_tier_fill_in_middle {
        let id = m.id.to_ascii_lowercase();
        let has_code_signal = m.strengths.iter().any(|s| s == "codegen" || s == "parsing")
            || id.contains("coder")
            || id.contains("code")
            || id.contains("instruct");
        if has_code_signal {
            FIM_CODE_SIGNAL_BONUS
        } else {
            FIM_NON_CODE_SIGNAL_PENALTY
        }
    } else {
        0.0
    };
    match preference {
        CostPreference::Economy => {
            w.efficiency = w.efficiency.saturating_add(ECONOMY_EFFICIENCY_BONUS)
        }
        CostPreference::Performance => {
            w.precision = w.precision.saturating_add(PERFORMANCE_PRECISION_BONUS)
        }
    }

    let (remaining, rate_limited) = model_budget_hint(m, hints);
    if rate_limited {
        return RATE_LIMITED_SCORE_FLOOR;
    }

    let balance_bias = 1.0_f64 - f64::from(res.context_fill_ratio.unwrap_or(0.0).clamp(0.0, 1.0));
    let availability_score = if remaining == 0 {
        EMPTY_BUDGET_AVAILABILITY_SCORE
    } else {
        (f64::from(remaining).log10() / BUDGET_LOG10_DIVISOR).clamp(BUDGET_AVAILABILITY_MIN, 1.0)
    };
    let total_w = f64::from(
        u16::from(w.efficiency)
            + u16::from(w.precision)
            + u16::from(w.latency)
            + u16::from(w.availability)
            + u16::from(w.balance)
            + u16::from(w.mobile),
    )
    .max(1.0);
    let score = f64::from(w.efficiency) * efficiency_score(m)
        + f64::from(w.precision) * quality_score(m)
        + f64::from(w.latency) * latency_score(m)
        + f64::from(w.availability) * availability_score
        + f64::from(w.balance) * balance_bias
        + f64::from(w.mobile) * mobile_score(m);
    (score / total_w) + fim_bias
}
