//! Registry vs docs / lib.rs / compilerd / dei / script duals validators.

use anyhow::{Context, Result, anyhow};
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use walkdir::WalkDir;

use crate::commands::ci::bounded_read::read_utf8_path_capped;

use super::docs_sync::{markdown_section, ref_cli_vox_ci_section, ref_cli_vox_codex_section};
use super::registry::RegistryFile;
use crate::command_contract::{
    EMBEDDED_COMMAND_REGISTRY_YAML, merged_feature_gate_from_vox_cli_ops,
};
use vox_install_policy::{
    DEFAULT_RELEASE_GITHUB_OWNER, DEFAULT_RELEASE_GITHUB_REPO, SOURCE_INSTALL_CLI_REL_PATH,
    SUPPORTED_RELEASE_TARGETS,
};

/// Known `latin_ns` values in [`contracts/cli/command-registry.yaml`] for `surface: vox-cli`.
const KNOWN_LATIN_NS: &[&str] = &[
    "fabrica", "mens", "diag", "ars", "ci", "codex", "recensio", "dei", "pm",
];
const KNOWN_PRODUCT_LANES: &[&str] = &["app", "workflow", "ai", "interop", "data", "platform"];

fn normalize_lf(s: &str) -> String {
    s.replace("\r\n", "\n").replace('\r', "\n")
}

/// Fail when the registry on disk disagrees with the `include_str!` embed (stale `vox` binary).
pub(crate) fn check_command_registry_embed_matches_disk(repo_root: &Path) -> Result<()> {
    let p = repo_root.join(super::registry::REGISTRY_REL);
    let disk = read_utf8_path_capped(&p).with_context(|| format!("read {}", p.display()))?;
    if normalize_lf(&disk) != normalize_lf(EMBEDDED_COMMAND_REGISTRY_YAML) {
        return Err(anyhow!(
            "{} does not match the vox-cli embedded registry — rebuild with `cargo build -p vox-cli` so `include_str!` picks up edits",
            p.display()
        ));
    }
    Ok(())
}

/// Catalog `feature_gate` must match merged registry rows for each path (SSOT).
pub(crate) fn check_catalog_feature_gates_match_registry(reg: &RegistryFile) -> Result<()> {
    let vox_cli: Vec<crate::command_registry_model::RegistryOperation> = reg
        .operations
        .iter()
        .filter(|o| o.surface == "vox-cli")
        .cloned()
        .collect();
    let catalog = crate::command_catalog::build_catalog();
    for e in &catalog.entries {
        let merged = merged_feature_gate_from_vox_cli_ops(&vox_cli, &e.path);
        if merged != e.feature_gate {
            return Err(anyhow!(
                "command catalog feature_gate {:?} for path {:?} != registry merge {:?}",
                e.feature_gate,
                e.path,
                merged
            ));
        }
    }
    Ok(())
}

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

    // SSOT drift prevention: ensure README doesn't duplicate long-form content from docs/src/index.md
    let forbidden_long_form_headers = [
        "## What Vox Saves You",
        "### No Duplicate Type Definitions",
        "### No Separate API Layer",
        "### No Null Checks, No Runtime Surprises",
        "### Durable Workflows That Survive Crashes",
        "### Persistent Actors",
        "### AI Agents out-of-the-box",
        "## Installation",
    ];
    for header in forbidden_long_form_headers {
        if readme.contains(header) {
            return Err(anyhow!(
                "README.md contains forbidden long-form content header `{header}`. This content belongs in docs/src/index.md (SSOT)."
            ));
        }
    }

    if !readme.contains("https://vox-lang.org") {
        return Err(anyhow!(
            "README.md must link to the canonical domain https://vox-lang.org"
        ));
    }

    let section = markdown_section(readme, "## The CLI").ok_or_else(|| {
        anyhow!("README.md is missing `## The CLI` section required for discoverability")
    })?;
    for stale_cmd in [
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
                "command-registry env_var_ssot_index: {needle} not found in docs/src/reference/env-vars.md (or env-vars-ssot.md)"
            ));
        }
    }
    Ok(())
}

fn env_var_call_regexes() -> (&'static Regex, &'static Regex) {
    static CALL: OnceLock<Regex> = OnceLock::new();
    static OPTION: OnceLock<Regex> = OnceLock::new();
    let call = CALL.get_or_init(|| {
        Regex::new(r#"(?:std::)?env::var(?:_os)?\(\s*"([^"]+)""#).expect("env::var literal regex")
    });
    let option = OPTION.get_or_init(|| {
        Regex::new(r#"option_env!\(\s*"([^"]+)""#).expect("option_env literal regex")
    });
    (call, option)
}

fn should_skip_scanned_env_name(name: &str) -> bool {
    if name.len() < 2 {
        return true;
    }
    if !name.chars().next().is_some_and(|c| c.is_ascii_uppercase()) {
        return true;
    }
    for prefix in ["CARGO_", "RUST_", "DEP_", "OUT_", "LLVM_", "DYLD_"] {
        if name.starts_with(prefix) {
            return true;
        }
    }
    matches!(
        name,
        "PATH"
            | "HOME"
            | "USER"
            | "USERPROFILE"
            | "HOMEDRIVE"
            | "HOMEPATH"
            | "USERNAME"
            | "SYSTEMROOT"
            | "WINDIR"
            | "COMSPEC"
            | "PATHEXT"
            | "LOCALAPPDATA"
            | "APPDATA"
            | "TMP"
            | "TEMP"
            | "TMPDIR"
            | "PWD"
            | "OLDPWD"
            | "SHLVL"
            | "TERM"
            | "NUMBER_OF_PROCESSORS"
    )
}

fn collect_env_var_names_from_rs_source(src: &str, found: &mut HashSet<String>) {
    let (call_re, opt_re) = env_var_call_regexes();
    for cap in call_re.captures_iter(src) {
        if let Some(m) = cap.get(1) {
            found.insert(m.as_str().to_string());
        }
    }
    for cap in opt_re.captures_iter(src) {
        if let Some(m) = cap.get(1) {
            found.insert(m.as_str().to_string());
        }
    }
}

/// Tier-1 guard: `std::env::var("…")` / `option_env!` string literals in `vox-db`, `vox-mcp`, and
/// selected orchestrator modules must appear in the env-var SSOT doc as `` `NAME` ``.
pub(crate) fn check_tier1_env_vars_documented(repo_root: &Path, env_ssot_md: &str) -> Result<()> {
    let mut found: HashSet<String> = HashSet::new();

    for rel in ["crates/vox-db/src", "crates/vox-mcp/src"] {
        let root = repo_root.join(rel);
        if !root.is_dir() {
            continue;
        }
        for entry in WalkDir::new(&root)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                name != "target" && !name.starts_with('.')
            })
            .filter_map(|e| e.ok())
        {
            let p = entry.path();
            if !p.is_file() {
                continue;
            }
            if p.extension().and_then(|s| s.to_str()) != Some("rs") {
                continue;
            }
            let rel_path = p.strip_prefix(repo_root).unwrap_or(p);
            let s = rel_path.to_string_lossy();
            if s.contains("/tests/") || s.contains("\\tests\\") {
                continue;
            }
            let src = read_utf8_path_capped(p).with_context(|| format!("read {}", p.display()))?;
            collect_env_var_names_from_rs_source(&src, &mut found);
        }
    }

    for rel in [
        "crates/vox-orchestrator/src/usage.rs",
        "crates/vox-orchestrator/src/usage_policy.rs",
        "crates/vox-orchestrator/src/lineage.rs",
        "crates/vox-orchestrator/src/models/spec.rs",
    ] {
        let p = repo_root.join(rel);
        if p.is_file() {
            let src = read_utf8_path_capped(&p).with_context(|| format!("read {}", p.display()))?;
            collect_env_var_names_from_rs_source(&src, &mut found);
        }
    }

    let mut missing: Vec<String> = found
        .into_iter()
        .filter(|n| !should_skip_scanned_env_name(n))
        .filter(|n| {
            let needle = format!("`{n}`");
            !env_ssot_md.contains(&needle)
        })
        .collect();
    missing.sort();
    if !missing.is_empty() {
        return Err(anyhow!(
            "env SSOT: the following env vars are read in tier-1 crates but missing from docs (use backticks `NAME`): {}",
            missing.join(", ")
        ));
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
        if let Some(ref lane) = op.product_lane {
            if !KNOWN_PRODUCT_LANES.contains(&lane.as_str()) {
                return Err(anyhow!(
                    "command-registry: unknown product_lane `{lane}` for vox-cli path {:?}",
                    op.path
                ));
            }
        } else {
            return Err(anyhow!(
                "command-registry: missing product_lane for vox-cli path {:?} (bell-curve lane SSOT)",
                op.path
            ));
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

/// Release/install SSOT (`vox-install-policy`) matches docs and key Rust entrypoints.
pub(crate) fn check_install_policy_surfaces(repo_root: &Path) -> Result<()> {
    let contract_path = repo_root.join("docs/src/ci/binary-release-contract.md");
    let contract = read_utf8_path_capped(&contract_path)
        .with_context(|| format!("read {}", contract_path.display()))?;
    for triple in SUPPORTED_RELEASE_TARGETS {
        if !contract.contains(triple) {
            return Err(anyhow!(
                "{}: missing release target `{triple}` (must match `vox_install_policy::SUPPORTED_RELEASE_TARGETS`)",
                contract_path.display()
            ));
        }
    }
    let org_repo = format!("{DEFAULT_RELEASE_GITHUB_OWNER}/{DEFAULT_RELEASE_GITHUB_REPO}");
    if !contract.contains(&org_repo) {
        return Err(anyhow!(
            "{}: must document default GitHub coordinates `{org_repo}`",
            contract_path.display()
        ));
    }
    if !contract.contains("--locked") {
        return Err(anyhow!(
            "{}: source fallback must document `cargo install --locked`",
            contract_path.display()
        ));
    }
    if !contract.contains(SOURCE_INSTALL_CLI_REL_PATH) {
        return Err(anyhow!(
            "{}: must document install path `{}`",
            contract_path.display(),
            SOURCE_INSTALL_CLI_REL_PATH
        ));
    }

    let bootstrap_install = repo_root.join("crates/vox-bootstrap/src/engine/install.rs");
    let bootstrap_txt = read_utf8_path_capped(&bootstrap_install)
        .with_context(|| format!("read {}", bootstrap_install.display()))?;
    if !bootstrap_txt.contains("vox_install_policy::") {
        return Err(anyhow!(
            "{}: must delegate install policy to `vox_install_policy` (avoid drift with bootstrap)",
            bootstrap_install.display()
        ));
    }
    if !bootstrap_txt.contains("CARGO_INSTALL_CLI_FROM_SOURCE") {
        return Err(anyhow!(
            "{}: source install must use `vox_install_policy::CARGO_INSTALL_CLI_FROM_SOURCE` (includes `--locked`)",
            bootstrap_install.display()
        ));
    }

    let repo_up = repo_root.join("crates/vox-cli/src/commands/repo_upgrade.rs");
    let repo_up_txt =
        read_utf8_path_capped(&repo_up).with_context(|| format!("read {}", repo_up.display()))?;
    if !repo_up_txt.contains("vox_install_policy::") {
        return Err(anyhow!(
            "{}: must import `vox_install_policy` for `cargo install` argv + layout checks",
            repo_up.display()
        ));
    }

    let tu = repo_root.join("crates/vox-cli/src/commands/toolchain_upgrade.rs");
    let tu_txt = read_utf8_path_capped(&tu).with_context(|| format!("read {}", tu.display()))?;
    if !tu_txt.contains("vox_install_policy::") {
        return Err(anyhow!(
            "{}: must import `vox_install_policy` for default GitHub release coordinates",
            tu.display()
        ));
    }

    if !repo_up_txt.contains("CARGO_INSTALL_CLI_FROM_SOURCE") {
        return Err(anyhow!(
            "{}: `cargo install` must use `CARGO_INSTALL_CLI_FROM_SOURCE` from `vox_install_policy`",
            repo_up.display()
        ));
    }

    println!("install-policy surfaces OK (vox-install-policy ↔ docs ↔ bootstrap/repo upgrade)");
    Ok(())
}

/// Project dependency commands (`add` / `remove` / `update` / `lock` / `sync`) must not invoke the
/// Vox toolchain self-upgrade lane (WP5 symmetry with [`check_upgrade_toolchain_only`]).
pub(crate) fn check_project_pm_commands_no_toolchain_lane(repo_root: &Path) -> Result<()> {
    const FILES: &[&str] = &[
        "crates/vox-cli/src/commands/add.rs",
        "crates/vox-cli/src/commands/remove.rs",
        "crates/vox-cli/src/commands/update.rs",
        "crates/vox-cli/src/commands/lock.rs",
        "crates/vox-cli/src/commands/sync.rs",
    ];
    const FORBIDDEN: &[&str] = &[
        "toolchain_upgrade",
        "run_toolchain_upgrade",
        "repo_upgrade",
        "self_update",
        "UpgradeToolchainArgs",
        "vox_install_policy::",
    ];
    for rel in FILES {
        let p = repo_root.join(rel);
        let s = read_utf8_path_capped(&p).with_context(|| format!("read {}", p.display()))?;
        for needle in FORBIDDEN {
            if s.contains(needle) {
                return Err(anyhow!(
                    "{}: project PM command file must not reference toolchain upgrade / install-policy (`{needle}`) — use only `vox_pm` / `pm_lifecycle` (WP5)",
                    p.display()
                ));
            }
        }
    }
    Ok(())
}

/// `vox upgrade` must not import or call project PM / lockfile APIs (WP5 namespace split).
pub(crate) fn check_upgrade_toolchain_only(repo_root: &Path) -> Result<()> {
    let p = repo_root.join("crates/vox-cli/src/commands/upgrade.rs");
    let s = read_utf8_path_capped(&p).with_context(|| format!("read {}", p.display()))?;
    for needle in [
        "vox_pm::",
        "VoxManifest",
        "Lockfile",
        "open_local_pm_store",
        "lockfile_path",
    ] {
        if s.contains(needle) {
            return Err(anyhow!(
                "{}: `vox upgrade` must not touch project dependency state — remove `{needle}` from upgrade path (use `vox update` / `vox sync` instead)",
                p.display()
            ));
        }
    }
    Ok(())
}

/// When a Dockerfile copies `Cargo.lock`, every `cargo build` line must pass `--locked` (WP7).
pub(crate) fn check_dockerfiles_cargo_locked_policy(repo_root: &Path) -> Result<()> {
    let mut dockerfiles: Vec<PathBuf> = Vec::new();
    let root_df = repo_root.join("Dockerfile");
    if root_df.is_file() {
        dockerfiles.push(root_df);
    }
    let docker_dir = repo_root.join("docker");
    if docker_dir.is_dir() {
        for e in fs::read_dir(&docker_dir)
            .with_context(|| format!("read_dir {}", docker_dir.display()))?
        {
            let p = e?.path();
            if p.is_file()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("Dockerfile"))
            {
                dockerfiles.push(p);
            }
        }
    }
    dockerfiles.sort();
    for p in dockerfiles {
        let s = read_utf8_path_capped(&p).with_context(|| format!("read {}", p.display()))?;
        let copies_lock = s.lines().any(|line| {
            let t = line.trim_start();
            t.starts_with("COPY ") && t.contains("Cargo.lock")
        });
        if !copies_lock {
            continue;
        }
        for line in s.lines() {
            let t = line.trim();
            if t.starts_with('#') || t.is_empty() {
                continue;
            }
            if t.contains("cargo build") && !t.contains("--locked") {
                return Err(anyhow!(
                    "{}: use `cargo build ... --locked` whenever `Cargo.lock` is copied (container reproducibility policy)",
                    p.display()
                ));
            }
        }
    }
    Ok(())
}

/// Forbid legacy “run/use `vox install` for packages” nudges outside explicit migration/arch pages (WP4).
pub(crate) fn check_operator_docs_no_legacy_vox_install_pm_nudge(repo_root: &Path) -> Result<()> {
    use std::fs;
    const BAD: &[&str] = &[
        "run `vox install`",
        "run vox install",
        "`vox install` first",
        "vox install first",
    ];
    let docs_src = repo_root.join("docs/src");
    if !docs_src.is_dir() {
        return Ok(());
    }
    fn allowed(rel_posix: &str) -> bool {
        rel_posix.contains("architecture/vox-packaging")
            || rel_posix == "reference/pm-migration-2026.md"
            || rel_posix == "reference/cli.md"
    }
    fn walk(dir: &Path, docs_src: &Path, bad: &[&str]) -> Result<()> {
        for e in fs::read_dir(dir).with_context(|| format!("read_dir {}", dir.display()))? {
            let e = e?;
            let p = e.path();
            if p.is_dir() {
                walk(&p, docs_src, bad)?;
            } else if p.extension().and_then(|x| x.to_str()) == Some("md") {
                let rel_posix = p
                    .strip_prefix(docs_src)
                    .unwrap_or(&p)
                    .to_string_lossy()
                    .replace('\\', "/");
                if allowed(&rel_posix) {
                    continue;
                }
                let s =
                    read_utf8_path_capped(&p).with_context(|| format!("read {}", p.display()))?;
                let lower = s.to_lowercase();
                for b in bad {
                    if lower.contains(&b.to_lowercase()) {
                        return Err(anyhow!(
                            "{}: forbidden legacy PM phrase `{b}` (WP4) — document `vox add` / `vox lock` / `vox sync` / `vox pm` instead; migration table: docs/src/reference/pm-migration-2026.md",
                            p.display()
                        ));
                    }
                }
            }
        }
        Ok(())
    }
    walk(&docs_src, &docs_src, BAD)?;
    Ok(())
}

/// Forbid resurrecting retired `vox container init` / uv product copy in user-facing bridge docs (WP6).
pub(crate) fn check_packaging_pm_docs_no_resurrected_uv_copies(repo_root: &Path) -> Result<()> {
    const PATHS: &[&str] = &[
        "docs/src/how-to/how-to-pytorch.md",
        "docs/src/api/vox-py.md",
    ];
    const BAD: &[&str] = &[
        "vox container init handles everything",
        "Local development — do nothing; .venv is found automatically after `uv sync`",
    ];
    for rel in PATHS {
        let p = repo_root.join(rel);
        let s = read_utf8_path_capped(&p).with_context(|| format!("read {}", p.display()))?;
        for frag in BAD {
            if s.contains(frag) {
                return Err(anyhow!(
                    "{}: forbidden doc fragment `{frag}` — keep Python/uv paths explicitly historical/retired",
                    p.display()
                ));
            }
        }
    }
    Ok(())
}

/// `feature-growth-boundaries.md` must name the triplet parity integration test and cargo invocation
/// so LLM/doc edits do not drop the canonical drift gate.
pub(crate) fn check_feature_growth_boundaries_projection_gate(repo_root: &Path) -> Result<()> {
    let p = repo_root.join("docs/src/architecture/feature-growth-boundaries.md");
    let s = read_utf8_path_capped(&p).with_context(|| format!("read {}", p.display()))?;
    for needle in [
        "projection_parity",
        "projection_triplet_is_deterministic",
        "cargo test -p vox-compiler --test projection_parity",
    ] {
        if !s.contains(needle) {
            return Err(anyhow!(
                "{} must document the WebIR/AppContract/RuntimeProjection drift gate (`{needle}` missing)",
                p.display()
            ));
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
