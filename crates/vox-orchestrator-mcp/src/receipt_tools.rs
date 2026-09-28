//! Phase 5 D-03: read-only verdict over the in-process receipt ledger;
//! detection only — callers decide what a fabricated claim means.

use crate::params::ToolResult;
use crate::server_state::ServerState;
use schemars::JsonSchema;
use serde::Deserialize;

/// Maximum number of receipt IDs allowed in a single claim verification call.
/// UUIDv7 ids are 36 chars; the cap bounds a model-supplied list at the trust boundary.
pub const MAX_RECEIPT_IDS: usize = 256;

/// Maximum length of any single receipt ID string.
/// UUIDv7 ids are 36 chars; the cap bounds a model-supplied list at the trust boundary.
pub const MAX_RECEIPT_ID_CHARS: usize = 128;

/// Parameters for `vox_verify_task_claims`.
#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
pub struct VerifyTaskClaimsParams {
    /// Receipt ids the agent claims to have received from runtime execution.
    #[serde(default)]
    pub receipt_ids: Vec<String>,
}

/// Verify receipt claims against the runtime receipt ledger.
pub async fn verify_task_claims(state: &ServerState, params: VerifyTaskClaimsParams) -> String {
    if params.receipt_ids.len() > MAX_RECEIPT_IDS {
        return ToolResult::<()>::err(format!(
            "Too many receipt IDs: {} exceeds maximum of {}",
            params.receipt_ids.len(),
            MAX_RECEIPT_IDS
        ))
        .to_json();
    }
    if let Some(bad_id) = params
        .receipt_ids
        .iter()
        .find(|id| id.len() > MAX_RECEIPT_ID_CHARS)
    {
        return ToolResult::<()>::err(format!(
            "Receipt ID exceeds maximum length of {} chars: {} chars",
            MAX_RECEIPT_ID_CHARS,
            bad_id.len()
        ))
        .to_json();
    }

    let handle = state.orchestrator.tool_ledger_handle();
    let ledger = vox_orchestrator::sync_lock::rw_read(&*handle);
    let outcome = ledger.validate_agent_claims(&params.receipt_ids);

    ToolResult::ok(serde_json::json!({
        "valid": outcome.valid,
        "fabricated": outcome.fabricated,
        "unverified": outcome.unverified,
    }))
    .to_json()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dispatch::{handle_tool_call, handle_tool_call_with_receipt};
    use crate::server_state::ServerState;
    use serde_json::json;

    #[tokio::test]
    async fn verify_task_claims_splits_real_and_fabricated_ids() {
        let state = ServerState::new_test().await;
        let (res, receipt_opt) =
            handle_tool_call_with_receipt(&state, "vox_skill_list", json!({}), None).await;
        let _ = res.expect("vox_skill_list should succeed");
        let receipt = receipt_opt.expect("receipt should be issued for registered tool");
        let real_id = receipt.receipt_id;

        let res = handle_tool_call(
            &state,
            "vox_verify_task_claims",
            json!({"receipt_ids": [real_id.clone(), "not-a-real-receipt"]}),
        )
        .await
        .expect("handle_tool_call should succeed");

        let val: serde_json::Value = serde_json::from_str(&res).expect("valid json");
        assert_eq!(val["success"], true, "payload: {val:?}");
        let valid = val["data"]["valid"].as_array().expect("valid array");
        assert_eq!(valid, &[serde_json::Value::String(real_id)]);
        let fabricated = val["data"]["fabricated"]
            .as_array()
            .expect("fabricated array");
        assert_eq!(
            fabricated,
            &[serde_json::Value::String("not-a-real-receipt".to_string())]
        );
        let unverified = val["data"]["unverified"]
            .as_array()
            .expect("unverified array");
        assert!(unverified.is_empty());
    }

    #[tokio::test]
    async fn the_verify_tool_gets_its_own_receipt() {
        let state = ServerState::new_test().await;
        let res = handle_tool_call(&state, "vox_verify_task_claims", json!({}))
            .await
            .expect("handle_tool_call should succeed");
        let val: serde_json::Value = serde_json::from_str(&res).expect("valid json");
        assert_eq!(val["success"], true);

        let handle = state.orchestrator.tool_ledger_handle();
        let ledger = vox_orchestrator::sync_lock::rw_read(&*handle);
        let snapshot = ledger.snapshot();
        let found = snapshot
            .values()
            .any(|(_, tool_name)| tool_name == "vox_verify_task_claims");
        assert!(
            found,
            "ledger snapshot should contain entry for vox_verify_task_claims"
        );
    }

    #[tokio::test]
    async fn empty_args_return_an_empty_verdict() {
        let state = ServerState::new_test().await;
        let res = handle_tool_call(&state, "vox_verify_task_claims", json!({}))
            .await
            .expect("handle_tool_call should succeed");
        let val: serde_json::Value = serde_json::from_str(&res).expect("valid json");
        assert_eq!(val["success"], true);
        assert_eq!(val["data"]["valid"], json!([]));
        assert_eq!(val["data"]["fabricated"], json!([]));
        assert_eq!(val["data"]["unverified"], json!([]));
    }

    #[tokio::test]
    async fn oversized_claim_lists_are_rejected() {
        let state = ServerState::new_test().await;
        // 257 ids returns an error envelope
        let ids_257: Vec<String> = (0..=MAX_RECEIPT_IDS).map(|i| format!("id-{i}")).collect();
        let res = handle_tool_call(
            &state,
            "vox_verify_task_claims",
            json!({"receipt_ids": ids_257}),
        )
        .await
        .expect("handle_tool_call returns ok string even for tool error envelope");
        let val: serde_json::Value = serde_json::from_str(&res).expect("valid json");
        assert_eq!(val["success"], false, "oversized list should fail");
        assert!(!val["error"].is_null());

        // one id of 129 chars returns an error envelope
        let long_id = "a".repeat(MAX_RECEIPT_ID_CHARS + 1);
        let res = handle_tool_call(
            &state,
            "vox_verify_task_claims",
            json!({"receipt_ids": [long_id]}),
        )
        .await
        .expect("handle_tool_call returns ok string even for tool error envelope");
        let val: serde_json::Value = serde_json::from_str(&res).expect("valid json");
        assert_eq!(val["success"], false, "oversized id should fail");
        assert!(!val["error"].is_null());
    }
}
