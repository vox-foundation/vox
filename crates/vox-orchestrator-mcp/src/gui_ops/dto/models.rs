//! Models-surface wire DTOs (moved from `vox-gui/src/commands/models.rs`, plan 07).

use serde::{Deserialize, Serialize};
use vox_config::AutoRoutingPriority;
use vox_orchestrator::models::ModelSpec;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCardDto {
    pub id: String,
    pub provider: String,
    /// Debug-format `ProviderType` (`OpenRouter`, `VoxLocal`) so the picker
    /// can match `inference_provider_status`. `provider` is often the org
    /// prefix (`aion-labs`, `anthropic`), not the backend.
    pub provider_type: String,
    pub tier: String,
    pub cost_per_1k: f64,
    pub max_tokens: u32,
    pub is_free: bool,
    pub latency_p50_ms: Option<u32>,
    pub success_rate: Option<f64>,
    pub quality_score: Option<f64>,
}

pub fn model_spec_to_card(m: &ModelSpec, success_rate: Option<f64>) -> ModelCardDto {
    ModelCardDto {
        id: m.id.clone(),
        provider: m.provider.clone(),
        provider_type: format!("{:?}", m.provider_type),
        tier: format!("{:?}", m.capabilities.tier),
        cost_per_1k: m.cost_per_1k,
        max_tokens: u32::try_from(m.max_tokens).unwrap_or(u32::MAX),
        is_free: m.is_free,
        latency_p50_ms: m.capabilities.latency_p50_ms,
        success_rate,
        // Deliberately not surfaced (Task M0/M2): scoreboard quality is a constant
        // until feedback rows exist — UI renders `—` for null.
        quality_score: None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoutingPriorityDto {
    pub efficiency: u8,
    pub precision: u8,
    pub latency: u8,
    pub availability: u8,
    pub balance: u8,
    pub mobile: u8,
}

impl From<AutoRoutingPriority> for RoutingPriorityDto {
    fn from(p: AutoRoutingPriority) -> Self {
        Self {
            efficiency: p.efficiency,
            precision: p.precision,
            latency: p.latency,
            availability: p.availability,
            balance: p.balance,
            mobile: p.mobile,
        }
    }
}

/// Parse a `VOX_AUTO_ROUTING_PRIORITY`-style CSV (`efficiency=25,precision=30,...`)
/// onto a base priority, leaving any axes not present in the CSV untouched.
pub fn apply_routing_csv(mut base: RoutingPriorityDto, csv: &str) -> RoutingPriorityDto {
    for part in csv.split(',') {
        let mut it = part.splitn(2, '=');
        let key = it.next().map(str::trim).unwrap_or("").to_ascii_lowercase();
        let Ok(parsed) = it.next().map(str::trim).unwrap_or("").parse::<u8>() else {
            continue;
        };
        match key.as_str() {
            "efficiency" | "cost" => base.efficiency = parsed,
            "precision" | "quality" => base.precision = parsed,
            "latency" | "speed" => base.latency = parsed,
            "availability" => base.availability = parsed,
            "balance" => base.balance = parsed,
            "mobile" => base.mobile = parsed,
            _ => {}
        }
    }
    base
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsPreferencesDto {
    pub active_model: Option<String>,
    /// Effective value: server env (`AutoRoutingPriority::from_env`) overlaid with the persisted CSV.
    pub routing_priority: RoutingPriorityDto,
    /// Persisted `SelectionPolicy` JSON, or `{"steps":[]}` when absent.
    pub selection_policy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingSummaryDto {
    pub active_model: Option<String>,
    pub exploration_spent_usd: f64,
    pub exploration_budget_usd: f64,
    pub routing_priority: RoutingPriorityDto,
    pub arm_count: usize,
    pub model_count: usize,
    pub decision_preview: Option<DecisionPreviewDto>,
    /// Family of the preview pick (`models::family::family_key`); `None` without a pick.
    pub family: Option<String>,
    /// `catalog` | `bootstrap` | `local` (`models::provenance::resolved_from`). The GUI shows a
    /// concrete version only for `catalog`.
    pub resolved_from: Option<String>,
    /// `SelectionReason` display text of the preview pick.
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionPreviewDto {
    pub selected_model: String,
    pub discovery_state: String,
    pub alternatives: Vec<String>,
    pub rejection_reasons: Vec<String>,
    pub intelligence_score: f64,
    pub efficiency_score: f64,
    pub latency_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreboardRowDto {
    pub model_id: String,
    pub task_category: String,
    pub strength_tag: String,
    pub n_calls: i64,
    pub success_rate: f64,
    pub p50_latency_ms: Option<i64>,
    pub cost_per_success_usd: Option<f64>,
    pub quality_score: f64,
}

impl From<vox_db::store::types::ModelScoreboardRow> for ScoreboardRowDto {
    fn from(r: vox_db::store::types::ModelScoreboardRow) -> Self {
        Self {
            model_id: r.model_id,
            task_category: r.task_category,
            strength_tag: r.strength_tag,
            n_calls: r.n_calls,
            success_rate: r.success_rate,
            p50_latency_ms: r.p50_latency_ms,
            cost_per_success_usd: r.cost_per_success_usd,
            quality_score: r.quality_score,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderStatusDto {
    /// Debug-format provider name, e.g. "OpenRouter", "Ollama".
    pub provider: String,
    pub key_present: bool,
    pub is_local: bool,
    /// Some(reachable) from the cached local probe; None for cloud providers.
    pub local_reachable: Option<bool>,
    /// Model names the local server reported (empty for cloud providers).
    pub local_models: Vec<String>,
}

/// One-call refresh of the Models surface: cards, routing summary, active model and providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelsSnapshotDto {
    pub cards: Vec<ModelCardDto>,
    pub routing: RoutingSummaryDto,
    pub active_model: Option<String>,
    pub providers: Vec<ProviderStatusDto>,
}

/// The six live routing-priority axes, in display order. Each maps 1:1 to a
/// field of [`RoutingPriorityDto`] / `vox_config::AutoRoutingPriority` and to a
/// CSV key understood by [`apply_routing_csv`].
pub const ROUTING_AXES: [(&str, &str); 6] = [
    ("efficiency", "Efficiency"),
    ("precision", "Precision"),
    ("latency", "Latency"),
    ("availability", "Availability"),
    ("balance", "Balance"),
    ("mobile", "Mobile"),
];

/// One cell of the Matrix "intentions" hex grid, derived from real routing
/// state (no fabricated rows). The shape matches the GUI `Intention` type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingIntentionDto {
    pub id: String,
    pub parent: String,
    pub branch: String,
    pub phase: String,
    pub conf: f64,
    pub note: String,
}

/// Direction of a Matrix Promote/Doubt action on a routing axis.
#[derive(Debug, Clone, Copy)]
pub enum NudgeDirection {
    /// Promote: increase this axis's weight (the orchestrator favors it more).
    Promote,
    /// Doubt: decrease this axis's weight (the orchestrator favors it less).
    Doubt,
}

/// Step size (priority points) applied per Promote/Doubt action.
const NUDGE_STEP: u8 = 5;

pub fn axis_value(p: &RoutingPriorityDto, axis: &str) -> Option<u8> {
    match axis {
        "efficiency" => Some(p.efficiency),
        "precision" => Some(p.precision),
        "latency" => Some(p.latency),
        "availability" => Some(p.availability),
        "balance" => Some(p.balance),
        "mobile" => Some(p.mobile),
        _ => None,
    }
}

pub fn axis_note(axis: &str) -> &'static str {
    match axis {
        "efficiency" => "Bias toward cheaper / lower-cost models.",
        "precision" => "Bias toward higher-quality, more capable models.",
        "latency" => "Bias toward faster (lower p50 latency) models.",
        "availability" => "Bias toward models with higher observed success rate.",
        "balance" => "Even weighting across cost, quality, and speed.",
        "mobile" => "Bias toward on-device / mobile-friendly models.",
        _ => "Routing priority axis.",
    }
}

/// Project the live routing priority onto Matrix hex cells. Confidence is the
/// axis weight normalized to 0..1; the top-weighted axis is `Validated`, the
/// bottom-weighted is `Doubted`, the rest are `Active`.
pub fn priority_to_intentions(p: &RoutingPriorityDto) -> Vec<RoutingIntentionDto> {
    let values: Vec<u8> = ROUTING_AXES
        .iter()
        .map(|(id, _)| axis_value(p, id).unwrap_or(0))
        .collect();
    let max = values.iter().copied().max().unwrap_or(0);
    let min = values.iter().copied().min().unwrap_or(0);
    ROUTING_AXES
        .iter()
        .map(|(id, label)| {
            let v = axis_value(p, id).unwrap_or(0);
            // Distinct max/min so a uniform profile stays Active rather than
            // flipping every cell to Validated+Doubted at once.
            let phase = if max != min && v == max {
                "Validated"
            } else if max != min && v == min {
                "Doubted"
            } else {
                "Active"
            };
            RoutingIntentionDto {
                id: (*id).to_string(),
                parent: "ROUTING-PRIORITY".to_string(),
                branch: (*label).to_string(),
                phase: phase.to_string(),
                conf: f64::from(v) / 100.0,
                note: axis_note(id).to_string(),
            }
        })
        .collect()
}

/// Apply a single Promote/Doubt nudge to one axis, clamped to `0..=100`. An
/// unknown axis is a no-op. Pure — does no I/O.
pub fn nudge_axis(p: &RoutingPriorityDto, axis: &str, dir: NudgeDirection) -> RoutingPriorityDto {
    let mut out = *p;
    let Some(cur) = axis_value(p, axis) else {
        return out;
    };
    let next = match dir {
        NudgeDirection::Promote => cur.saturating_add(NUDGE_STEP).min(100),
        NudgeDirection::Doubt => cur.saturating_sub(NUDGE_STEP),
    };
    match axis {
        "efficiency" => out.efficiency = next,
        "precision" => out.precision = next,
        "latency" => out.latency = next,
        "availability" => out.availability = next,
        "balance" => out.balance = next,
        "mobile" => out.mobile = next,
        _ => {}
    }
    out
}

pub fn priority_to_csv(p: &RoutingPriorityDto) -> String {
    format!(
        "efficiency={},precision={},latency={},availability={},balance={},mobile={}",
        p.efficiency, p.precision, p.latency, p.availability, p.balance, p.mobile
    )
}

/// One model's graded `vox model bench` results over the leaderboard window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchRowDto {
    pub model_id: String,
    pub served_as: Option<String>,
    pub cases: i64,
    pub passed: i64,
    pub mean_score: f64,
    pub error_rate: f64,
    pub p50_ttft_ms: Option<i64>,
    pub p95_latency_ms: Option<i64>,
    pub tokens_per_sec: Option<f64>,
    pub cost_per_solve_usd: Option<f64>,
    pub tokens_per_solve: Option<f64>,
    pub confidence: String,
    pub last_run_ms: i64,
}

/// Axis leaderboard: what each mode routes per benchmarked category, and the bench table behind it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderboardDto {
    pub routes: Vec<vox_orchestrator::models::leaderboard::ModeRoute>,
    pub bench: Vec<BenchRowDto>,
    pub window_days: i64,
}

pub fn bench_row_dto(r: &vox_db::ModelBenchmarkSummaryRow, confidence: &str) -> BenchRowDto {
    BenchRowDto {
        model_id: r.model_id.clone(),
        served_as: r.resolved_model_id.clone().filter(|s| *s != r.model_id),
        cases: r.cases,
        passed: r.passed,
        mean_score: r.mean_score,
        error_rate: if r.cases > 0 {
            r.errors as f64 / r.cases as f64
        } else {
            0.0
        },
        p50_ttft_ms: r.p50_ttft_ms,
        p95_latency_ms: r.p95_latency_ms,
        tokens_per_sec: r.mean_tokens_per_sec,
        cost_per_solve_usd: r.cost_per_solve_usd(),
        tokens_per_solve: r.tokens_per_solve(),
        confidence: confidence.to_string(),
        last_run_ms: r.last_recorded_at_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> RoutingPriorityDto {
        RoutingPriorityDto {
            efficiency: 1,
            precision: 2,
            latency: 3,
            availability: 4,
            balance: 5,
            mobile: 6,
        }
    }

    fn sample_priority() -> RoutingPriorityDto {
        RoutingPriorityDto {
            efficiency: 40,
            precision: 30,
            latency: 10,
            availability: 10,
            balance: 5,
            mobile: 5,
        }
    }

    #[test]
    fn apply_routing_csv_overlays_known_axes_and_aliases_only() {
        let p = apply_routing_csv(base(), "cost=40, quality=30,bogus=9,latency=x,mobile=7");
        assert_eq!(p.efficiency, 40);
        assert_eq!(p.precision, 30);
        assert_eq!(p.latency, 3);
        assert_eq!(p.mobile, 7);
        assert_eq!(p.balance, 5);
    }

    #[test]
    fn preferences_dto_round_trips() {
        let dto = ModelsPreferencesDto {
            active_model: Some("m".into()),
            routing_priority: base(),
            selection_policy: "{}".into(),
        };
        let back: ModelsPreferencesDto =
            serde_json::from_value(serde_json::to_value(&dto).unwrap()).unwrap();
        assert_eq!(back.routing_priority, base());
        assert_eq!(back.active_model.as_deref(), Some("m"));
    }

    #[test]
    fn axes_to_intentions_yields_one_cell_per_axis_with_normalized_confidence() {
        let cells = priority_to_intentions(&sample_priority());
        assert_eq!(cells.len(), ROUTING_AXES.len());
        let eff = cells.iter().find(|c| c.id == "efficiency").unwrap();
        assert_eq!(eff.parent, "ROUTING-PRIORITY");
        assert!((eff.conf - 0.40).abs() < 1e-6, "got {}", eff.conf);
        assert_eq!(eff.branch, "Efficiency");
    }

    #[test]
    fn top_axis_is_validated_bottom_is_doubted() {
        let cells = priority_to_intentions(&sample_priority());
        let eff = cells.iter().find(|c| c.id == "efficiency").unwrap();
        assert_eq!(eff.phase, "Validated");
        let lowest = cells
            .iter()
            .find(|c| c.id == "balance" || c.id == "mobile")
            .unwrap();
        assert_eq!(lowest.phase, "Doubted");
    }

    #[test]
    fn nudge_promote_raises_axis_doubt_lowers_axis_and_clamps() {
        let base = sample_priority();
        let up = nudge_axis(&base, "latency", NudgeDirection::Promote);
        assert!(up.latency > base.latency);
        let down = nudge_axis(&base, "latency", NudgeDirection::Doubt);
        assert!(down.latency < base.latency);
        assert_eq!(up.precision, base.precision);
        let mut maxed = sample_priority();
        maxed.precision = 100;
        let still = nudge_axis(&maxed, "precision", NudgeDirection::Promote);
        assert_eq!(still.precision, 100);
        let mut zeroed = sample_priority();
        zeroed.mobile = 0;
        let still0 = nudge_axis(&zeroed, "mobile", NudgeDirection::Doubt);
        assert_eq!(still0.mobile, 0);
    }

    #[test]
    fn nudge_unknown_axis_is_a_noop() {
        let base = sample_priority();
        assert_eq!(nudge_axis(&base, "nonsense", NudgeDirection::Promote), base);
    }

    #[test]
    fn priority_to_csv_round_trips_through_apply() {
        let p = sample_priority();
        let zero = RoutingPriorityDto {
            efficiency: 0,
            precision: 0,
            latency: 0,
            availability: 0,
            balance: 0,
            mobile: 0,
        };
        assert_eq!(apply_routing_csv(zero, &priority_to_csv(&p)), p);
    }

    #[test]
    fn model_card_dto_serializes_provider_type() {
        let card = ModelCardDto {
            id: "openai/gpt-4o".to_string(),
            provider: "openai".to_string(),
            provider_type: "OpenAi".to_string(),
            tier: "Pro".to_string(),
            cost_per_1k: 0.005,
            max_tokens: 128_000,
            is_free: false,
            latency_p50_ms: Some(450),
            success_rate: Some(0.99),
            quality_score: None,
        };
        let val = serde_json::to_value(&card).expect("serialize");
        assert_eq!(val["provider_type"], "OpenAi");
        assert_eq!(val["id"], "openai/gpt-4o");
        assert_eq!(val["provider"], "openai");
    }

    #[test]
    fn provider_status_dto_serializes_shape_frontend_expects() {
        let dto = ProviderStatusDto {
            provider: "Anthropic".into(),
            key_present: false,
            is_local: false,
            local_reachable: None,
            local_models: vec![],
        };
        let j = serde_json::to_value(&dto).expect("serialize");
        assert_eq!(j["provider"], "Anthropic");
        assert_eq!(j["key_present"], false);
        assert!(j["local_reachable"].is_null());
    }

    #[test]
    fn routing_summary_serializes_absent_provenance_as_null() {
        let dto = RoutingSummaryDto {
            active_model: None,
            exploration_spent_usd: 0.0,
            exploration_budget_usd: 0.0,
            routing_priority: sample_priority(),
            arm_count: 0,
            model_count: 0,
            decision_preview: None,
            family: None,
            resolved_from: None,
            reason: None,
        };
        let v = serde_json::to_value(&dto).expect("serialize");
        for key in ["family", "resolved_from", "reason"] {
            assert!(
                v.get(key).is_some_and(serde_json::Value::is_null),
                "{key} must serialize as null, got {v}"
            );
        }
    }
}
