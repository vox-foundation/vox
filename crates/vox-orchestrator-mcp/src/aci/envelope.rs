//! Attach and validate `aci` sibling metadata on [`crate::params::ToolResult`]-shaped JSON.

use std::sync::LazyLock;

use anyhow::Context as _;
use serde_json::{Value, json};
use vox_jsonschema_util::{compile_validator, validate};
use vox_orchestrator::agentos::checkpoint_engine::should_sparse_checkpoint;

static ACI_TOOL_RESPONSE_SCHEMA: LazyLock<Value> = LazyLock::new(|| {
    serde_json::from_str(include_str!(
        "../../../../contracts/aci/agent-computer-interface.v1.schema.json"
    ))
    .expect("parse embedded agent-computer-interface.v1.schema.json")
});

static ACI_TOOL_RESPONSE_VALIDATOR: LazyLock<vox_jsonschema_util::Validator> =
    LazyLock::new(|| {
        compile_validator(&ACI_TOOL_RESPONSE_SCHEMA, "aci mcp_tool_response_v1")
            .expect("compile ACI response schema")
    });

/// Parses `inner_json`, inserts `aci`, validates against the repo schema, returns compact JSON.
pub fn attach_aci_envelope(tool: &str, inner_json: &str, checkpoint_hints: bool) -> anyhow::Result<String> {
    let canon = super::normalization::tool_name_for_aci(tool);
    let mut val: Value = serde_json::from_str(inner_json)
        .with_context(|| format!("aci attach: tool {tool} output was not JSON"))?;
    let obj = val
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("aci attach: expected JSON object"))?;

    let success = obj.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
    let error_kind = if success {
        Value::Null
    } else {
        Value::String("tool_error".into())
    };

    let sparse = checkpoint_hints && should_sparse_checkpoint(canon);
    let aci = json!({
        "version": 1,
        "tool": canon,
        "mutation_kind": vox_orchestrator::agentos::mutation_classifier::mutation_kind_for_tool(canon),
        "shell_backend": Value::Null,
        "error_kind": error_kind,
        "side_effects": Value::Array(vec![]),
        "checkpoint_hint": {
            "sparse_checkpoint_recommended": sparse
        }
    });

    obj.insert("aci".to_string(), aci);

    validate(
        &val,
        &ACI_TOOL_RESPONSE_VALIDATOR,
        format!("mcp tool {canon} aci envelope"),
    )?;

    Ok(serde_json::to_string(&val)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aci_round_trip_on_tool_result_shape() {
        let inner = r#"{"success":true,"data":{"x":1}}"#;
        let out = attach_aci_envelope("vox_git_status", inner, false).expect("attach");
        let v: Value = serde_json::from_str(&out).expect("parse");
        assert_eq!(v["success"], true);
        assert_eq!(v["aci"]["tool"], "vox_git_status");
        assert_eq!(v["aci"]["mutation_kind"], "read_only");
    }
}
