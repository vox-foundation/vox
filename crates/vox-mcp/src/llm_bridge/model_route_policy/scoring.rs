use vox_config::AutoRoutingPriority;
use vox_orchestrator::config::CostPreference;
use vox_orchestrator::models::ModelSpec;
use vox_orchestrator::usage::RemainingBudget;

use super::types::McpChatModelResolution;

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
    let paid_component = if m.is_free { 0.35 } else { 0.95 };
    ((token_component * 0.6) + (paid_component * 0.4)).clamp(0.0, 1.0)
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
    (1.0 / (1.0 + blended * 100.0)).clamp(0.0, 1.0)
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
    if res.complexity >= 8 {
        w.precision = w.precision.saturating_add(10);
    } else if res.complexity <= 3 {
        w.efficiency = w.efficiency.saturating_add(10);
        w.latency = w.latency.saturating_add(5);
    }
    match preference {
        CostPreference::Economy => w.efficiency = w.efficiency.saturating_add(15),
        CostPreference::Performance => w.precision = w.precision.saturating_add(12),
    }

    let (remaining, rate_limited) = model_budget_hint(m, hints);
    if rate_limited {
        return -10_000.0;
    }

    let balance_bias = 1.0_f64 - f64::from(res.context_fill_ratio.unwrap_or(0.0).clamp(0.0, 1.0));
    let availability_score = if remaining == 0 {
        0.35
    } else {
        (f64::from(remaining).log10() / 3.0).clamp(0.4, 1.0)
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
    score / total_w
}
