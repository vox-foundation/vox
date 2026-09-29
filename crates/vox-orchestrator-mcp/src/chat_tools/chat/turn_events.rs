//! Chat turn events shared with the GUI trace. `contracts/gui/turn-event-kinds.v1.json`
//! is checked from this side (the real producers) and from the GUI side
//! (`crates/vox-gui/ui/src/components/surfaces/Chat/turnEventContract.test.tsx`).

use serde_json::{Value, json};
use vox_orchestrator::mode::ClutchProfile;
use vox_orchestrator::models::ModelSpec;
use vox_orchestrator::models::family::family_key;
use vox_orchestrator::models::provenance::resolved_from;

use super::message::SelectionSource;

/// Longest `reason` echoed into a turn event (it renders in system-styled chrome).
const REASON_CAP: usize = 200;
/// Shown instead of any rationale that names the requested model: a requested or pinned id
/// can come from composer input a model influences, so it is never echoed into chrome.
const FALLBACK_REASON: &str = "Requested model unavailable; fell back to auto-routing";
const STICKY_LOCAL_REASON: &str = "Pinned local model, not yet in the registry";

/// Wire value of a mode: `ClutchProfile`'s serde name and the GUI's `ClutchId`.
fn mode_wire(mode: ClutchProfile) -> &'static str {
    match mode {
        ClutchProfile::Free => "free",
        ClutchProfile::Efficiency => "efficiency",
        ClutchProfile::Balanced => "balanced",
        ClutchProfile::Genius => "genius",
    }
}

/// What each mode promises, in the words the trace shows.
fn mode_objective(mode: ClutchProfile) -> &'static str {
    match mode {
        ClutchProfile::Free => "Zero-priced models only",
        ClutchProfile::Efficiency => {
            "Most quality per dollar; no flagship while another model fits"
        }
        ClutchProfile::Balanced => {
            "Cost, speed and quality weighed evenly; no flagship while another model fits"
        }
        ClutchProfile::Genius => "Highest available intelligence",
    }
}

/// The reason a turn shows. Every source but `AutoRouted` maps to a constant; an auto-routed
/// rationale comes from the scorer (`SelectionReason`) or the free-tier router, except the two
/// `resolve.rs` lines that echo a requested id, which become constants too.
// ponytail: the two prefixes mirror resolve.rs format strings (pinned by a test); if resolve.rs
// ever returns a typed rationale, match on that instead.
pub(crate) fn routing_reason(rationale: Option<&str>, source: SelectionSource) -> String {
    let text = match source {
        SelectionSource::Fallback => FALLBACK_REASON,
        SelectionSource::UserOverride => "Your pick",
        SelectionSource::Global => "Pinned by the global model override",
        SelectionSource::AutoRouted => match rationale.map(str::trim).filter(|r| !r.is_empty()) {
            Some(r) if r.starts_with("Fallback: requested") => FALLBACK_REASON,
            Some(r) if r.starts_with("Sticky VoxLocal:") => STICKY_LOCAL_REASON,
            Some(r) => r,
            None => "No selection reason recorded for this route",
        },
    };
    text.chars().take(REASON_CAP).collect()
}

/// The `routing_decision` turn event (`contracts/gui/turn-event-kinds.v1.json`). `mode` and
/// `objective` appear only when the composer sent a mode: with none, the resolver used its own
/// default axes and the event makes no promise about them.
pub(crate) fn routing_decision_event(
    spec: &ModelSpec,
    mode: Option<ClutchProfile>,
    reason: &str,
) -> Value {
    let mut ev = json!({
        "kind": "routing_decision",
        "family": family_key(&spec.id),
        "resolved_id": spec.id,
        "resolved_from": resolved_from(spec).as_str(),
        "reason": reason.chars().take(REASON_CAP).collect::<String>(),
    });
    if let Some(mode) = mode {
        ev["mode"] = json!(mode_wire(mode));
        ev["objective"] = json!(mode_objective(mode));
    }
    ev
}

/// Test-only spec builder shared by this module's tests and `message`'s turn-trace tests.
#[cfg(test)]
pub(crate) fn test_spec(
    id: &str,
    provider_type: vox_orchestrator::models::ProviderType,
    pricing_source: vox_orchestrator::models::spec::PricingSource,
) -> ModelSpec {
    ModelSpec {
        id: id.into(),
        canonical_slug: id.into(),
        provider: "test".into(),
        provider_type,
        max_tokens: 32_000,
        cost_per_1k: 0.001,
        cost_per_1k_input: 0.001,
        cost_per_1k_output: 0.001,
        is_free: false,
        observed_cost_per_1k: None,
        strengths: vec![vox_orchestrator::models::StrengthTag::Generalist],
        capabilities: vox_orchestrator::models::ModelCapabilities::default(),
        cache_creation_cost_per_1k: 0.0,
        cache_read_cost_per_1k: 0.0,
        supports_prompt_caching: false,
        pricing_source,
        supported_parameters: vec![],
    }
}

#[cfg(test)]
mod routing_tests {
    use std::collections::HashSet;

    use super::*;
    use vox_orchestrator::models::ProviderType;
    use vox_orchestrator::models::spec::PricingSource;

    const HOSTILE: &str = "<img src=x onerror=alert(1)>";

    fn widget(pricing: PricingSource) -> ModelSpec {
        test_spec("acme/widget-5.5", ProviderType::OpenRouter, pricing)
    }

    #[test]
    fn a_catalog_pick_carries_family_provenance_mode_and_objective() {
        let ev = routing_decision_event(
            &widget(PricingSource::OpenRouter),
            Some(ClutchProfile::Efficiency),
            "why",
        );
        assert_eq!(ev["kind"], "routing_decision");
        assert_eq!(ev["family"], "acme/widget");
        assert_eq!(ev["resolved_id"], "acme/widget-5.5");
        assert_eq!(ev["resolved_from"], "catalog");
        assert_eq!(ev["mode"], "efficiency");
        assert_eq!(
            ev["objective"],
            "Most quality per dollar; no flagship while another model fits"
        );
        assert_eq!(ev["reason"], "why");
    }

    #[test]
    fn a_bootstrap_pick_is_marked_bootstrap() {
        let ev = routing_decision_event(&widget(PricingSource::Bootstrap), None, "why");
        assert_eq!(ev["resolved_from"], "bootstrap");
        assert_eq!(ev["family"], "acme/widget");
    }

    #[test]
    fn without_a_composer_mode_there_is_no_mode_or_objective() {
        let ev = routing_decision_event(&widget(PricingSource::OpenRouter), None, "why");
        assert!(ev.get("mode").is_none(), "no mode was sent: {ev}");
        assert!(ev.get("objective").is_none(), "no mode, no promise: {ev}");
    }

    #[test]
    fn every_mode_has_its_serde_wire_value_and_its_own_objective() {
        let modes = [
            ClutchProfile::Free,
            ClutchProfile::Efficiency,
            ClutchProfile::Balanced,
            ClutchProfile::Genius,
        ];
        let spec = widget(PricingSource::OpenRouter);
        let mut objectives = HashSet::new();
        for mode in modes {
            let ev = routing_decision_event(&spec, Some(mode), "why");
            assert_eq!(
                ev["mode"],
                serde_json::to_value(mode).expect("ClutchProfile serializes"),
                "mode must equal ClutchProfile's serde name (the GUI's ClutchId)"
            );
            objectives.insert(ev["objective"].as_str().expect("objective").to_string());
        }
        assert_eq!(objectives.len(), 4, "each mode states its own objective");
    }

    #[test]
    fn an_auto_routed_rationale_is_used_and_capped() {
        assert_eq!(
            routing_reason(
                Some("Chosen by the model scorer"),
                SelectionSource::AutoRouted
            ),
            "Chosen by the model scorer"
        );
        let long = "x".repeat(300);
        assert_eq!(
            routing_reason(Some(&long), SelectionSource::AutoRouted)
                .chars()
                .count(),
            200
        );
        assert_eq!(
            routing_reason(Some("   "), SelectionSource::AutoRouted),
            "No selection reason recorded for this route",
            "a blank rationale is no rationale"
        );
    }

    #[test]
    fn a_hostile_requested_id_is_never_echoed() {
        let fallback = format!("Fallback: requested `{HOSTILE}` is not in the registry");
        let sticky = format!("Sticky VoxLocal: `{HOSTILE}` synthesized (not yet in orch registry)");
        for (rationale, source) in [
            (fallback.as_str(), SelectionSource::AutoRouted),
            (fallback.as_str(), SelectionSource::Fallback),
            (sticky.as_str(), SelectionSource::AutoRouted),
            (sticky.as_str(), SelectionSource::UserOverride),
            (sticky.as_str(), SelectionSource::Global),
        ] {
            let reason = routing_reason(Some(rationale), source);
            assert!(
                !reason.contains("onerror"),
                "{source:?} echoed the requested id: {reason}"
            );
        }
        assert_eq!(
            routing_reason(Some(&fallback), SelectionSource::Fallback),
            "Requested model unavailable; fell back to auto-routing"
        );
    }

    #[test]
    fn without_a_rationale_the_reason_names_the_selection_source() {
        assert_eq!(
            routing_reason(None, SelectionSource::Global),
            "Pinned by the global model override"
        );
        assert_eq!(
            routing_reason(None, SelectionSource::UserOverride),
            "Your pick"
        );
        assert_eq!(
            routing_reason(None, SelectionSource::AutoRouted),
            "No selection reason recorded for this route"
        );
        assert_eq!(
            routing_reason(None, SelectionSource::Fallback),
            "Requested model unavailable; fell back to auto-routing"
        );
    }

    /// The prefix checks above mirror two format strings in resolve.rs; if those are reworded,
    /// revisit `routing_reason` before this test is updated.
    #[test]
    fn resolve_rs_still_formats_the_echoing_rationales_this_module_replaces() {
        let src = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/llm_bridge/model_route_policy/resolve.rs"
        ));
        assert!(src.contains("Fallback: requested `{id}`"));
        assert!(src.contains("Sticky VoxLocal: `{id}`"));
    }

    #[test]
    fn an_over_long_reason_is_capped_in_the_event() {
        let ev = routing_decision_event(
            &widget(PricingSource::OpenRouter),
            Some(ClutchProfile::Genius),
            &"y".repeat(500),
        );
        assert_eq!(ev["reason"].as_str().expect("reason").chars().count(), 200);
    }
}

#[cfg(test)]
mod contract_tests {
    use serde_json::{Value, json};

    use super::super::agent_loop::{receipt_turn_event, turn_event_for_result};
    use crate::dispatch::ToolReceiptOutcome;

    const CONTRACT: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../contracts/gui/turn-event-kinds.v1.json"
    ));

    fn contract() -> Value {
        serde_json::from_str(CONTRACT)
            .expect("contracts/gui/turn-event-kinds.v1.json is valid JSON")
    }

    fn contract_kind_names(c: &Value) -> Vec<String> {
        c["kinds"]
            .as_array()
            .expect("`kinds` array")
            .iter()
            .map(|k| {
                k["kind"]
                    .as_str()
                    .expect("every entry names its kind")
                    .to_string()
            })
            .collect()
    }

    fn sample_receipt() -> ToolReceiptOutcome {
        ToolReceiptOutcome {
            receipt_id: "01920000-aaaa-7bbb-8ccc-000000000001".into(),
            tool_name: "vox_git_status".into(),
            fulfilled: true,
        }
    }

    /// Events Rust emits outside `turn_event_for_result`, one per producer.
    fn standalone_events() -> Vec<Value> {
        vec![receipt_turn_event(&sample_receipt(), true)]
    }

    /// What the real producer emits for each contract kind, from fixed inputs.
    fn produced(kind: &str) -> Option<Value> {
        match kind {
            "skill_activated" => {
                turn_event_for_result("vox_skill_use", &json!({"id": "ponytail"}), "", true)
            }
            "delegation_spawned" => turn_event_for_result(
                "vox_submit_task",
                &json!({}),
                r#"{"success":true,"data":{"task_id":812,"agent_id":3}}"#,
                true,
            ),
            "research_milestone" => turn_event_for_result(
                "vox_deep_research",
                &json!({"query": "SQLite JSONB performance"}),
                r#"{"success":true,"data":{"waves_executed":3,"claims_verified":12,"contradictions_resolved":1}}"#,
                true,
            ),
            "tool_receipt" => Some(receipt_turn_event(&sample_receipt(), true)),
            "receipt_claims" => turn_event_for_result(
                "vox_verify_task_claims",
                &json!({}),
                r#"{"success":true,"data":{"valid":["a","b"],"fabricated":["x"],"unverified":[]}}"#,
                true,
            ),
            _ => None,
        }
    }

    /// Kinds in `emitted` that `listed` lacks.
    fn kinds_missing_from_contract(emitted: &[String], listed: &[String]) -> Vec<String> {
        emitted
            .iter()
            .filter(|k| !listed.contains(k))
            .cloned()
            .collect()
    }

    #[test]
    fn every_contract_example_equals_what_rust_emits() {
        let c = contract();
        for entry in c["kinds"].as_array().expect("`kinds` array") {
            let kind = entry["kind"].as_str().expect("kind");
            assert_eq!(
                entry["example"]["kind"], kind,
                "the {kind} example must carry its own kind"
            );
            if entry.get("status").and_then(Value::as_str) == Some("planned") {
                assert!(
                    produced(kind).is_none(),
                    "{kind} has a Rust producer now: delete its \"status\": \"planned\""
                );
                continue;
            }
            let actual = produced(kind)
                .unwrap_or_else(|| panic!("contract kind {kind} has no Rust producer"));
            assert_eq!(
                actual, entry["example"],
                "the contract example for {kind} differs from what Rust emits"
            );
            for field in entry["required"].as_array().expect("`required` array") {
                let field = field.as_str().expect("field name");
                assert!(
                    actual.get(field).is_some_and(|v| !v.is_null()),
                    "{kind} must always carry required field {field}"
                );
            }
        }
    }

    #[test]
    fn every_kind_rust_can_emit_is_in_the_contract() {
        // A success body carrying every field any producer arm reads, so each arm fires.
        let body = r#"{"success":true,"data":{"agent_id":1,"task_id":2,"valid":[],"fabricated":[],"unverified":[],"waves_executed":1,"claims_verified":0,"contradictions_resolved":0}}"#;
        let args = json!({"id": "ponytail", "query": "q"});
        let mut emitted: Vec<String> = vox_mcp_registry::TOOL_REGISTRY
            .iter()
            .filter_map(|entry| turn_event_for_result(entry.name, &args, body, true))
            .chain(standalone_events())
            .map(|ev| {
                ev["kind"]
                    .as_str()
                    .expect("every event names its kind")
                    .to_string()
            })
            .collect();
        emitted.sort();
        emitted.dedup();
        assert!(
            emitted.len() >= 5,
            "the sweep must reach every producer arm, got {emitted:?}"
        );
        let missing = kinds_missing_from_contract(&emitted, &contract_kind_names(&contract()));
        assert!(
            missing.is_empty(),
            "Rust emits kinds the contract does not list: {missing:?}"
        );
    }

    #[test]
    fn the_missing_kind_check_flags_an_unlisted_kind() {
        let emitted = vec!["tool_receipt".to_string(), "brand_new_kind".to_string()];
        let listed = vec!["tool_receipt".to_string()];
        assert_eq!(
            kinds_missing_from_contract(&emitted, &listed),
            vec!["brand_new_kind".to_string()]
        );
    }
}
