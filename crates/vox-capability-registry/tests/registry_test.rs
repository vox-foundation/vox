#![allow(missing_docs)]
//! Smoke tests for `vox-capability-registry` public API.

use vox_capability_registry::{
    PopuliExposure, capability_to_openai_function, default_registry,
    populi_chat_parameters,
};

#[test]
fn default_registry_has_oratio_capabilities() {
    let reg = default_registry();
    let caps: Vec<_> = reg.populi_chat_capabilities().collect();
    assert!(
        !caps.is_empty(),
        "default_registry should expose at least one Populi-chat capability"
    );
    let ids: Vec<&str> = caps.iter().map(|c| c.capability_id.as_str()).collect();
    assert!(
        ids.contains(&"oratio.transcribe"),
        "default_registry should expose oratio.transcribe, got: {:?}",
        ids
    );
    assert!(
        ids.contains(&"oratio.status"),
        "default_registry should expose oratio.status, got: {:?}",
        ids
    );
}

#[test]
fn all_auto_exposed_capabilities_have_mcp_tool_name() {
    let reg = default_registry();
    for cap in reg.populi_chat_capabilities() {
        assert!(
            cap.populi_exposure == PopuliExposure::Auto,
            "populi_chat_capabilities iterator should only yield Auto-exposed caps"
        );
        assert!(
            cap.invocation_forms.mcp_tool.is_some(),
            "Auto-exposed capability '{}' must have an MCP tool name",
            cap.capability_id
        );
    }
}

#[test]
fn populi_chat_parameters_oratio_transcribe_has_path_property() {
    let params = populi_chat_parameters("oratio.transcribe");
    assert_eq!(
        params["type"].as_str().unwrap(),
        "object",
        "parameters must be an object schema"
    );
    assert!(
        params["properties"]["path"].is_object(),
        "oratio.transcribe params must have 'path' property"
    );
    let required = params["required"].as_array().unwrap();
    assert!(
        required.iter().any(|v| v.as_str() == Some("path")),
        "'path' must be in required list"
    );
}

#[test]
fn populi_chat_parameters_oratio_status_is_empty_object_schema() {
    let params = populi_chat_parameters("oratio.status");
    assert_eq!(
        params["type"].as_str().unwrap(),
        "object",
        "parameters must be an object schema"
    );
}

#[test]
fn populi_chat_parameters_unknown_returns_object_schema() {
    let params = populi_chat_parameters("unknown.capability");
    assert_eq!(
        params["type"].as_str().unwrap(),
        "object",
        "unknown capability should return a minimal object schema"
    );
}

#[test]
fn capability_to_openai_function_produces_correct_shape() {
    use serde_json::json;
    let params = json!({ "type": "object", "properties": {} });
    let def = capability_to_openai_function("my_tool", "Does something useful", params.clone());
    assert_eq!(def["type"].as_str().unwrap(), "function");
    assert_eq!(def["function"]["name"].as_str().unwrap(), "my_tool");
    assert_eq!(
        def["function"]["description"].as_str().unwrap(),
        "Does something useful"
    );
    assert_eq!(def["function"]["parameters"], params);
}
