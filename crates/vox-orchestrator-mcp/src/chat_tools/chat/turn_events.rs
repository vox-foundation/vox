//! Chat turn events shared with the GUI trace. `contracts/gui/turn-event-kinds.v1.json`
//! is checked from this side (the real producers) and from the GUI side
//! (`crates/vox-gui/ui/src/components/surfaces/Chat/turnEventContract.test.tsx`).

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
