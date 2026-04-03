use anyhow::{Context, Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::LazyLock;

use super::guards::run_sql_surface_guard;
use crate::commands::ci::bounded_read::read_utf8_path_capped;
use crate::commands::ci::cargo_bin;
use crate::commands::ci::command_compliance;
use crate::commands::ci::completion_quality;
use crate::commands::ci::constants::{
    CODEX_SSOT_FILES, DOCS_SSOT_FILES, MANIFEST_SNIPPETS, OPENAPI_SUBSTRINGS,
};
use crate::commands::ci::contracts_index;
use crate::commands::ci::exec_policy_contract;
use crate::commands::ci::scientia_novelty_ledger_contract;
use crate::commands::ci::scientia_worthiness_contract;

pub(crate) fn run_manifest(root: &Path) -> Result<()> {
    let status = Command::new(cargo_bin())
        .current_dir(root)
        .args(["metadata", "--locked", "--format-version", "1", "--no-deps"])
        .stdout(Stdio::null())
        .status()
        .context("spawn cargo metadata")?;
    if !status.success() {
        return Err(anyhow!("cargo metadata --locked failed"));
    }
    println!("OK: workspace manifest resolves (cargo metadata --locked --no-deps)");
    Ok(())
}

pub(crate) fn check_docs_ssot(root: &Path) -> Result<()> {
    for rel in DOCS_SSOT_FILES {
        let p = root.join(rel);
        if !p.is_file() {
            return Err(anyhow!("missing: {}", p.display()));
        }
    }
    let doc_inv = root.join("docs/agents/doc-inventory.json");
    if !doc_inv.is_file() {
        return Err(anyhow!(
            "missing: {} (run: vox ci doc-inventory generate)",
            doc_inv.display()
        ));
    }
    let raw = read_utf8_path_capped(&doc_inv)?;
    let v: serde_json::Value = serde_json::from_str(&raw)?;
    let sv = v
        .get("schema_version")
        .and_then(|x| x.as_i64())
        .unwrap_or(0);
    if sv < 3 {
        return Err(anyhow!("doc-inventory.json: expected schema_version >= 3"));
    }

    let inv = root.join("docs/src/architecture/orphan-surface-inventory.md");
    let inv_text = read_utf8_path_capped(&inv)?;
    if !inv_text.contains("workspace-crates-start") {
        return Err(anyhow!(
            "orphan inventory: missing workspace-crates-start marker"
        ));
    }
    if !inv_text.contains("workspace-crates-end") {
        return Err(anyhow!(
            "orphan inventory: missing workspace-crates-end marker"
        ));
    }

    let listed = parse_workspace_crate_block(&inv_text);
    let crates_dir = root.join("crates");
    for entry in
        fs::read_dir(&crates_dir).with_context(|| format!("read {}", crates_dir.display()))?
    {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let toml = entry.path().join("Cargo.toml");
        if !toml.is_file() {
            continue;
        }
        let name = read_package_name(&toml)?;
        if !listed.contains(&name) {
            return Err(anyhow!(
                "orphan inventory workspace crate list missing: {name} (from {})",
                toml.display()
            ));
        }
    }

    check_stale_doc_and_workflow_refs(root)?;

    println!("Docs SSOT guard OK");
    Ok(())
}

/// Fail if docs or GitHub workflows reference retired Python inventory paths or shell gates.
fn check_stale_doc_and_workflow_refs(root: &Path) -> Result<()> {
    const WORKFLOW_BANNED: &[&str] = &["verify_doc_inventory_fresh.py", "populi_release_gate.sh"];
    const DOC_BANNED: &[&str] = &["verify_doc_inventory_fresh.py", "populi_release_gate.sh"];
    // Retired crate paths / broken SSOT links — see `docs/src/architecture/nomenclature-migration-map.md`.
    const NOMENCLATURE_DOC_BANNED: &[&str] = &[
        "reference/mens.md",
        "reference/mens-ssot.md",
        "crates/vox-mens/",
        "crates/vox-codex-api/",
    ];
    const DOC_PATH_BANNED: &[&str] = &["docs/how-to-ai-agents.md", "docs/src/how-to-ai-agents.md"];

    let wf_dir = root.join(".github/workflows");
    if wf_dir.is_dir() {
        for entry in fs::read_dir(&wf_dir).with_context(|| format!("read {}", wf_dir.display()))? {
            let entry = entry?;
            let p = entry.path();
            if p.extension().and_then(|x| x.to_str()) != Some("yml")
                && p.extension().and_then(|x| x.to_str()) != Some("yaml")
            {
                continue;
            }
            let text = read_utf8_path_capped(&p)?;
            for b in WORKFLOW_BANNED {
                if text.contains(b) {
                    return Err(anyhow!(
                        "{}: stale or retired reference {:?} (use `vox ci` guards; see docs/src/ci/doc-inventory-ssot.md)",
                        p.display(),
                        b
                    ));
                }
            }
        }
    }

    let docs_dir = root.join("docs");
    if docs_dir.is_dir() {
        let mut files = Vec::new();
        collect_text_files_under(&docs_dir, &mut files)?;
        for rel in ["README.md", "AGENTS.md", "CONTRIBUTING.md"] {
            let p = root.join(rel);
            if p.is_file() {
                files.push(p);
            }
        }
        for p in files {
            let ext = p.extension().and_then(|x| x.to_str());
            if ext != Some("md") && ext != Some("yml") && ext != Some("yaml") {
                continue;
            }
            let text = read_utf8_path_capped(&p)?;
            for b in DOC_BANNED {
                if text.contains(b) {
                    return Err(anyhow!(
                        "{}: stale reference {:?} — removed from tree; update docs",
                        p.display(),
                        b
                    ));
                }
            }
            for b in NOMENCLATURE_DOC_BANNED {
                if text.contains(b) {
                    return Err(anyhow!(
                        "{}: nomenclature drift {:?} — use canonical crate paths (see docs/src/architecture/nomenclature-migration-map.md)",
                        p.display(),
                        b
                    ));
                }
            }
            for b in DOC_PATH_BANNED {
                if text.contains(b) {
                    return Err(anyhow!(
                        "{}: stale docs path {:?} — link the canonical mdBook path instead",
                        p.display(),
                        b
                    ));
                }
            }
        }
    }

    println!("stale doc/workflow ref scan OK");
    Ok(())
}

fn collect_text_files_under(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let entry = entry?;
        let p = entry.path();
        let t = entry.file_type()?;
        if t.is_dir() {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == "target" || name == ".git" || name == "book" || name == "theme" {
                continue;
            }
            collect_text_files_under(&p, out)?;
        } else if t.is_file() {
            out.push(p);
        }
    }
    Ok(())
}

static CRATE_LINE_RE: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^[a-z0-9_-]+$").expect("CRATE_LINE_RE"));

fn parse_workspace_crate_block(md: &str) -> std::collections::HashSet<String> {
    use std::collections::HashSet;
    let mut out = HashSet::new();
    let mut in_block = false;
    for line in md.lines() {
        let t = line.trim_end();
        if t.contains("workspace-crates-start") {
            in_block = true;
            continue;
        }
        if t.contains("workspace-crates-end") {
            in_block = false;
            continue;
        }
        if in_block {
            let s = t.trim();
            if CRATE_LINE_RE.is_match(s) {
                out.insert(s.to_string());
            }
        }
    }
    out
}

fn read_package_name(toml_path: &Path) -> Result<String> {
    let text = read_utf8_path_capped(toml_path)?;
    let v: toml::Value =
        toml::from_str(&text).with_context(|| format!("parse TOML {}", toml_path.display()))?;
    v.get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(str::to_string)
        .ok_or_else(|| anyhow!("could not read package.name from {}", toml_path.display()))
}

fn verify_baseline_policy_alignment(root: &Path) -> Result<()> {
    let policy_path = root.join("contracts/db/baseline-version-policy.yaml");
    let raw = read_utf8_path_capped(&policy_path)
        .with_context(|| format!("read {}", policy_path.display()))?;
    let v: serde_yaml::Value =
        serde_yaml::from_str(&raw).with_context(|| format!("parse {}", policy_path.display()))?;
    let expected = v
        .get("policy")
        .and_then(|p| p.get("repository_baseline_integer"))
        .and_then(|x| x.as_i64())
        .ok_or_else(|| {
            anyhow!(
                "{}: missing policy.repository_baseline_integer",
                policy_path.display()
            )
        })?;
    let manifest_path = root.join("crates/vox-db/src/schema/manifest.rs");
    let man = read_utf8_path_capped(&manifest_path)
        .with_context(|| format!("read {}", manifest_path.display()))?;
    let re = regex::Regex::new(r"pub const BASELINE_VERSION:\s*i64\s*=\s*(\d+)")
        .expect("BASELINE_VERSION parse regex");
    let got = man
        .lines()
        .find_map(|line| {
            re.captures(line)
                .and_then(|c| c.get(1)?.as_str().parse::<i64>().ok())
        })
        .ok_or_else(|| {
            anyhow!(
                "could not parse BASELINE_VERSION from {}",
                manifest_path.display()
            )
        })?;
    if got != expected {
        return Err(anyhow!(
            "baseline mismatch: {} has repository_baseline_integer={expected}, {} has BASELINE_VERSION={got}",
            policy_path.display(),
            manifest_path.display()
        ));
    }
    let digest_expected = v
        .get("policy")
        .and_then(|p| p.get("repository_baseline_digest_hex"))
        .and_then(|x| x.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if let Some(ed) = digest_expected {
        let digest_got = vox_db::schema::schema_baseline_digest_hex();
        if digest_got != ed {
            return Err(anyhow!(
                "baseline digest mismatch: {} policy.repository_baseline_digest_hex={ed:?}, vox_db baseline_sql Keccak256={digest_got:?} (update the contract when SCHEMA_FRAGMENTS or schema::spec DDL changes)",
                policy_path.display()
            ));
        }
    }
    Ok(())
}

pub(crate) fn run_ssot_drift(root: &Path) -> Result<()> {
    check_docs_ssot(root)?;
    check_codex_ssot(root)?;
    // Full-workspace scan; transitional allowlist in docs/agents/sql-connection-api-allowlist.txt
    run_sql_surface_guard(root, true)?;
    crate::commands::ci::nomenclature_guard::run(root, false)?;
    crate::commands::ci::operations_catalog::verify(root)?;
    command_compliance::run(root)?;
    crate::commands::ci::capability_sync::run(root, false)?;
    contracts_index::run(root)?;
    exec_policy_contract::run(root)?;
    completion_quality::run_audit_verify_ssot(root)?;
    scientia_worthiness_contract::run(root)?;
    scientia_novelty_ledger_contract::run(root)?;
    super::run_data_ssot_guards(root)?;
    println!("ssot-drift: nested SSOT guards OK");
    Ok(())
}

pub(crate) fn check_codex_ssot(root: &Path) -> Result<()> {
    for rel in CODEX_SSOT_FILES {
        let p = root.join(rel);
        if !p.is_file() {
            return Err(anyhow!("missing: {}", p.display()));
        }
    }
    let m = root.join("crates/vox-db/src/schema/manifest.rs");
    let manifest = read_utf8_path_capped(&m).with_context(|| format!("read {}", m.display()))?;
    for needle in MANIFEST_SNIPPETS {
        if !manifest.contains(needle) {
            return Err(anyhow!("{} must contain substring: {needle}", m.display()));
        }
    }
    verify_baseline_policy_alignment(root)?;
    let o = root.join("contracts/codex-api.openapi.yaml");
    let o_text = read_utf8_path_capped(&o)?;
    for needle in OPENAPI_SUBSTRINGS {
        if !o_text.contains(needle) {
            return Err(anyhow!("openapi guard failed: missing {needle}"));
        }
    }
    println!("Codex SSOT doc guard OK");
    Ok(())
}
