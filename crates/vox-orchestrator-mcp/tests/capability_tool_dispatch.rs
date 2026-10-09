//! `vox_capability_explain` / `vox_capability_features` dispatch end to end: registry row (the
//! receipt ledger fails closed on unknown names) -> dispatch arm -> `capability_tools` -> the index.

use std::sync::Arc;

use vox_capability_registry::{bundled_index, explain_rows, resolve};
use vox_orchestrator_mcp::{ServerState, handle_tool_call, load_config};

async fn call(name: &str, args: serde_json::Value) -> serde_json::Value {
    // Multi-thread callers: the dispatch path resolves secrets through `block_in_place`.
    let state = Arc::new(ServerState::new_full(load_config()));
    let raw = handle_tool_call(&state, name, args)
        .await
        .expect("dispatch ok");
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("not JSON ({e}): {raw}"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn explain_reaches_the_index() {
    let v = call(
        "vox_capability_explain",
        serde_json::json!({"query": "check"}),
    )
    .await;
    let r = resolve(bundled_index(), "check").unwrap();
    assert_eq!(v["success"], true, "{v}");
    assert_eq!(v["data"], serde_json::to_value(explain_rows(&r)).unwrap());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn features_reaches_the_index() {
    let v = call(
        "vox_capability_features",
        serde_json::json!({"domain": "secrets"}),
    )
    .await;
    let rows = v["data"].as_array().expect("data array");
    assert!(!rows.is_empty(), "{v}");
    assert!(rows.iter().all(|r| r["domain"] == "secrets"), "{v}");
}
