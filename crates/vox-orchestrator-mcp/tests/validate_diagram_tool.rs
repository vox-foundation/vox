//! `vox_validate_diagram` end to end: dispatch arm -> `diagram_tools` -> `vox_diagram::validate`.

use std::sync::Arc;

use serde_json::{Value, json};
use vox_diagram::{Emit, ValidateOpts, validate};
use vox_orchestrator_mcp::{ServerState, handle_tool_call, load_config};

const BROKEN: &str = "graph {\n    node a: \"A\"\n    a -> b\n    node c: C\n}\n";

async fn call_raw(name: &str, args: Value) -> String {
    // Multi-thread callers: the dispatch path resolves secrets through `block_in_place`.
    let state = Arc::new(ServerState::new_full(load_config()));
    handle_tool_call(&state, name, args)
        .await
        .expect("dispatch ok")
}

async fn call(name: &str, args: Value) -> Value {
    let raw = call_raw(name, args).await;
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

const FLOW: &str = "flowchart TD\n    a[Start] --> b{Ok?}\n";
const OK: &str = "graph {\n    node a: \"A\"\n    node b: \"B\"\n    a -> b\n}\n";

fn expect(src: &str, opts: ValidateOpts) -> Value {
    serde_json::to_value(validate(src, opts)).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn from_mermaid_returns_conversion_or_unsupported() {
    let o = ValidateOpts {
        from_mermaid: true,
        emit: None,
    };
    let v = call(
        "vox_validate_diagram",
        json!({"source": FLOW, "from_mermaid": true}),
    )
    .await;
    assert_eq!(v["data"], expect(FLOW, o), "{v}");
    assert_eq!(v["data"]["unsupported"], json!([]));
    assert!(v["data"]["normalized_source"].is_string());

    let bad = "gantt\n    title X\n";
    let v = call(
        "vox_validate_diagram",
        json!({"source": bad, "from_mermaid": true}),
    )
    .await;
    assert_eq!(v["data"], expect(bad, o), "{v}");
    assert_eq!(v["data"]["valid"], false);
    assert!(v["data"]["normalized_source"].is_null());
    assert!(!v["data"]["unsupported"].as_array().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn emit_svg_and_mermaid_match_the_crate() {
    for (name, emit) in [("svg", Emit::Svg), ("mermaid", Emit::Mermaid)] {
        let v = call("vox_validate_diagram", json!({"source": OK, "emit": name})).await;
        let o = ValidateOpts {
            from_mermaid: false,
            emit: Some(emit),
        };
        assert_eq!(v["data"], expect(OK, o), "{name}");
        assert!(v["data"]["emitted"].as_str().is_some_and(|s| !s.is_empty()));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn rejects_unknown_fields_and_bad_emit() {
    let v = call(
        "vox_validate_diagram",
        json!({"source": "graph {\n}\n", "extra": 1}),
    )
    .await;
    assert_eq!(v["success"], false, "{v}");
    let v = call("vox_validate_diagram", json!({"source": OK, "emit": "png"})).await;
    assert_eq!(v["success"], false, "{v}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deterministic_across_calls() {
    let args = json!({"source": OK, "emit": "svg"});
    let a = call_raw("vox_validate_diagram", args.clone()).await;
    let b = call_raw("vox_validate_diagram", args).await;
    assert_eq!(a, b);
}

#[test]
fn tool_list_advertises_the_card() {
    let base = vox_orchestrator_mcp::TOOL_REGISTRY
        .iter()
        .find(|e| e.name == "vox_validate_diagram")
        .expect("registered")
        .description;
    let tool = vox_orchestrator_mcp::registry::tool_registry()
        .into_iter()
        .find(|t| t.name == "vox_validate_diagram")
        .expect("advertised");
    let d = tool.description.expect("description");
    assert!(d.starts_with(base), "{d}");
    assert!(d.ends_with(vox_diagram::GRAMMAR_CARD), "{d}");
}

#[test]
fn grammar_card_fits_600_tokens() {
    let n = tiktoken_rs::o200k_base_singleton()
        .encode_with_special_tokens(vox_diagram::GRAMMAR_CARD)
        .len();
    assert!(n <= 600, "grammar card is {n} o200k tokens, budget is 600");
}

#[test]
fn lang_profile_lists_the_tool() {
    let p = vox_mcp_registry::ActiveProfile::resolve(&["lang"]).expect("lang resolves");
    assert!(p.contains_tool("vox_validate_diagram"));
}
