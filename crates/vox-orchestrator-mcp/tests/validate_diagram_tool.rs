//! `vox_validate_diagram` end to end: dispatch arm -> `diagram_tools` -> `vox_diagram::validate`.

use std::sync::Arc;

use serde_json::{Value, json};
use vox_diagram::{ValidateOpts, validate};
use vox_orchestrator_mcp::{ServerState, handle_tool_call, load_config};

const BROKEN: &str = "graph {\n    node a: \"A\"\n    a -> b\n    node c: C\n}\n";

async fn call(name: &str, args: Value) -> Value {
    // Multi-thread callers: the dispatch path resolves secrets through `block_in_place`.
    let state = Arc::new(ServerState::new_full(load_config()));
    let raw = handle_tool_call(&state, name, args)
        .await
        .expect("dispatch ok");
    serde_json::from_str(&raw).unwrap_or_else(|e| panic!("not JSON ({e}): {raw}"))
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tracer_broken_graph_returns_every_diagnostic_with_fixes() {
    let v = call("vox_validate_diagram", json!({ "source": BROKEN })).await;
    assert_eq!(v["success"], true, "{v}");
    let d = &v["data"];
    assert_eq!(d["valid"], false, "{v}");
    assert_eq!(d["schema_version"], 1);
    let diags = d["diagnostics"].as_array().expect("diagnostics array");
    assert!(diags.len() >= 2, "{v}");
    for x in diags {
        assert!(x["code"].as_str().unwrap().starts_with("diagram/"), "{x}");
        for k in ["severity", "message"] {
            assert!(x.get(k).is_some(), "{k} missing in {x}");
        }
        assert!(x["span"]["line"].is_number() && x["span"]["col"].is_number());
    }
    assert!(
        diags
            .iter()
            .any(|x| x["fixes"].as_array().is_some_and(|f| !f.is_empty()
                && f[0].get("span").is_some()
                && f[0].get("replacement").is_some())),
        "no diagnostic carries fixes: {v}"
    );
    assert!(d.as_object().unwrap().contains_key("normalized_source"));
    assert!(d["normalized_source"].is_null());
    assert!(d["stats"].is_object());
    assert_eq!(
        *d,
        serde_json::to_value(validate(BROKEN, ValidateOpts::default())).unwrap()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tracer_valid_graph_returns_canonical_source() {
    let src = "graph {\n  node a:\"A\"\n}";
    let v = call("vox_validate_diagram", json!({ "source": src })).await;
    let d = &v["data"];
    assert_eq!(d["valid"], true, "{v}");
    assert_eq!(d["diagnostics"].as_array().unwrap().len(), 0);
    assert_eq!(d["normalized_source"], json!(vox_diagram::fmt(src)));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn missing_source_is_an_error() {
    let v = call("vox_validate_diagram", json!({})).await;
    assert_eq!(v["success"], false, "{v}");
    assert!(v["error"].as_str().unwrap().contains("source"), "{v}");
}
