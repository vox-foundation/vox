//! Command-registry YAML types and JSON Schema validation.

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::collections::HashSet;
use std::path::Path;

pub(crate) use crate::command_registry_model::RegistryFile;
use crate::commands::ci::bounded_read::read_utf8_path_capped;

pub(crate) const REGISTRY_REL: &str = "contracts/cli/command-registry.yaml";
pub(crate) const SCHEMA_REL: &str = "contracts/cli/command-registry.schema.json";
pub(crate) const MCP_TOOL_REGISTRY_REL: &str = "contracts/mcp/tool-registry.canonical.yaml";

pub(crate) fn validate_registry_against_json_schema(
    repo_root: &Path,
    yaml_text: &str,
) -> Result<()> {
    let schema_path = repo_root.join(SCHEMA_REL);
    let schema_val: JsonValue = serde_json::from_str(&read_utf8_path_capped(&schema_path)?)
        .with_context(|| {
            format!(
                "parse {} as JSON",
                schema_path
                    .strip_prefix(repo_root)
                    .unwrap_or(&schema_path)
                    .display()
            )
        })?;
    let instance: JsonValue =
        serde_yaml::from_str(yaml_text).context("parse command-registry.yaml to JSON value")?;
    let validator = vox_jsonschema_util::compile_validator(&schema_val, schema_path.display())
        .context("compile command-registry JSON Schema")?;
    vox_jsonschema_util::validate(
        &instance,
        &validator,
        "command-registry.yaml vs command-registry.schema.json",
    )
    .map_err(|e| anyhow!("{e:#}"))?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct McpCanonicalRegistry {
    #[allow(dead_code)]
    version: u32,
    tools: Vec<McpCanonicalTool>,
}

#[derive(Debug, Deserialize)]
struct McpCanonicalTool {
    name: String,
    #[allow(dead_code)]
    description: String,
}

pub(crate) fn parse_mcp_registry_yaml(yaml: &str) -> Result<Vec<String>> {
    let root: McpCanonicalRegistry =
        serde_yaml::from_str(yaml).context("parse MCP tool-registry.canonical.yaml")?;
    let out: Vec<String> = root.tools.into_iter().map(|t| t.name).collect();
    if out.is_empty() {
        return Err(anyhow!("MCP registry: `tools` must be non-empty"));
    }
    let mut seen = HashSet::<&str>::new();
    for n in &out {
        if !seen.insert(n.as_str()) {
            return Err(anyhow!("MCP registry: duplicate tool name `{n}`"));
        }
    }
    Ok(out)
}

/// Tool names from [`MCP_TOOL_REGISTRY_REL`] (SSOT); descriptions are enforced by `vox-mcp-registry` build.
pub(crate) fn extract_mcp_registry_tool_names(repo_root: &Path) -> Result<Vec<String>> {
    let p = repo_root.join(MCP_TOOL_REGISTRY_REL);
    let raw = read_utf8_path_capped(&p).with_context(|| format!("read {}", p.display()))?;
    parse_mcp_registry_yaml(&raw)
}
