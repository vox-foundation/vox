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

/// Extract the `### `vox ci …` section from `ref-cli.md` (until the next `### ` heading).
fn ref_cli_vox_ci_section(ref_text: &str) -> Option<&str> {
    let key = "### `vox ci";
    let start = ref_text.find(key)?;
    let after = &ref_text[start + 1..];
    let rel = after.find("\n### ").unwrap_or(after.len());
    let end = start + 1 + rel;
    Some(&ref_text[start..end])
}

/// Extract the `### `vox codex` section from `ref-cli.md` (until the next `### ` heading).
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

    let env_ssot = fs::read_to_string(repo_root.join("docs/src/reference/env-vars-ssot.md"))
        .context("read docs/src/reference/env-vars-ssot.md")?;
    check_env_var_ssot_index(&reg, &env_ssot)?;

    let ref_cli = fs::read_to_string(repo_root.join("docs/src/ref-cli.md"))
        .context("read docs/src/ref-cli.md")?;
    let reach =
        fs::read_to_string(repo_root.join("docs/src/architecture/cli-reachability-ssot.md"))
            .context("read cli-reachability-ssot.md")?;
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
    let vox_cli_src = repo_root.join("crates/vox-cli/src");

    check_vox_cli_lib(&reg, &lib_rs)?;
    check_registry_latin_and_handlers(&reg, &vox_cli_src)?;
    check_ref_cli(&reg, &ref_cli)?;
    check_reachability(&reg, &reach)?;
    check_compilerd(&reg, &compilerd)?;
    check_dei(&reg, &dei)?;
    check_mcp_tool_wiring(&mcp_mod, &mcp_tool_aliases)?;
    check_script_duals(&reg, &duals_doc, &scripts_readme)?;

    println!(
        "command-compliance OK (registry schema v{}, {} operations)",
        reg.schema_version,
        reg.operations.len()
    );
    Ok(())
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
                    "ref-cli.md: missing `### `vox ci …` section for registry path {:?}",
                    op.path
                )
            })?;
            let ticked = format!("`{sub}");
            if !ci_doc.contains(&ticked) {
                return Err(anyhow!(
                    "ref-cli.md: `vox ci` section must document `{sub}` with a backtick prefix (registry path {:?})",
                    op.path
                ));
            }
            continue;
        }
        if op.path.len() >= 2 && op.path[0] == "codex" {
            let sub = &op.path[1];
            let codex_doc = ref_cli_vox_codex_section(ref_text).ok_or_else(|| {
                anyhow!(
                    "ref-cli.md: missing `### `vox codex` section for registry path {:?}",
                    op.path
                )
            })?;
            let ticked = format!("`{sub}");
            if !codex_doc.contains(&ticked) && !codex_doc.contains(sub) {
                return Err(anyhow!(
                    "ref-cli.md: `vox codex` section should mention `{sub}` (registry path {:?})",
                    op.path
                ));
            }
            continue;
        }
        let needle = format!("vox {}", op.path.join(" "));
        if !ref_text.contains(&needle) {
            return Err(anyhow!(
                "ref-cli.md must mention `{needle}` (registry path {:?})",
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
                "cli-reachability-ssot.md: add table row `{needle}` for `{top}`"
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

/// Inner slice of `TOOL_REGISTRY` array literals — **anchor-based** end marker so `]` inside
/// description strings does not break parsing (see unit test). Keep end marker aligned with
/// `crates/vox-mcp/src/tools/mod.rs` before `pub fn tool_registry`.
fn tool_registry_array_slice(src: &str) -> Result<&str> {
    const START: &str = "pub const TOOL_REGISTRY: &[(&str, &str)] = &[";
    const END: &str = "\n];\n\n/// Convert the static [`TOOL_REGISTRY`]";
    let i = src
        .find(START)
        .ok_or_else(|| anyhow!("vox-mcp tools/mod.rs: missing `{START}`"))?;
    let tail = &src[i + START.len()..];
    let j = tail.find(END).ok_or_else(|| {
        anyhow!(
            "vox-mcp tools/mod.rs: missing TOOL_REGISTRY end anchor before `tool_registry()` (expected {END:?})"
        )
    })?;
    Ok(&tail[..j])
}

/// MCP tools use the `vox_*` naming convention; nonconforming names would be skipped here.
fn vox_mcp_tool_string_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#""(vox_[a-z0-9_]+)""#).expect("vox MCP quoted tool name pattern")
    })
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

fn extract_mcp_registry_tools(src: &str) -> Result<Vec<String>> {
    let block = tool_registry_array_slice(src)?;
    let re = vox_mcp_tool_string_regex();
    let mut out = Vec::new();
    for c in re.captures_iter(block) {
        out.push(c[1].to_string());
    }
    if out.is_empty() {
        return Err(anyhow!("vox-mcp: no tools parsed from TOOL_REGISTRY"));
    }
    Ok(out)
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
const KNOWN_LATIN_NS: &[&str] = &[
    "fabrica", "mens", "ars", "ci", "codex", "populi", "recensio",
];

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

fn check_mcp_tool_wiring(mcp_mod: &str, tool_aliases_rs: &str) -> Result<()> {
    let reg_tools = extract_mcp_registry_tools(mcp_mod)?;
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
    fn tool_registry_slice_tolerates_bracket_in_description() {
        let src = r#"
pub const TOOL_REGISTRY: &[(&str, &str)] = &[
    (
        "vox_bracket_test",
        "Description with ] bracket inside string",
    ),
];

/// Convert the static [`TOOL_REGISTRY`]
pub fn tool_registry() {}
"#;
        let tools = extract_mcp_registry_tools(src).expect("parse");
        assert!(tools.contains(&"vox_bracket_test".to_string()));
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
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../vox-mcp/src/tools");
        let src = fs::read_to_string(base.join("mod.rs")).expect("read vox-mcp tools/mod.rs");
        let aliases = fs::read_to_string(base.join("tool_aliases.rs"))
            .expect("read vox-mcp tools/tool_aliases.rs");
        let reg = extract_mcp_registry_tools(&src).expect("registry tools");
        let han = extract_mcp_handler_tools(&src).expect("handler tools");
        let missing: Vec<&String> = reg.iter().filter(|t| !han.contains(*t)).collect();
        assert!(
            missing.is_empty(),
            "registry tools missing from handle_tool_call parse: {:?}",
            missing
        );
        check_mcp_tool_wiring(&src, &aliases).expect("mcp wiring + aliases");
    }
}
