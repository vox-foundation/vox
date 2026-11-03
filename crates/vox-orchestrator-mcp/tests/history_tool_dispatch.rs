//! `vox_search_history` dispatches end to end: registry row (receipt ledger fails closed on
//! unknown names) -> dispatch arm -> `history_tools::history` -> vox-graph-reader.

use std::sync::Arc;

use vox_orchestrator_mcp::{ServerState, handle_tool_call, load_config};

// Multi-thread: the dispatch path resolves secrets through `block_in_place`.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn search_history_brief_reaches_the_history_facade() {
    let state = Arc::new(ServerState::new_full(load_config()));
    let raw = handle_tool_call(
        &state,
        "vox_search_history",
        serde_json::json!({ "query": "brief" }),
    )
    .await
    .expect("dispatch ok");
    let v: serde_json::Value =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("not JSON ({e}): {raw}"));
    // `brief` never ingests. It succeeds in a checkout with `main`; without one (e.g. a
    // detached CI clone) it fails inside the facade, which still proves the routing.
    if v["success"] == true {
        assert!(v["data"]["rows"].is_string(), "{raw}");
    } else {
        let err = v["error"].as_str().unwrap_or_default();
        assert!(err.starts_with("history: "), "not a facade error: {raw}");
    }
}
