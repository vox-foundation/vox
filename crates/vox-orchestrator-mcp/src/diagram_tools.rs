//! `vox_validate_diagram`: deterministic VoxDiagram validation for agents (D-01). Pure: no model
//! call, no network, no file written; the body is `vox_diagram::validate` serialised as-is.

use serde::Deserialize;
use serde_json::Value;

use crate::ServerState;
use crate::params::ToolResult;

/// Tool arguments. Fields are spelled out (not `flatten`): serde ignores the unknown-field
/// refusal under `flatten`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Args {
    source: String,
    #[serde(default)]
    from_mermaid: bool,
    #[serde(default)]
    emit: Option<vox_diagram::Emit>,
}

/// The description agents see in `tools/list`: for `vox_validate_diagram` the catalog text
/// followed by the one grammar card (D-05); every other tool is unchanged.
pub fn advertised_description(name: &str, base: &str) -> String {
    if name == "vox_validate_diagram" {
        format!("{base}\n\n{}", vox_diagram::GRAMMAR_CARD)
    } else {
        base.to_string()
    }
}

/// Validate a VoxDiagram `source` and return the shared `vox_diagram::Validation` envelope.
pub async fn vox_validate_diagram(_state: &ServerState, args: Value) -> String {
    run(args).await.to_json()
}

async fn run(args: Value) -> ToolResult<Value> {
    let args: Args = match serde_json::from_value(args) {
        Ok(a) => a,
        Err(e) => return ToolResult::err(format!("invalid arguments: {e}")),
    };
    let opts = vox_diagram::ValidateOpts {
        from_mermaid: args.from_mermaid,
        emit: args.emit,
    };
    // SVG rendering is synchronous and can take seconds at the caps: keep it off async workers.
    let joined =
        tokio::task::spawn_blocking(move || vox_diagram::validate(&args.source, opts)).await;
    match joined.map(|v| serde_json::to_value(&v)) {
        Ok(Ok(v)) => ToolResult::ok(v),
        Ok(Err(e)) => ToolResult::err(format!("could not serialise validation: {e}")),
        Err(e) => ToolResult::err(format!("validation task failed: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    async fn out(args: Value) -> Value {
        serde_json::from_str(&run(args).await.to_json()).unwrap()
    }

    #[tokio::test]
    async fn missing_source_is_an_error() {
        let r = out(json!({})).await;
        assert_eq!(r["success"], false);
        assert!(r["error"].as_str().unwrap().contains("source"), "{r}");
    }

    #[tokio::test]
    async fn present_source_is_validated() {
        let r = out(json!({"source": "graph {\n    node a: \"A\"\n}\n"})).await;
        assert_eq!(r["data"]["valid"], true, "{r}");
    }

    #[test]
    fn advertised_description_appends_the_card() {
        assert_eq!(
            advertised_description("vox_validate_diagram", "d"),
            format!("d\n\n{}", vox_diagram::GRAMMAR_CARD)
        );
        assert_eq!(advertised_description("vox_validate_vuv", "d"), "d");
    }

    #[tokio::test]
    async fn unknown_field_is_refused() {
        let r = out(json!({"source": "graph {\n}\n", "extra": 1})).await;
        assert_eq!(r["success"], false, "{r}");
    }
}
