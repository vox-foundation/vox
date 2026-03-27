//! `vox ci command-compliance` — validate [`contracts/cli/command-registry.yaml`](../../../../../../contracts/cli/command-registry.yaml) against docs and implementation sources.

use anyhow::{Context, Result, anyhow};
use std::path::Path;

use crate::commands::ci::bounded_read::read_utf8_path_capped;

mod docs_sync;
mod mcp_wiring;
mod registry;
#[cfg(test)]
mod tests;
mod validators;

use docs_sync::{read_cli_reference_for_compliance, read_env_vars_ssot_doc, read_reachability_doc};
use mcp_wiring::check_mcp_tool_wiring;
use registry::{REGISTRY_REL, RegistryFile, SCHEMA_REL, validate_registry_against_json_schema};
use validators::{
    check_catalog_feature_gates_match_registry, check_catalog_generation_smoke,
    check_command_registry_embed_matches_disk, check_compilerd, check_dei,
    check_dockerfiles_cargo_locked_policy, check_env_var_ssot_index, check_install_policy_surfaces,
    check_operator_docs_no_legacy_vox_install_pm_nudge,
    check_packaging_pm_docs_no_resurrected_uv_copies, check_project_pm_commands_no_toolchain_lane,
    check_reachability, check_ref_cli, check_registry_latin_and_handlers,
    check_root_readme_cli_drift, check_script_duals, check_upgrade_toolchain_only, check_vox_cli_lib,
};

use super::command_sync;

/// Run all command-compliance checks from a repository root (directory containing `AGENTS.md`).
pub fn run(repo_root: &Path) -> Result<()> {
    let reg_path = repo_root.join(REGISTRY_REL);
    let raw =
        read_utf8_path_capped(&reg_path).with_context(|| format!("read {}", reg_path.display()))?;
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
    let duals_doc = read_utf8_path_capped(&repo_root.join("docs/src/ci/command-surface-duals.md"))
        .context("read command-surface-duals.md")?;
    let lib_rs = read_utf8_path_capped(&repo_root.join("crates/vox-cli/src/lib.rs"))
        .context("read lib.rs")?;
    let compilerd = read_utf8_path_capped(&repo_root.join("crates/vox-cli/src/compilerd.rs"))
        .context("read compilerd.rs")?;
    let dei = read_utf8_path_capped(&repo_root.join("crates/vox-cli/src/dei_daemon.rs"))
        .context("read dei_daemon.rs")?;
    let mcp_mod = read_utf8_path_capped(&repo_root.join("crates/vox-mcp/src/tools/mod.rs"))
        .context("read vox-mcp tools/mod.rs")?;
    let mcp_dispatch =
        read_utf8_path_capped(&repo_root.join("crates/vox-mcp/src/tools/dispatch.rs"))
            .context("read vox-mcp tools/dispatch.rs")?;
    let mcp_tool_aliases =
        read_utf8_path_capped(&repo_root.join("crates/vox-mcp/src/tools/tool_aliases.rs"))
            .context("read vox-mcp tools/tool_aliases.rs")?;
    let scripts_readme = read_utf8_path_capped(&repo_root.join("scripts/README.md"))
        .context("read scripts/README.md")?;
    let root_readme =
        read_utf8_path_capped(&repo_root.join("README.md")).context("read README.md")?;
    let vox_cli_src = repo_root.join("crates/vox-cli/src");

    check_vox_cli_lib(&reg, &lib_rs)?;
    check_install_policy_surfaces(repo_root)?;
    check_upgrade_toolchain_only(repo_root)?;
    check_project_pm_commands_no_toolchain_lane(repo_root)?;
    check_dockerfiles_cargo_locked_policy(repo_root)?;
    check_operator_docs_no_legacy_vox_install_pm_nudge(repo_root)?;
    check_packaging_pm_docs_no_resurrected_uv_copies(repo_root)?;
    check_registry_latin_and_handlers(&reg, &vox_cli_src)?;
    check_ref_cli(&reg, &ref_cli)?;
    check_reachability(&reg, &reach)?;
    check_compilerd(&reg, &compilerd)?;
    check_dei(&reg, &dei)?;
    check_mcp_tool_wiring(repo_root, &mcp_mod, &mcp_dispatch, &mcp_tool_aliases)?;
    check_script_duals(&reg, &duals_doc, &scripts_readme)?;
    check_catalog_generation_smoke()?;
    check_command_registry_embed_matches_disk(repo_root)?;
    check_catalog_feature_gates_match_registry(&reg)?;
    command_sync::verify(repo_root)?;
    check_root_readme_cli_drift(&root_readme)?;

    println!(
        "command-compliance OK (registry schema v{}, {} operations)",
        reg.schema_version,
        reg.operations.len()
    );
    Ok(())
}
