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
            "vox-mcp tools/mod.rs: must `{NEEDLE}` (registry source: {MCP_TOOL_REGISTRY_REL})"
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

    let fn_pos = src
        .find("pub async fn handle_tool_call")
        .ok_or_else(|| anyhow!("vox-mcp: missing handle_tool_call"))?;
    let after_fn = &src[fn_pos..];
    let match_needle = "match name {";
    let m_start = after_fn
        .find(match_needle)
        .ok_or_else(|| anyhow!("vox-mcp: handle_tool_call missing `match name {{`"))?;
    let from_brace = &after_fn[m_start + match_needle.len()..];
    let end = default_arm
        .find(from_brace)
        .map(|m| m.start())
        .ok_or_else(|| anyhow!("vox-mcp: handle_tool_call missing `_ =>` default arm"))?;
    let block = &from_brace[..end];
    let mut out = HashSet::new();
    for line in block.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        if !trimmed.contains("=>") {
            continue;
        }
        if !(trimmed.starts_with('"') || trimmed.starts_with('|')) {
            continue;
        }
        for name in mcp_handler_arm_quoted_names(trimmed) {
            out.insert(name);
        }
    }
    Ok(out)
}

fn extract_mcp_tool_aliases(alias_rs: &str) -> Result<Vec<(String, String)>> {
    let re = Regex::new(r#"\("([^"]+)",\s*"([^"]+)"\)"#).expect("mcp alias pair regex");
    let mut out = Vec::new();
    for c in re.captures_iter(alias_rs) {
        let a = c[1].to_string();
        let b = c[2].to_string();
        if a.starts_with("vox_") && b.starts_with("vox_") {
            out.push((a, b));
        }
    }
    if out.is_empty() {
        return Err(anyhow!(
            "vox-mcp tools/tool_aliases.rs: expected TOOL_WIRE_ALIASES vox_* pairs"
        ));
    }
    Ok(out)
}

pub(crate) fn check_mcp_tool_wiring(
    repo_root: &Path,
    mcp_mod: &str,
    mcp_dispatch: &str,
    tool_aliases_rs: &str,
) -> Result<()> {
    assert_mcp_mod_reexports_registry(mcp_mod)?;
    let reg_tools = extract_mcp_registry_tool_names(repo_root)?;
    let reg_set: HashSet<String> = reg_tools.iter().cloned().collect();
    let alias_pairs = extract_mcp_tool_aliases(tool_aliases_rs)?;
    for (alias, canonical) in &alias_pairs {
        if reg_set.contains(alias) {
            return Err(anyhow!(
                "vox-mcp: tool_aliases alias `{alias}` must not duplicate a TOOL_REGISTRY name"
            ));
        }
        if !reg_set.contains(canonical) {
            return Err(anyhow!(
                "vox-mcp: tool_aliases `{alias}` → `{canonical}` but canonical not in TOOL_REGISTRY"
            ));
        }
    }
    let han_tools = extract_mcp_handler_tools(mcp_dispatch)?;
    for t in &reg_tools {
        if !han_tools.contains(t) {
            return Err(anyhow!(
                "vox-mcp: tool `{t}` listed in TOOL_REGISTRY but missing from `handle_tool_call` match arms"
            ));
        }
    }
    for t in &han_tools {
        if !reg_tools.contains(t) {
            return Err(anyhow!(
                "vox-mcp: `handle_tool_call` matches `{t}` but it is not listed in TOOL_REGISTRY"
            ));
        }
    }
    Ok(())
}
