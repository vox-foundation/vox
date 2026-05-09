//! Command-registry YAML types and JSON Schema validation.

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::collections::HashSet;
use std::path::Path;

pub(crate) use crate::command_registry_model::RegistryFile;
use vox_bounded_fs::read_utf8_path_capped;

pub(crate) const REGISTRY_REL: &str = "contracts/cli/command-registry.yaml";
pub(crate) const SCHEMA_REL: &str = "contracts/cli/command-registry.schema.json";
pub(crate) const MCP_TOOL_REGISTRY_REL: &str = "contracts/mcp/tool-registry.canonical.yaml";
pub(crate) const MCP_TOOL_REGISTRY_SCHEMA_REL: &str = "contracts/mcp/tool-registry.schema.json";
pub(crate) const MCP_HTTP_READ_ROLE_GOVERNANCE_REL: &str =
    "contracts/mcp/http-read-role-governance.yaml";
pub(crate) const MCP_HTTP_READ_ROLE_GOVERNANCE_SCHEMA_REL: &str =
    "contracts/mcp/http-read-role-governance.schema.json";
pub(crate) const CAPABILITY_REGISTRY_REL: &str = "contracts/capability/capability-registry.yaml";
pub(crate) const CAPABILITY_REGISTRY_SCHEMA_REL: &str =
    "contracts/capability/capability-registry.schema.json";

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

pub(crate) fn validate_mcp_tool_registry_against_json_schema(
    repo_root: &Path,
    yaml_text: &str,
) -> Result<()> {
    let schema_path = repo_root.join(MCP_TOOL_REGISTRY_SCHEMA_REL);
    let schema_val: JsonValue = serde_json::from_str(&read_utf8_path_capped(&schema_path)?)
        .with_context(|| format!("parse {} as JSON", schema_path.display()))?;
    let instance: JsonValue = serde_yaml::from_str(yaml_text)
        .context("parse MCP tool-registry.canonical.yaml to JSON")?;
    let validator = vox_jsonschema_util::compile_validator(&schema_val, schema_path.display())
        .context("compile MCP tool-registry JSON Schema")?;
    vox_jsonschema_util::validate(
        &instance,
        &validator,
        "tool-registry.canonical.yaml vs tool-registry.schema.json",
    )
    .map_err(|e| anyhow!("{e:#}"))?;
    Ok(())
}

pub(crate) fn validate_capability_registry_against_json_schema(
    repo_root: &Path,
    yaml_text: &str,
) -> Result<()> {
    let schema_path = repo_root.join(CAPABILITY_REGISTRY_SCHEMA_REL);
    let schema_val: JsonValue = serde_json::from_str(&read_utf8_path_capped(&schema_path)?)
        .with_context(|| format!("parse {} as JSON", schema_path.display()))?;
    let instance: JsonValue =
        serde_yaml::from_str(yaml_text).context("parse capability-registry.yaml to JSON value")?;
    let validator = vox_jsonschema_util::compile_validator(&schema_val, schema_path.display())
        .context("compile capability-registry JSON Schema")?;
    vox_jsonschema_util::validate(
        &instance,
        &validator,
        "capability-registry.yaml vs capability-registry.schema.json",
    )
    .map_err(|e| anyhow!("{e:#}"))?;
    Ok(())
}

pub(crate) fn validate_mcp_http_read_role_governance_against_json_schema(
    repo_root: &Path,
    yaml_text: &str,
) -> Result<()> {
    let schema_path = repo_root.join(MCP_HTTP_READ_ROLE_GOVERNANCE_SCHEMA_REL);
    let schema_val: JsonValue = serde_json::from_str(&read_utf8_path_capped(&schema_path)?)
        .with_context(|| format!("parse {} as JSON", schema_path.display()))?;
    let instance: JsonValue = serde_yaml::from_str(yaml_text)
        .context("parse MCP http-read-role-governance.yaml to JSON")?;
    let validator = vox_jsonschema_util::compile_validator(&schema_val, schema_path.display())
        .context("compile MCP HTTP read-role governance JSON Schema")?;
    vox_jsonschema_util::validate(
        &instance,
        &validator,
        "http-read-role-governance.yaml vs http-read-role-governance.schema.json",
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
    product_lane: String,
    #[serde(default)]
    http_read_role_eligible: bool,
}

const KNOWN_MCP_PRODUCT_LANES: &[&str] = &["app", "workflow", "ai", "interop", "data", "platform"];

pub(crate) fn parse_mcp_registry_yaml(yaml: &str) -> Result<Vec<String>> {
    let root: McpCanonicalRegistry =
        serde_yaml::from_str(yaml).context("parse MCP tool-registry.canonical.yaml")?;
    for t in &root.tools {
        if !KNOWN_MCP_PRODUCT_LANES.contains(&t.product_lane.as_str()) {
            return Err(anyhow!(
                "MCP registry: unknown product_lane `{}` for tool `{}`",
                t.product_lane,
                t.name
            ));
        }
    }
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

pub(crate) fn parse_mcp_registry_read_role_eligible(yaml: &str) -> Result<Vec<String>> {
    let root: McpCanonicalRegistry =
        serde_yaml::from_str(yaml).context("parse MCP tool-registry.canonical.yaml")?;
    for t in &root.tools {
        if !KNOWN_MCP_PRODUCT_LANES.contains(&t.product_lane.as_str()) {
            return Err(anyhow!(
                "MCP registry: unknown product_lane `{}` for tool `{}`",
                t.product_lane,
                t.name
            ));
        }
    }
    let mut out: Vec<String> = root
        .tools
        .into_iter()
        .filter(|t| t.http_read_role_eligible)
        .map(|t| t.name)
        .collect();
    out.sort();
    Ok(out)
}

/// Tool names from [`MCP_TOOL_REGISTRY_REL`] (SSOT); descriptions are enforced by `vox-mcp-registry` build.
pub(crate) fn extract_mcp_registry_tool_names(repo_root: &Path) -> Result<Vec<String>> {
    let p = repo_root.join(MCP_TOOL_REGISTRY_REL);
    let raw = read_utf8_path_capped(&p).with_context(|| format!("read {}", p.display()))?;
    parse_mcp_registry_yaml(&raw)
}

/// Read-role eligible MCP tool names from [`MCP_TOOL_REGISTRY_REL`] (`http_read_role_eligible: true`).
pub(crate) fn extract_mcp_registry_read_role_eligible(repo_root: &Path) -> Result<Vec<String>> {
    let p = repo_root.join(MCP_TOOL_REGISTRY_REL);
    let raw = read_utf8_path_capped(&p).with_context(|| format!("read {}", p.display()))?;
    parse_mcp_registry_read_role_eligible(&raw)
}
