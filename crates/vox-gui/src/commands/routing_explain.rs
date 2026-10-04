//! Tauri commands that explain model routing from the selector's own ranking
//! (`vox_orchestrator::models::ranking`). Nothing here recomputes a score.

use serde::Serialize;
use vox_orchestrator::mode::ClutchProfile;
use vox_orchestrator::models::health::{RoutingHealth, check_routing_health};
use vox_orchestrator::models::ranking::{Exclusion, Ranking};
use vox_orchestrator::models::spec::QualitySource;
use vox_orchestrator::models::{RoutingTask, TaskCategory};

/// Candidates shown, best first.
pub const MAX_CANDIDATES: usize = 10;
/// Example ids shown per exclusion reason.
pub const MAX_EXAMPLES: usize = 3;

#[derive(Debug, Serialize, PartialEq)]
pub struct QualityDto {
    pub value: f64,
    /// `benchmark` | `inherited` | `estimate` | `unknown`.
    pub source: &'static str,
    pub index: Option<f32>,
    pub inherited_from: Option<String>,
}

/// Each field is that group's weighted share of `score`; together they sum to it.
#[derive(Debug, Serialize, PartialEq)]
pub struct PartsDto {
    pub quality: f64,
    pub efficiency: f64,
    pub latency: f64,
    /// Availability + context balance + mobile.
    pub other: f64,
    /// Free, off-peak, telemetry, fill-in-middle and VRAM adjustments.
    pub bonuses: f64,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct CandidateDto {
    pub id: String,
    pub provider: String,
    pub tier: String,
    pub is_free: bool,
    /// USD per million output tokens; `None` when a paid model's price is unknown.
    pub price_out_per_m: Option<f64>,
    pub score: f64,
    pub quality: QualityDto,
    pub parts: PartsDto,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct ExclusionGroupDto {
    /// The `Exclusion` kind (`no_provider_key`, `superseded`, …).
    pub reason: String,
    pub count: usize,
    pub examples: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct RouteExplanationDto {
    pub mode: String,
    pub task: String,
    pub complexity: u8,
    pub chosen: Option<String>,
    /// The mode's preferred models were all unavailable; the choice came from the fallback pass.
    pub only_candidate: bool,
    pub total_models: usize,
    pub candidates: Vec<CandidateDto>,
    pub excluded: Vec<ExclusionGroupDto>,
}

/// `codegen` | `research` | `review` | `general` (case-insensitive).
#[must_use]
pub fn task_category_from_label(label: &str) -> Option<TaskCategory> {
    match label.trim().to_ascii_lowercase().as_str() {
        "codegen" => Some(TaskCategory::CodeGen),
        "research" => Some(TaskCategory::Research),
        "review" => Some(TaskCategory::Review),
        "general" => Some(TaskCategory::General),
        _ => None,
    }
}

fn exclusion_kind(e: &Exclusion) -> String {
    serde_json::to_value(e)
        .ok()
        .and_then(|v| v.get("kind").and_then(|k| k.as_str()).map(str::to_string))
        .unwrap_or_else(|| "unknown".into())
}

fn quality_dto(m: &vox_orchestrator::models::ModelSpec, value: f64) -> QualityDto {
    match m.capabilities.quality_prior.as_ref().map(|p| &p.source) {
        Some(QualitySource::Benchmark { index }) => QualityDto {
            value,
            source: "benchmark",
            index: Some(*index),
            inherited_from: None,
        },
        Some(QualitySource::Inherited { index, from }) => QualityDto {
            value,
            source: "inherited",
            index: Some(*index),
            inherited_from: Some(from.clone()),
        },
        Some(QualitySource::Estimate) => QualityDto {
            value,
            source: "estimate",
            index: None,
            inherited_from: None,
        },
        None => QualityDto {
            value,
            source: "unknown",
            index: None,
            inherited_from: None,
        },
    }
}

/// Build the explanation from a ranking. Pure: no registry, no I/O.
#[must_use]
pub fn explanation_dto(
    mode: ClutchProfile,
    task: TaskCategory,
    complexity: u8,
    total_models: usize,
    ranking: &Ranking,
    only_candidate: bool,
) -> RouteExplanationDto {
    let candidates = ranking
        .ranked
        .iter()
        .take(MAX_CANDIDATES)
        .map(|r| {
            let p = &r.parts;
            let share = |a: &vox_orchestrator::models::scoring::AxisPart| {
                f64::from(a.weight) * a.value / p.weight_sum
            };
            let price = r.spec.cost_per_1k_output;
            CandidateDto {
                id: r.spec.id.clone(),
                provider: r.spec.provider.clone(),
                tier: format!("{:?}", r.spec.capabilities.tier),
                is_free: r.spec.is_free,
                price_out_per_m: if r.spec.is_free {
                    Some(0.0)
                } else if price.is_finite() && price > 0.0 {
                    Some(price * 1000.0)
                } else {
                    None
                },
                score: p.total,
                quality: quality_dto(&r.spec, p.quality.value),
                parts: PartsDto {
                    quality: share(&p.quality),
                    efficiency: share(&p.efficiency),
                    latency: share(&p.latency),
                    other: share(&p.availability) + share(&p.balance) + share(&p.mobile),
                    bonuses: p.fill_in_middle
                        + p.free_bonus
                        + p.off_peak_bonus
                        + p.telemetry
                        + p.vram,
                },
            }
        })
        .collect();
    let mut groups: Vec<ExclusionGroupDto> = Vec::new();
    for (id, e) in &ranking.excluded {
        let kind = exclusion_kind(e);
        match groups.iter_mut().find(|g| g.reason == kind) {
            Some(g) => {
                g.count += 1;
                if g.examples.len() < MAX_EXAMPLES {
                    g.examples.push(id.clone());
                }
            }
            None => groups.push(ExclusionGroupDto {
                reason: kind,
                count: 1,
                examples: vec![id.clone()],
            }),
        }
    }
    groups.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.reason.cmp(&b.reason)));
    RouteExplanationDto {
        mode: serde_json::to_value(mode)
            .ok()
            .and_then(|v| v.as_str().map(str::to_string))
            .unwrap_or_default(),
        task: format!("{task:?}").to_ascii_lowercase(),
        complexity,
        chosen: ranking.chosen().map(|m| m.id.clone()),
        only_candidate,
        total_models,
        candidates,
        excluded: groups,
    }
}

/// The explanation for `registry` (pure: no DB, no I/O).
#[must_use]
pub fn explain(
    registry: &vox_orchestrator::models::ModelRegistry,
    clutch: ClutchProfile,
    task_category: TaskCategory,
    complexity: u8,
) -> RouteExplanationDto {
    let mut probe = vox_orchestrator::types::AgentTask::new(
        vox_orchestrator::types::TaskId(0),
        "routing explanation",
        vox_orchestrator::types::TaskPriority::Normal,
        vec![],
    );
    probe.task_category = task_category;
    probe.estimated_complexity = complexity;
    let resolved = clutch.resolve();
    // Dispatch's own rules (runtime.rs), minus the two that need the running orchestrator's live state
    // (exploration spend, provider usage); the panel says so.
    let gate = vox_orchestrator::models::mode_select::DispatchGate {
        force_free_pool: resolved.force_free_pool,
        ..Default::default()
    };
    let routing_probe = RoutingTask::from(&probe);
    match registry.best_for_task_under_gate(&routing_probe, resolved.cost_preference, clutch, &gate)
    {
        Some(sel) => explanation_dto(
            clutch,
            task_category,
            complexity,
            registry.list_models().len(),
            &sel.ranking,
            sel.only_candidate,
        ),
        None => {
            // Dispatch would choose nothing (no key, or every model gated): show why, but name no choice.
            let ranking =
                registry.rank_task_with_filter(&routing_probe, resolved.cost_preference, |m| {
                    gate.exclusion(m)
                });
            let mut dto = explanation_dto(
                clutch,
                task_category,
                complexity,
                registry.list_models().len(),
                &ranking,
                false,
            );
            dto.chosen = None;
            dto
        }
    }
}

/// How task dispatch would choose now for `mode` / `task` / `complexity` (1–10). Not chat: chat resolves
/// through `models::select::decide` (premium aliases, confidence gating, per-request axes).
#[tauri::command]
pub async fn explain_routing(
    task: String,
    mode: String,
    complexity: u8,
) -> Result<RouteExplanationDto, String> {
    let clutch = ClutchProfile::from_label(&mode).ok_or("unknown mode")?;
    let task_category = task_category_from_label(&task).ok_or("unknown task")?;
    let registry = super::models::registry_with_scoreboard().await;
    Ok(explain(
        &registry,
        clutch,
        task_category,
        complexity.clamp(1, 10),
    ))
}

/// The registry's routing health, checked now.
#[tauri::command]
pub async fn get_routing_health() -> Result<RoutingHealth, String> {
    let registry = super::models::registry_with_scoreboard().await;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    Ok(check_routing_health(&registry, now))
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_orchestrator::models::ranking::{Exclusion, RankedModel, Ranking};
    use vox_orchestrator::models::scoring::auto_score_parts;
    use vox_orchestrator::models::spec::{PricingSource, QualityPrior, QualitySource};
    use vox_orchestrator::models::{
        ModelCapabilities, ModelSpec, ModelTier, ProviderType, StrengthTag,
    };

    fn spec(id: &str, out_per_1k: f64, source: QualitySource) -> ModelSpec {
        ModelSpec {
            id: id.into(),
            canonical_slug: id.into(),
            provider: "acme".into(),
            provider_type: ProviderType::OpenRouter,
            max_tokens: 64_000,
            cost_per_1k: out_per_1k,
            cost_per_1k_input: out_per_1k,
            cost_per_1k_output: out_per_1k,
            is_free: false,
            observed_cost_per_1k: None,
            strengths: vec![StrengthTag::Generalist],
            capabilities: ModelCapabilities {
                tier: ModelTier::Pro,
                quality_prior: Some(QualityPrior { value: 0.5, source }),
                ..Default::default()
            },
            cache_creation_cost_per_1k: 0.0,
            cache_read_cost_per_1k: 0.0,
            supports_prompt_caching: false,
            pricing_source: PricingSource::OpenRouter,
            supported_parameters: vec![],
        }
    }

    fn ranked(m: ModelSpec) -> RankedModel {
        let parts = auto_score_parts(
            &m,
            5,
            false,
            None,
            vox_orchestrator::config::CostPreference::Economy,
            None,
            None,
        );
        RankedModel { spec: m, parts }
    }

    fn ranking() -> Ranking {
        let mut ranked_models: Vec<RankedModel> = (0..12)
            .map(|i| ranked(spec(&format!("acme/m-{i}"), 0.001, QualitySource::Estimate)))
            .collect();
        ranked_models[0] = ranked(spec(
            "acme/bench",
            0.0009,
            QualitySource::Benchmark { index: 46.3 },
        ));
        ranked_models[1] = ranked(spec(
            "acme/heir",
            0.0,
            QualitySource::Inherited {
                index: 38.2,
                from: "acme/widget-5.0".into(),
            },
        ));
        Ranking {
            ranked: ranked_models,
            // Superseded first, so an unsorted grouping comes out in the wrong order (R13).
            excluded: vec![
                (
                    "acme/old".into(),
                    Exclusion::Superseded {
                        by: "acme/new".into(),
                    },
                ),
                ("acme/x1".into(), Exclusion::NoProviderKey),
                ("acme/x2".into(), Exclusion::NoProviderKey),
                ("acme/x3".into(), Exclusion::NoProviderKey),
                ("acme/x4".into(), Exclusion::NoProviderKey),
            ],
        }
    }

    #[test]
    fn the_explanation_is_built_from_the_ranking() {
        let dto = explanation_dto(
            ClutchProfile::Efficiency,
            TaskCategory::CodeGen,
            7,
            99,
            &ranking(),
            false,
        );
        assert_eq!(dto.mode, "efficiency");
        assert_eq!(dto.task, "codegen");
        assert_eq!(dto.chosen.as_deref(), Some("acme/bench"));
        assert_eq!(dto.candidates.len(), MAX_CANDIDATES);
        assert_eq!(dto.total_models, 99);
    }

    #[test]
    fn every_number_carries_its_provenance() {
        let dto = explanation_dto(
            ClutchProfile::Balanced,
            TaskCategory::General,
            5,
            12,
            &ranking(),
            false,
        );
        let bench = &dto.candidates[0];
        assert_eq!(bench.quality.source, "benchmark");
        assert_eq!(bench.quality.index, Some(46.3));
        assert!((bench.price_out_per_m.unwrap() - 0.9).abs() < 1e-9);
        let heir = &dto.candidates[1];
        assert_eq!(heir.quality.source, "inherited");
        assert_eq!(
            heir.quality.inherited_from.as_deref(),
            Some("acme/widget-5.0")
        );
        assert_eq!(
            heir.price_out_per_m, None,
            "an unknown paid price is None, never 0"
        );
        assert_eq!(dto.candidates[2].quality.source, "estimate");
    }

    #[test]
    fn the_parts_add_up_to_the_score() {
        let dto = explanation_dto(
            ClutchProfile::Genius,
            TaskCategory::CodeGen,
            5,
            12,
            &ranking(),
            false,
        );
        for c in &dto.candidates {
            let sum = c.parts.quality
                + c.parts.efficiency
                + c.parts.latency
                + c.parts.other
                + c.parts.bonuses;
            assert!(
                (sum - c.score).abs() < 1e-9,
                "{}: {sum} vs {}",
                c.id,
                c.score
            );
        }
    }

    #[test]
    fn exclusions_are_grouped_by_reason_largest_first_with_few_examples() {
        let dto = explanation_dto(
            ClutchProfile::Efficiency,
            TaskCategory::CodeGen,
            5,
            12,
            &ranking(),
            false,
        );
        assert_eq!(dto.excluded[0].reason, "no_provider_key");
        assert_eq!(dto.excluded[0].count, 4);
        assert_eq!(dto.excluded[0].examples.len(), MAX_EXAMPLES);
        assert_eq!(dto.excluded[1].reason, "superseded");
        assert_eq!(dto.excluded[1].examples, vec!["acme/old".to_string()]);
    }

    #[test]
    fn free_mode_explains_a_free_choice_and_why_paid_models_are_out() {
        let mut reg = vox_orchestrator::models::ModelRegistry::default();
        let mut paid = spec("acme/paid", 0.001, QualitySource::Estimate);
        paid.provider_type = ProviderType::Ollama;
        let mut free = spec("acme/free", 0.0, QualitySource::Estimate);
        free.provider_type = ProviderType::Ollama;
        free.is_free = true;
        reg.register(paid);
        reg.register(free);
        let dto = explain(&reg, ClutchProfile::Free, TaskCategory::General, 5);
        assert_eq!(dto.chosen.as_deref(), Some("acme/free"));
        assert!(
            dto.excluded
                .iter()
                .any(|g| g.reason == "not_free" && g.examples == vec!["acme/paid".to_string()]),
            "{:?}",
            dto.excluded
        );
    }

    #[test]
    fn when_dispatch_would_choose_nothing_no_model_is_named() {
        let mut reg = vox_orchestrator::models::ModelRegistry::default();
        reg.register(spec("acme/keyless", 0.001, QualitySource::Estimate)); // OpenRouter
        vox_orchestrator::models::key_guard::set_test_key_availability(Some(vec![]));
        let dto = explain(&reg, ClutchProfile::Efficiency, TaskCategory::General, 5);
        vox_orchestrator::models::key_guard::set_test_key_availability(None);
        assert_eq!(dto.chosen, None);
        assert_eq!(
            dto.candidates.first().map(|c| c.id.as_str()),
            Some("acme/keyless"),
            "still shown, not chosen"
        );
    }

    #[test]
    fn labels_parse_and_unknown_labels_are_rejected() {
        assert_eq!(
            task_category_from_label("CodeGen"),
            Some(TaskCategory::CodeGen)
        );
        assert_eq!(
            task_category_from_label("general"),
            Some(TaskCategory::General)
        );
        assert_eq!(task_category_from_label("nope"), None);
    }
}
