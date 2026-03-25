//! `vox ci command-compliance` — validate [`contracts/cli/command-registry.yaml`](../../../../../../contracts/cli/command-registry.yaml) against docs and implementation sources.

use anyhow::{Context, Result, anyhow};
use regex::Regex;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

const REGISTRY_REL: &str = "contracts/cli/command-registry.yaml";
const SCHEMA_REL: &str = "contracts/cli/command-registry.schema.json";
const MCP_TOOL_REGISTRY_REL: &str = "contracts/mcp/tool-registry.canonical.yaml";

#[derive(Debug, Deserialize)]
struct RegistryFile {
    schema_version: u32,
    operations: Vec<RegistryOperation>,
    #[serde(default)]
    script_duals: Vec<ScriptDual>,
    /// Environment variable names that must appear in `docs/src/reference/env-vars-ssot.md`.
    #[serde(default)]
    env_var_ssot_index: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RegistryOperation {
    surface: String,
    path: Vec<String>,
    #[serde(default = "default_status")]
    status: String,
    #[serde(default)]
    latin_ns: Option<String>,
    #[serde(default = "default_true")]
    ref_cli_required: bool,
    #[serde(default)]
    reachability_required: Option<bool>,
    #[serde(default)]
    handler_rust: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ScriptDual {
    script_glob: String,
    canonical_cli: String,
}

fn default_status() -> String {
    "active".to_string()
}

fn default_true() -> bool {
    true
}

fn validate_registry_against_json_schema(repo_root: &Path, yaml_text: &str) -> Result<()> {
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

/// Extract the `### `vox ci …` section from the CLI reference doc (until the next `### ` heading).
fn ref_cli_vox_ci_section(ref_text: &str) -> Option<&str> {
    let key = "### `vox ci";
    let start = ref_text.find(key)?;
    let after = &ref_text[start + 1..];
    let rel = after.find("\n### ").unwrap_or(after.len());
    let end = start + 1 + rel;
    Some(&ref_text[start..end])
}

/// Extract the `### `vox codex` section from the CLI reference doc (until the next `### ` heading).
fn ref_cli_vox_codex_section(ref_text: &str) -> Option<&str> {
    let key = "### `vox codex";
    let start = ref_text.find(key)?;
    let after = &ref_text[start + 1..];
    let rel = after.find("\n### ").unwrap_or(after.len());
    let end = start + 1 + rel;
    Some(&ref_text[start..end])
}

/// Run all command-compliance checks from a repository root (directory containing `AGENTS.md`).
pub fn run(repo_root: &Path) -> Result<()> {
    let reg_path = repo_root.join(REGISTRY_REL);
    let raw =
        fs::read_to_string(&reg_path).with_context(|| format!("read {}", reg_path.display()))?;
    let schema_path = repo_root.join(SCHEMA_REL);
    if !schema_path.is_file() {
        return Err(anyhow!("missing {}", schema_path.display()));
    }
    validate_registry_against_json_schema(repo_root, &raw)
        .context("command-registry.yaml JSON Schema validation")?;
    let reg: RegistryFile = serde_yaml::from_str(&raw).context("parse command-registry.yaml")?;
    if reg.schema_version < 1 {
        return Err(anyhow!(
            "command-registry.yaml: schema_version must be >= 1 (got {})",
            reg.schema_version
        ));
    }

    let env_ssot = read_env_vars_ssot_doc(repo_root)?;
    check_env_var_ssot_index(&reg, &env_ssot)?;

    let ref_cli = read_cli_reference_for_compliance(repo_root)?;
    let reach = read_reachability_doc(repo_root)?;
    let duals_doc = fs::read_to_string(repo_root.join("docs/src/ci/command-surface-duals.md"))
        .context("read command-surface-duals.md")?;
    let lib_rs =
        fs::read_to_string(repo_root.join("crates/vox-cli/src/lib.rs")).context("read lib.rs")?;
    let compilerd = fs::read_to_string(repo_root.join("crates/vox-cli/src/compilerd.rs"))
        .context("read compilerd.rs")?;
    let dei = fs::read_to_string(repo_root.join("crates/vox-cli/src/dei_daemon.rs"))
        .context("read dei_daemon.rs")?;
    let mcp_mod = fs::read_to_string(repo_root.join("crates/vox-mcp/src/tools/mod.rs"))
        .context("read vox-mcp tools/mod.rs")?;
    let mcp_tool_aliases =
        fs::read_to_string(repo_root.join("crates/vox-mcp/src/tools/tool_aliases.rs"))
            .context("read vox-mcp tools/tool_aliases.rs")?;
    let scripts_readme = fs::read_to_string(repo_root.join("scripts/README.md"))
        .context("read scripts/README.md")?;
    let root_readme = fs::read_to_string(repo_root.join("README.md")).context("read README.md")?;
    let vox_cli_src = repo_root.join("crates/vox-cli/src");

    check_vox_cli_lib(&reg, &lib_rs)?;
    check_registry_latin_and_handlers(&reg, &vox_cli_src)?;
    check_ref_cli(&reg, &ref_cli)?;
    check_reachability(&reg, &reach)?;
    check_compilerd(&reg, &compilerd)?;
    check_dei(&reg, &dei)?;
    check_mcp_tool_wiring(repo_root, &mcp_mod, &mcp_tool_aliases)?;
    check_script_duals(&reg, &duals_doc, &scripts_readme)?;
    check_catalog_generation_smoke()?;
    check_root_readme_cli_drift(&root_readme)?;

    println!(
        "command-compliance OK (registry schema v{}, {} operations)",
        reg.schema_version,
        reg.operations.len()
    );
    Ok(())
}

fn check_catalog_generation_smoke() -> Result<()> {
    let catalog = crate::command_catalog::build_catalog();
    if catalog.entries.is_empty() {
        return Err(anyhow!("command catalog generation produced zero entries"));
    }
    for required in ["vox build", "vox check", "vox run"] {
        if !catalog.entries.iter().any(|e| e.command == required) {
            return Err(anyhow!(
                "command catalog generation missing expected command `{required}`"
            ));
        }
    }
    Ok(())
}

fn check_root_readme_cli_drift(readme: &str) -> Result<()> {
    for stale in ["docs/src/ref-cli.md", "docs/src/faq.md"] {
        if readme.contains(stale) {
            return Err(anyhow!(
                "README.md contains stale path `{stale}`; use canonical docs paths"
            ));
        }
    }
    let section = markdown_section(readme, "## The CLI").ok_or_else(|| {
        anyhow!("README.md is missing `## The CLI` section required for discoverability")
    })?;
    for stale_cmd in [
        "vox upgrade",
        "vox doc",
        "vox schola train",
        "vox dashboard",
        "vox agent list",
    ] {
        let pat = Regex::new(&format!(r"(?m)\b{}\b", regex::escape(stale_cmd)))
            .expect("hardcoded stale command regex should compile");
        if pat.is_match(section) {
            return Err(anyhow!(
                "README.md `## The CLI` contains stale command `{stale_cmd}`"
            ));
        }
    }
    if !section.contains("vox commands --recommended") {
        return Err(anyhow!(
            "README.md `## The CLI` must include `vox commands --recommended` for first-time discovery"
        ));
    }
    Ok(())
}

fn markdown_section<'a>(doc: &'a str, heading: &str) -> Option<&'a str> {
    let start = doc.find(heading)?;
    let after = &doc[start + heading.len()..];
    let rel_end = after.find("\n## ").unwrap_or(after.len());
    Some(&doc[start..start + heading.len() + rel_end])
}

fn check_env_var_ssot_index(reg: &RegistryFile, env_ssot_md: &str) -> Result<()> {
    for name in &reg.env_var_ssot_index {
        let needle = format!("`{name}`");
        if !env_ssot_md.contains(&needle) {
            return Err(anyhow!(
                "command-registry env_var_ssot_index: {needle} not found in docs/src/reference/env-vars-ssot.md"
            ));
        }
    }
    Ok(())
}

/// CLI reference text for `ref_cli_required` needles: `docs/src/ref-cli.md` if present, else canonical `docs/src/reference/cli.md`.
/// Strict `check_ref_cli` always runs on the resolved body (no silent skip when only the canonical doc exists).
fn read_cli_reference_for_compliance(repo_root: &Path) -> Result<String> {
    let legacy = repo_root.join("docs/src/ref-cli.md");
    if legacy.is_file() {
        return fs::read_to_string(&legacy).with_context(|| format!("read {}", legacy.display()));
    }
    let canonical = repo_root.join("docs/src/reference/cli.md");
    fs::read_to_string(&canonical).with_context(|| {
        format!(
            "read {} (and docs/src/ref-cli.md is absent)",
            canonical.display()
        )
    })
}

fn read_env_vars_ssot_doc(repo_root: &Path) -> Result<String> {
    let preferred = repo_root.join("docs/src/reference/env-vars-ssot.md");
    if preferred.is_file() {
        return fs::read_to_string(&preferred)
            .with_context(|| format!("read {}", preferred.display()));
    }
    let fallback = repo_root.join("docs/src/reference/env-vars.md");
    fs::read_to_string(&fallback).with_context(|| {
        format!(
            "read {} (fallback when docs/src/reference/env-vars-ssot.md is absent)",
            fallback.display()
        )
    })
}

fn read_reachability_doc(repo_root: &Path) -> Result<String> {
    let p = repo_root.join("docs/src/reference/cli.md");
    fs::read_to_string(&p).with_context(|| {
        format!(
            "read {} (reachability matrix under 'CLI command reachability')",
            p.display()
        )
    })
}

fn check_vox_cli_lib(reg: &RegistryFile, lib_rs: &str) -> Result<()> {
    for op in &reg.operations {
        if op.surface != "vox-cli" || op.status == "retired" {
            continue;
        }
        if op.path == ["tool-registry"] {
            continue;
        }
        let top = op.path.first().expect("registry path empty");
        if !lib_contains_subcommand(lib_rs, top) {
            return Err(anyhow!(
                "registry vox-cli path {:?} — top-level `{top}` not found in crates/vox-cli/src/lib.rs `Cli`",
                op.path
            ));
        }
    }
    Ok(())
}

fn kebab_to_pascal(s: &str) -> String {
    s.split(|c: char| ['-', '.'].contains(&c))
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut c = p.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            }
        })
        .collect()
}

fn lib_contains_subcommand(lib_rs: &str, kebab_name: &str) -> bool {
    let pascal = kebab_to_pascal(kebab_name);
    if lib_rs.contains(&format!("{pascal} {{")) || lib_rs.contains(&format!("\n    {pascal} {{")) {
        return true;
    }
    for line in lib_rs.lines() {
        let t = line.trim_start();
        if t.starts_with(&format!("{pascal},")) || t == pascal {
            return true;
        }
    }
    if lib_rs.contains(&format!(r#"name = "{kebab_name}""#)) {
        return true;
    }
    false
}

fn check_ref_cli(reg: &RegistryFile, ref_text: &str) -> Result<()> {
    for op in &reg.operations {
        if op.surface != "vox-cli" || !op.ref_cli_required {
            continue;
        }
        if matches!(op.status.as_str(), "retired" | "internal" | "deprecated") {
            continue;
        }
        if op.path.len() >= 2 && op.path[0] == "ci" {
            let sub = &op.path[1];
            let ci_doc = ref_cli_vox_ci_section(ref_text).ok_or_else(|| {
                anyhow!(
                    "CLI reference: missing `### `vox ci …` section for registry path {:?} (see docs/src/reference/cli.md)",
                    op.path
                )
            })?;
            let ticked = format!("`{sub}");
            if !ci_doc.contains(&ticked) {
                return Err(anyhow!(
                    "CLI reference `vox ci` section must document `{sub}` with a backtick prefix (registry path {:?}; docs/src/reference/cli.md)",
                    op.path
                ));
            }
            continue;
        }
        if op.path.len() >= 2 && op.path[0] == "codex" {
            let sub = &op.path[1];
            let codex_doc = ref_cli_vox_codex_section(ref_text).ok_or_else(|| {
                anyhow!(
                    "CLI reference: missing `### `vox codex` section for registry path {:?} (see docs/src/reference/cli.md)",
                    op.path
                )
            })?;
            let ticked = format!("`{sub}");
            if !codex_doc.contains(&ticked) && !codex_doc.contains(sub) {
                return Err(anyhow!(
                    "CLI reference `vox codex` section should mention `{sub}` (registry path {:?}; docs/src/reference/cli.md)",
                    op.path
                ));
            }
            continue;
        }
        let needle = format!("vox {}", op.path.join(" "));
        if !ref_text.contains(&needle) {
            return Err(anyhow!(
                "CLI reference must mention `{needle}` (registry path {:?}; docs/src/reference/cli.md)",
                op.path
            ));
        }
    }
    Ok(())
}

fn check_reachability(reg: &RegistryFile, reach: &str) -> Result<()> {
    for op in &reg.operations {
        if op.surface != "vox-cli" || op.path.len() != 1 {
            continue;
        }
        if op.reachability_required == Some(false) {
            continue;
        }
        if matches!(op.status.as_str(), "retired" | "internal" | "deprecated") {
            continue;
        }
        let top = &op.path[0];
        if matches!(
            top.as_str(),
            "completions" | "fabrica" | "mens" | "ars" | "recensio"
        ) {
            continue;
        }
        let needle = format!("| `{top}` |");
        if !reach.contains(&needle) {
            return Err(anyhow!(
                "docs/src/reference/cli.md (reachability table): add row `{needle}` for `{top}`"
            ));
        }
    }
    Ok(())
}

fn check_compilerd(reg: &RegistryFile, compilerd: &str) -> Result<()> {
    for op in &reg.operations {
        if op.surface != "compilerd" || op.status == "retired" {
            continue;
        }
        let method = op.path.join(".");
        let pattern = format!("\"{method}\" =>");
        if !compilerd.contains(&pattern) {
            return Err(anyhow!(
                "compilerd.rs: expected dispatch arm `{pattern}` for registry {:?}",
                op.path
            ));
        }
    }
    Ok(())
}

fn check_dei(reg: &RegistryFile, dei: &str) -> Result<()> {
    for op in &reg.operations {
        if op.surface != "dei-daemon" {
            continue;
        }
        let id = op.path.join(".");
        if !dei.contains(&format!("\"{id}\"")) {
            return Err(anyhow!(
                "dei_daemon.rs: expected RPC id `\"{id}\"` constant for registry {:?}",
                op.path
            ));
        }
    }
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

fn parse_mcp_registry_yaml(yaml: &str) -> Result<Vec<String>> {
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
fn extract_mcp_registry_tool_names(repo_root: &Path) -> Result<Vec<String>> {
    let p = repo_root.join(MCP_TOOL_REGISTRY_REL);
    let raw = fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))?;
    parse_mcp_registry_yaml(&raw)
}

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

fn extract_mcp_handler_tools(src: &str) -> Result<HashSet<String>> {
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

/// Known `latin_ns` values in [`contracts/cli/command-registry.yaml`] for `surface: vox-cli`.
const KNOWN_LATIN_NS: &[&str] = &["fabrica", "mens", "ars", "ci", "codex", "mens", "recensio"];

fn vox_cli_src_contains_needle(root: &Path, needle: &str) -> Result<bool> {
    fn walk(dir: &Path, needle: &str, found: &mut bool) -> Result<()> {
        for e in fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))? {
            let e = e?;
            let p = e.path();
            if p.is_dir() {
                walk(&p, needle, found)?;
            } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                let s = fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))?;
                if s.contains(needle) {
                    *found = true;
                }
            }
        }
        Ok(())
    }
    let mut found = false;
    walk(root, needle, &mut found)?;
    Ok(found)
}

fn check_registry_latin_and_handlers(reg: &RegistryFile, vox_cli_src: &Path) -> Result<()> {
    for op in &reg.operations {
        if op.surface != "vox-cli" || matches!(op.status.as_str(), "retired" | "internal") {
            continue;
        }
        if let Some(ref ns) = op.latin_ns {
            if !KNOWN_LATIN_NS.contains(&ns.as_str()) {
                return Err(anyhow!(
                    "command-registry: unknown latin_ns `{ns}` for vox-cli path {:?}",
                    op.path
                ));
            }
        }
        if let Some(ref h) = op.handler_rust {
            if matches!(op.status.as_str(), "deprecated") {
                continue;
            }
            if !vox_cli_src_contains_needle(vox_cli_src, h)? {
                return Err(anyhow!(
                    "command-registry: handler_rust `{h}` for path {:?} not found under crates/vox-cli/src",
                    op.path
                ));
            }
        }
    }
    Ok(())
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

fn check_mcp_tool_wiring(repo_root: &Path, mcp_mod: &str, tool_aliases_rs: &str) -> Result<()> {
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
    let han_tools = extract_mcp_handler_tools(mcp_mod)?;
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

fn check_script_duals(reg: &RegistryFile, duals_doc: &str, scripts_readme: &str) -> Result<()> {
    for d in &reg.script_duals {
        let canon_ok =
            duals_doc.contains(&d.canonical_cli) || scripts_readme.contains(&d.canonical_cli);
        if !canon_ok {
            return Err(anyhow!(
                "scripts/README.md or command-surface-duals.md must mention `{}` (registry script_duals)",
                d.canonical_cli
            ));
        }
        let mut base = d.script_glob.trim();
        base = base.trim_end_matches(".*");
        base = base.rsplit_once('/').map(|(_, b)| b).unwrap_or(base);
        if !scripts_readme.contains(base) {
            return Err(anyhow!(
                "scripts/README.md should reference script stem `{}` (from glob `{}`)",
                base,
                d.script_glob
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kebab_pascal() {
        assert_eq!(kebab_to_pascal("stub-check"), "StubCheck");
        assert_eq!(kebab_to_pascal("fmt.check"), "FmtCheck");
    }

    #[test]
    fn ref_cli_vox_ci_finds_manifest_backtick() {
        let md = "\n### `vox ci …`\n\n| `manifest` | x |\n\n### `vox dev`\n";
        let sec = ref_cli_vox_ci_section(md).expect("section");
        assert!(sec.contains("`manifest"));
    }

    #[test]
    fn mcp_registry_yaml_tolerates_bracket_in_description() {
        let yaml = r#"
version: 1
tools:
  - name: "vox_bracket_test"
    description: "Description with ] bracket inside string"
"#;
        let tools = parse_mcp_registry_yaml(yaml).expect("parse");
        assert_eq!(tools, vec!["vox_bracket_test".to_string()]);
    }

    #[test]
    fn mcp_handler_extract_includes_alternation_arms() {
        let src = r#"
pub async fn handle_tool_call() {
    match name {
        "vox_config_get" | "vox_get_config" => { ok }
        "vox_other" => { ok }
        _ => { err }
    }
}
"#;
        let h = extract_mcp_handler_tools(src).expect("parse");
        assert!(h.contains("vox_config_get"));
        assert!(h.contains("vox_get_config"));
        assert!(h.contains("vox_other"));
    }

    #[test]
    fn mcp_handler_default_arm_tolerates_indent() {
        let src = r#"pub async fn handle_tool_call() {
    match name {
			"vox_indented_only" => { ok }
			_ => { err }
    }
}"#;
        let h = extract_mcp_handler_tools(src).expect("parse");
        assert!(h.contains("vox_indented_only"));
    }

    #[test]
    fn ref_cli_vox_codex_section_excludes_other_headings() {
        let md = "\n### `vox codex`\n\n| `import` | x |\n\n### `vox dev`\nverify unrelated\n";
        let sec = ref_cli_vox_codex_section(md).expect("codex section");
        assert!(sec.contains("`import"));
        assert!(!sec.contains("verify"));
    }

    #[test]
    fn ref_cli_vox_ci_section_until_eof_when_last_heading() {
        let md = "### `vox ci …`\n\n| `manifest` | x |\n";
        let sec = ref_cli_vox_ci_section(md).expect("ci section");
        assert!(sec.contains("`manifest"));
    }

    /// Guard against drift in `vox-mcp` layout: full wiring must stay parseable by the compliance gate.
    #[test]
    fn mcp_extract_matches_workspace_vox_mcp_mod_rs() {
        let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("vox-cli lives at crates/vox-cli");
        let base = repo_root.join("crates/vox-mcp/src/tools");
        let src = fs::read_to_string(base.join("mod.rs")).expect("read vox-mcp tools/mod.rs");
        let aliases = fs::read_to_string(base.join("tool_aliases.rs"))
            .expect("read vox-mcp tools/tool_aliases.rs");
        let reg = extract_mcp_registry_tool_names(repo_root).expect("registry tools");
        let han = extract_mcp_handler_tools(&src).expect("handler tools");
        let missing: Vec<&String> = reg.iter().filter(|t| !han.contains(*t)).collect();
        assert!(
            missing.is_empty(),
            "registry tools missing from handle_tool_call parse: {:?}",
            missing
        );
        check_mcp_tool_wiring(repo_root, &src, &aliases).expect("mcp wiring + aliases");
    }
}
