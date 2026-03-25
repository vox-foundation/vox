//! Command-registry YAML types and JSON Schema validation.

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

pub(crate) const REGISTRY_REL: &str = "contracts/cli/command-registry.yaml";
pub(crate) const SCHEMA_REL: &str = "contracts/cli/command-registry.schema.json";
pub(crate) const MCP_TOOL_REGISTRY_REL: &str = "contracts/mcp/tool-registry.canonical.yaml";

#[derive(Debug, Deserialize)]
pub(crate) struct RegistryFile {
    pub(crate) schema_version: u32,
    pub(crate) operations: Vec<RegistryOperation>,
    #[serde(default)]
    pub(crate) script_duals: Vec<ScriptDual>,
    /// Environment variable names that must appear in `docs/src/reference/env-vars-ssot.md`.
    #[serde(default)]
    pub(crate) env_var_ssot_index: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RegistryOperation {
    pub(crate) surface: String,
    pub(crate) path: Vec<String>,
    #[serde(default = "default_status")]
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) latin_ns: Option<String>,
    #[serde(default = "default_true")]
    pub(crate) ref_cli_required: bool,
    #[serde(default)]
    pub(crate) reachability_required: Option<bool>,
    #[serde(default)]
    pub(crate) handler_rust: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ScriptDual {
    pub(crate) script_glob: String,
    pub(crate) canonical_cli: String,
}

fn default_status() -> String {
    "active".to_string()
}

fn default_true() -> bool {
    true
}

pub(crate) fn validate_registry_against_json_schema(
    repo_root: &Path,
    yaml_text: &str,
) -> Result<()> {
    let schema_path = repo_root.join(SCHEMA_REL);
    let schema_val: JsonValue = serde_json::from_str(&fs::read_to_string(&schema_path)?)
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
    let validator =
        jsonschema::validator_for(&schema_val).context("compile command-registry JSON Schema")?;
    validator
        .validate(&instance)
        .map_err(|e| anyhow!("command-registry.yaml does not match schema: {e}"))?;
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
    let raw = fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))?;
    parse_mcp_registry_yaml(&raw)
}
