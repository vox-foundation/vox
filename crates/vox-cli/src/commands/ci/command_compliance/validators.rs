//! Registry vs docs / lib.rs / compilerd / dei / script duals validators.

use anyhow::{Result, anyhow};
use regex::Regex;
use std::fs;
use std::path::Path;

use crate::commands::ci::bounded_read::read_utf8_path_capped;

use super::docs_sync::{markdown_section, ref_cli_vox_ci_section, ref_cli_vox_codex_section};
use super::registry::RegistryFile;

/// Known `latin_ns` values in [`contracts/cli/command-registry.yaml`] for `surface: vox-cli`.
const KNOWN_LATIN_NS: &[&str] = &["fabrica", "mens", "ars", "ci", "codex", "mens", "recensio"];

pub(crate) fn check_catalog_generation_smoke() -> Result<()> {
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

pub(crate) fn check_root_readme_cli_drift(readme: &str) -> Result<()> {
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

pub(crate) fn check_env_var_ssot_index(reg: &RegistryFile, env_ssot_md: &str) -> Result<()> {
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

pub(crate) fn check_vox_cli_lib(reg: &RegistryFile, lib_rs: &str) -> Result<()> {
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

pub(crate) fn kebab_to_pascal(s: &str) -> String {
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

pub(crate) fn check_ref_cli(reg: &RegistryFile, ref_text: &str) -> Result<()> {
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

pub(crate) fn check_reachability(reg: &RegistryFile, reach: &str) -> Result<()> {
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

pub(crate) fn check_compilerd(reg: &RegistryFile, compilerd: &str) -> Result<()> {
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

pub(crate) fn check_dei(reg: &RegistryFile, dei: &str) -> Result<()> {
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

fn vox_cli_src_contains_needle(root: &Path, needle: &str) -> Result<bool> {
    fn walk(dir: &Path, needle: &str, found: &mut bool) -> Result<()> {
        use anyhow::Context;
        for e in fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))? {
            let e = e?;
            let p = e.path();
            if p.is_dir() {
                walk(&p, needle, found)?;
            } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                let s =
                    read_utf8_path_capped(&p).with_context(|| format!("read {}", p.display()))?;
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

pub(crate) fn check_registry_latin_and_handlers(
    reg: &RegistryFile,
    vox_cli_src: &Path,
) -> Result<()> {
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

pub(crate) fn check_script_duals(
    reg: &RegistryFile,
    duals_doc: &str,
    scripts_readme: &str,
) -> Result<()> {
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
