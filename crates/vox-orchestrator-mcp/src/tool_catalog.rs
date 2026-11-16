//! Deterministic tool-catalog export from the live MCP registry for behavioral agent evals.

use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashMap};

/// Default catalog tool names selected for the held-out agent episode suite (`D-11`).
///
/// Spans three product lanes (`app`, `ai`, `platform`), includes same-lane hard negatives,
/// and mixes `read_only`, `writes_files`, and `spawns_process` side-effect classes.
pub const AGENT_EPISODE_TOOL_NAMES: &[&str] = &[
    "vox_validate_file",
    "vox_run_tests",
    "vox_build_crate",
    "vox_lint_crate",
    "vox_git_status",
    "vox_git_diff",
    "vox_submit_task",
    "vox_task_status",
    "vox_cancel_task",
    "vox_apply_structured_edit",
    "vox_config_get",
];

fn sort_value_keys(val: Value) -> Value {
    match val {
        Value::Array(items) => Value::Array(items.into_iter().map(sort_value_keys).collect()),
        Value::Object(map) => {
            let sorted: BTreeMap<String, Value> = map
                .into_iter()
                .map(|(k, v)| (k, sort_value_keys(v)))
                .collect();
            let mut out = Map::new();
            for (k, v) in sorted {
                out.insert(k, v);
            }
            Value::Object(out)
        }
        other => other,
    }
}

/// Export a compact, schema-pinned JSON catalog for `names` in the caller's exact order.
pub fn catalog_for(names: &[&str]) -> Result<Value, String> {
    let live = crate::registry::tool_registry();
    let by_name: HashMap<String, _> = live
        .into_iter()
        .map(|tool| (tool.name.to_string(), tool))
        .collect();

    let mut tools = Vec::with_capacity(names.len());
    for &name in names {
        let rmcp_tool = by_name
            .get(name)
            .ok_or_else(|| format!("unknown or feature-disabled MCP tool '{name}'"))?;
        let entry = vox_mcp_registry::registry_entry(name)
            .ok_or_else(|| format!("tool '{name}' missing from vox_mcp_registry::TOOL_REGISTRY"))?;
        let description = rmcp_tool
            .description
            .as_deref()
            .unwrap_or(entry.description)
            .to_string();
        let input_schema = Value::Object((*rmcp_tool.input_schema).clone());
        let obj = serde_json::json!({
            "name": entry.name,
            "description": description,
            "input_schema": input_schema,
            "side_effect_class": entry.side_effect_class.as_str(),
            "product_lane": entry.product_lane,
        });
        tools.push(sort_value_keys(obj));
    }

    Ok(sort_value_keys(serde_json::json!({
        "schema": "vox_tool_catalog_v1",
        "tools": tools,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn committed_catalog_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("mens/data/agent_episodes/tool-catalog.v1.json")
    }

    #[test]
    fn catalog_for_returns_requested_tools_and_rejects_unknown() {
        let v = catalog_for(&["vox_validate_file", "vox_config_get"]).unwrap();
        assert_eq!(v["schema"], "vox_tool_catalog_v1");
        let tools = v["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0]["name"], "vox_validate_file");
        assert_eq!(tools[1]["name"], "vox_config_get");
        assert!(tools[0]["input_schema"].is_object());
        assert!(tools[1]["input_schema"].is_object());

        let err = catalog_for(&["vox_definitely_does_not_exist"]).unwrap_err();
        assert!(err.contains("vox_definitely_does_not_exist"), "{err}");
    }

    #[test]
    fn tool_catalog_lock() {
        let path = committed_catalog_path();
        let update_golden = std::env::var("VOX_UPDATE_GOLDEN").as_deref() == Ok("1");
        if update_golden || !path.exists() {
            let regenerated = catalog_for(AGENT_EPISODE_TOOL_NAMES)
                .expect("default agent episode tool names must exist in live registry");
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            let rendered = format!("{}\n", serde_json::to_string_pretty(&regenerated).unwrap());
            std::fs::write(&path, rendered).unwrap();
        }

        let committed_raw = std::fs::read_to_string(&path).expect("committed tool catalog");
        let committed_val: Value =
            serde_json::from_str(&committed_raw).expect("valid catalog JSON");
        let names: Vec<String> = committed_val["tools"]
            .as_array()
            .expect("tools array")
            .iter()
            .map(|t| t["name"].as_str().expect("tool name").to_string())
            .collect();
        let name_refs: Vec<&str> = names.iter().map(String::as_str).collect();
        let regenerated = sort_value_keys(catalog_for(&name_refs).expect("catalog_for"));
        assert_eq!(
            sort_value_keys(committed_val),
            regenerated,
            "committed tool-catalog.v1.json drifted from live registry; re-run with VOX_UPDATE_GOLDEN=1"
        );
    }
}
