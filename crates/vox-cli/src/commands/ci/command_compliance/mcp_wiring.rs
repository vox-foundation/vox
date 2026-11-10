//! MCP tool registry vs `handle_tool_call` wiring checks.

use anyhow::{Result, anyhow};
use regex::Regex;
use std::collections::HashSet;
use std::path::Path;
use std::sync::OnceLock;

use super::registry::{MCP_TOOL_REGISTRY_REL, extract_mcp_registry_tool_names};

fn assert_mcp_mod_reexports_registry(mcp_mod: &str) -> Result<()> {
    const NEEDLE: &str = "pub use vox_mcp_registry::TOOL_REGISTRY";
    if !mcp_mod.contains(NEEDLE) {
        return Err(anyhow!(
            "vox-orchestrator mcp_tools/mod.rs: must `{NEEDLE}` (registry source: {MCP_TOOL_REGISTRY_REL})"
        ));
    }
    Ok(())
}

/// Quoted identifiers before `=>` on a `handle_tool_call` match arm (supports `"a" | "b" =>`).
fn mcp_handler_arm_quoted_names(line: &str) -> Vec<String> {
    if !line.contains("=>") {
        return Vec::new();
    }
    static QUOTED: OnceLock<Regex> = OnceLock::new();
    let re = QUOTED.get_or_init(|| Regex::new(r#""([^"]+)""#).expect("quoted string pattern"));
    let before_arrow = line.split("=>").next().unwrap_or("");
    re.captures_iter(before_arrow)
        .map(|c| c[1].to_string())
        .collect()
}

pub(crate) fn extract_mcp_handler_tools(src: &str) -> Result<HashSet<String>> {
    static DEFAULT_ARM: OnceLock<Regex> = OnceLock::new();
    let default_arm = DEFAULT_ARM.get_or_init(|| {
        Regex::new(r"(?m)^\s*_\s*=>").expect("handle_tool_call default arm pattern")
    });

    let mut out = HashSet::new();

    if let Some(fn_pos) = src.find("pub async fn handle_tool_call") {
        let after_fn = &src[fn_pos..];
        let match_needle = "match name {";
        if let Some(m_start) = after_fn.find(match_needle) {
            let from_brace = &after_fn[m_start + match_needle.len()..];
            if let Some(end) = default_arm.find(from_brace).map(|m| m.start()) {
                let block = &from_brace[..end];
                for line in block.lines() {
                    let trimmed = line.trim_start();
                    if trimmed.starts_with("//") || !trimmed.contains("=>") {
                        continue;
                    }
                    if !(trimmed.starts_with('"') || trimmed.starts_with('|')) {
                        continue;
                    }
                    for name in mcp_handler_arm_quoted_names(trimmed) {
                        out.insert(name);
                    }
                }
            }
        }
    }

    if src.contains("dispatch_domains") {
        let candidate_dirs = [
            std::path::PathBuf::from("crates/vox-orchestrator-mcp/src/dispatch_domains"),
            std::path::PathBuf::from("../vox-orchestrator-mcp/src/dispatch_domains"),
        ];
        let domains_dir = candidate_dirs.into_iter().find(|p| p.is_dir()).or_else(|| {
            std::env::var("CARGO_MANIFEST_DIR").ok().and_then(|m| {
                let p = std::path::PathBuf::from(m);
                for anc in p.ancestors() {
                    let cand = anc.join("crates/vox-orchestrator-mcp/src/dispatch_domains");
                    if cand.is_dir() {
                        return Some(cand);
                    }
                }
                None
            })
        });

        if let Some(dir) = domains_dir {
            if let Ok(entries) = std::fs::read_dir(dir) {
                let mut entries: Vec<_> = entries.flatten().collect();
                entries.sort_by_key(|e| e.path());
                for entry in entries {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                        if let Ok(content) = std::fs::read_to_string(&path) {
                            for line in content.lines() {
                                let trimmed = line.trim_start();
                                if trimmed.starts_with("//") || !trimmed.contains("=>") {
                                    continue;
                                }
                                if !(trimmed.starts_with('"') || trimmed.starts_with('|')) {
                                    continue;
                                }
                                for name in mcp_handler_arm_quoted_names(trimmed) {
                                    out.insert(name);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if out.is_empty() {
        return Err(anyhow!(
            "vox-orchestrator: handle_tool_call missing `match name {{`"
        ));
    }
    Ok(out)
}

fn extract_mcp_tool_aliases(alias_yaml: &str) -> Result<Vec<(String, String)>> {
    let v: serde_yaml::Value = serde_yaml::from_str(alias_yaml)
        .map_err(|e| anyhow!("contracts/mcp/tool-wire-aliases.v1.yaml parse error: {e}"))?;
    let aliases = v
        .get("aliases")
        .and_then(|a| a.as_sequence())
        .ok_or_else(|| anyhow!("contracts/mcp/tool-wire-aliases.v1.yaml: missing `aliases`"))?;
    let mut out = Vec::new();
    for row in aliases {
        let Some(map) = row.as_mapping() else {
            continue;
        };
        let alias = map
            .get(serde_yaml::Value::String("alias".to_string()))
            .and_then(|x| x.as_str())
            .ok_or_else(|| {
                anyhow!("contracts/mcp/tool-wire-aliases.v1.yaml: alias row missing `alias`")
            })?;
        let canonical = map
            .get(serde_yaml::Value::String("canonical".to_string()))
            .and_then(|x| x.as_str())
            .ok_or_else(|| {
                anyhow!("contracts/mcp/tool-wire-aliases.v1.yaml: alias row missing `canonical`")
            })?;
        if alias.starts_with("vox_") && canonical.starts_with("vox_") {
            out.push((alias.to_string(), canonical.to_string()));
        }
    }
    if out.is_empty() {
        return Err(anyhow!(
            "contracts/mcp/tool-wire-aliases.v1.yaml: expected at least one vox_* alias pair"
        ));
    }
    Ok(out)
}

pub(crate) fn check_mcp_tool_wiring(
    repo_root: &Path,
    mcp_mod: &str,
    mcp_dispatch: &str,
    tool_aliases_yaml: &str,
) -> Result<()> {
    assert_mcp_mod_reexports_registry(mcp_mod)?;
    let reg_tools = extract_mcp_registry_tool_names(repo_root)?;
    let reg_set: HashSet<String> = reg_tools.iter().cloned().collect();
    let alias_pairs = extract_mcp_tool_aliases(tool_aliases_yaml)?;
    for (alias, canonical) in &alias_pairs {
        if reg_set.contains(alias) {
            return Err(anyhow!(
                "vox-orchestrator: tool_aliases alias `{alias}` must not duplicate a TOOL_REGISTRY name"
            ));
        }
        if !reg_set.contains(canonical) {
            return Err(anyhow!(
                "vox-orchestrator: tool_aliases `{alias}` → `{canonical}` but canonical not in TOOL_REGISTRY"
            ));
        }
    }
    let mut handler_allowed: HashSet<String> = reg_set.clone();
    for (alias, _) in &alias_pairs {
        handler_allowed.insert(alias.clone());
    }
    let han_tools = extract_mcp_handler_tools(mcp_dispatch)?;
    for t in &reg_tools {
        if !han_tools.contains(t) {
            return Err(anyhow!(
                "vox-orchestrator: tool `{t}` listed in TOOL_REGISTRY but missing from `handle_tool_call` match arms"
            ));
        }
    }
    for t in &han_tools {
        if !handler_allowed.contains(t) {
            return Err(anyhow!(
                "vox-orchestrator: `handle_tool_call` matches `{t}` but it is not listed in TOOL_REGISTRY (or `tool_aliases` wire alias)"
            ));
        }
    }

    let vscode_dir = repo_root.join("apps").join("editor").join("vox-vscode");
    if vscode_dir.is_dir() {
        let status = std::process::Command::new("node")
            .arg("scripts/check-mcp-tool-parity.mjs")
            .current_dir(&vscode_dir)
            .status();
        if let Ok(st) = status {
            if !st.success() {
                return Err(anyhow!("vox-vscode parity check failed. See output above."));
            }
        }
    }

    Ok(())
}
