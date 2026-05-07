//! JSON manifest for external models and Mens planner indexes.

use serde::Serialize;
use serde_json::{Value, json};

use crate::document::{CapabilityRegistryDoc, CuratedCapability};
use crate::ids::{implicit_cli_capability_id, implicit_mcp_capability_id};

#[derive(Debug, Serialize)]
pub struct ModelCapabilityManifest {
    pub schema: &'static str,
    pub schema_version: u32,
    pub mcp_tools: Vec<Value>,
    pub cli_commands: Vec<Value>,
    pub runtime_builtin_maps: Vec<Value>,
}

/// Build a machine-readable manifest merging implicit MCP/CLI ids with curated metadata.
pub fn build_model_manifest(
    doc: &CapabilityRegistryDoc,
    mcp_tools: &[String],
    cli_paths_active: &[Vec<String>],
) -> ModelCapabilityManifest {
    let curated_by_id: std::collections::HashMap<String, &CuratedCapability> =
        doc.curated.iter().map(|c| (c.id.clone(), c)).collect();

    let mcp_tools_out: Vec<Value> = mcp_tools
        .iter()
        .map(|t| {
            let id = implicit_mcp_capability_id(t);
            merge_mcp_entry(t, &id, curated_by_id.get(&id))
        })
        .collect();

    let cli_commands: Vec<Value> = cli_paths_active
        .iter()
        .map(|path| {
            let id = implicit_cli_capability_id(path);
            merge_cli_entry(path, &id, curated_by_id.get(&id))
        })
        .collect();

    let runtime_builtin_maps: Vec<Value> = doc
        .runtime_builtin_maps
        .iter()
        .map(|m| {
            json!({
                "namespace": m.namespace,
                "method": m.method,
                "capability_id": m.capability_id,
            })
        })
        .collect();

    ModelCapabilityManifest {
        schema: "vox_capability_model_manifest_v1",
        schema_version: doc.schema_version,
        mcp_tools: mcp_tools_out,
        cli_commands,
        runtime_builtin_maps,
    }
}

fn merge_mcp_entry(tool: &str, capability_id: &str, curated: Option<&&CuratedCapability>) -> Value {
    let base = json!({
        "capability_id": capability_id,
        "mcp_tool": tool,
        "implicit": curated.is_none(),
    });
    if let Some(c) = curated {
        merge_curated(base, c)
    } else {
        base
    }
}

fn merge_cli_entry(
    path: &[String],
    capability_id: &str,
    curated: Option<&&CuratedCapability>,
) -> Value {
    let base = json!({
        "capability_id": capability_id,
        "cli_path": path,
        "implicit": curated.is_none(),
    });
    if let Some(c) = curated {
        let mut merged = merge_curated(base, c);
        if let Some(params) = &c.parameters
            && let Some(obj) = merged.as_object_mut()
        {
            obj.insert("parameters".into(), params.clone());
        }
        merged
    } else {
        base
    }
}

fn merge_curated(mut base: Value, c: &CuratedCapability) -> Value {
    if let Some(obj) = base.as_object_mut() {
        if let Some(t) = &c.title {
            obj.insert("title".into(), json!(t));
        }
        if let Some(d) = &c.description_model {
            obj.insert("description_model".into(), json!(d));
        } else if let Some(d) = &c.description_human {
            obj.insert("description_model".into(), json!(d));
        }
        if !c.intent_tags.is_empty() {
            obj.insert("intent_tags".into(), json!(c.intent_tags));
        }
        if let Some(v) = &c.side_effect_class {
            obj.insert("side_effect_class".into(), json!(v));
        }
        if let Some(v) = &c.scope_kind {
            obj.insert("scope_kind".into(), json!(v));
        }
        if let Some(v) = c.reversible {
            obj.insert("reversible".into(), json!(v));
        }
        if let Some(v) = c.requires_repo {
            obj.insert("requires_repo".into(), json!(v));
        }
        if let Some(v) = c.requires_git {
            obj.insert("requires_git".into(), json!(v));
        }
        if let Some(v) = c.preferred_for_models {
            obj.insert("preferred_for_models".into(), json!(v));
        }
        if let Some(v) = c.human_takeover_friendly {
            obj.insert("human_takeover_friendly".into(), json!(v));
        }
        if let Some(v) = c.mens_planner_visible {
            obj.insert("mens_planner_visible".into(), json!(v));
        }
        obj.insert("curated".into(), json!(true));
    }
    base
}
