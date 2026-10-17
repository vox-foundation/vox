//! `vox_validate_diagram`: deterministic VoxDiagram validation for agents (D-01). Pure: no model
//! call, no network, no file written; the body is `vox_diagram::validate` serialised as-is.

use serde_json::Value;

use crate::ServerState;
use crate::params::ToolResult;

/// Validate a VoxDiagram `source` and return the shared `vox_diagram::Validation` envelope.
pub async fn vox_validate_diagram(_state: &ServerState, args: Value) -> String {
    run(&args).to_json()
}

fn run(args: &Value) -> ToolResult<Value> {
    let Some(source) = args.get("source").and_then(Value::as_str) else {
        return ToolResult::err("missing required string field 'source'");
    };
    match serde_json::to_value(vox_diagram::validate(
        source,
        vox_diagram::ValidateOpts::default(),
    )) {
        Ok(v) => ToolResult::ok(v),
        Err(e) => ToolResult::err(format!("could not serialise validation: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn missing_source_is_an_error() {
        let r: Value = serde_json::from_str(&run(&json!({})).to_json()).unwrap();
        assert_eq!(r["success"], false);
        assert!(r["error"].as_str().unwrap().contains("source"), "{r}");
    }

    #[test]
    fn present_source_is_validated() {
        let r = run(&json!({"source": "graph {\n    node a: \"A\"\n}\n"}));
        let r: Value = serde_json::from_str(&r.to_json()).unwrap();
        assert_eq!(r["data"]["valid"], true, "{r}");
    }
}
