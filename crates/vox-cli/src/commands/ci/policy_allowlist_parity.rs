//! `vox ci policy-allowlist-parity` — verifies that every crate in the
//! `tiers.a_relational.allow_direct_access` (and `temporary_exceptions`) list
//! of `contracts/db/data-storage-policy.v1.yaml` is reachable from
//! `docs/agents/turso-import-allowlist.txt` (plus the built-in guard prefixes).
//!
//! This is the mechanical drift-detector for the policy-guard split: when a
//! crate is added to the policy, the guard must be updated in the same PR.

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

const BUILTIN_PREFIXES: &[&str] = &["vox-db", "vox-package", "vox-compiler"];

#[derive(Debug, Deserialize)]
struct Policy {
    tiers: Tiers,
}

#[derive(Debug, Deserialize)]
struct Tiers {
    a_relational: TierA,
}

#[derive(Debug, Deserialize)]
struct TierA {
    #[serde(default)]
    allow_direct_access: Vec<String>,
    #[serde(default)]
    temporary_exceptions: Vec<String>,
}

pub fn run(root: &Path) -> Result<()> {
    let policy_path = root.join("contracts/db/data-storage-policy.v1.yaml");
    let yaml = fs::read_to_string(&policy_path)
        .with_context(|| format!("read {}", policy_path.display()))?;
    let policy: Policy = serde_yaml::from_str(&yaml).context("parse data-storage policy")?;

    let allowlist_path = root.join("docs/agents/turso-import-allowlist.txt");
    let allowlist_text = fs::read_to_string(&allowlist_path)
        .with_context(|| format!("read {}", allowlist_path.display()))?;
    let allowlist_crates: BTreeSet<String> = allowlist_text
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            l.strip_prefix("crates/")
                .map(|s| s.trim_end_matches('/').to_string())
        })
        .map(|s| s.split('/').next().unwrap_or(&s).to_string())
        .collect();

    let mut policy_crates: BTreeSet<String> = policy
        .tiers
        .a_relational
        .allow_direct_access
        .iter()
        .cloned()
        .collect();
    policy_crates.extend(policy.tiers.a_relational.temporary_exceptions.iter().cloned());

    let crates_dir = root.join("crates");
    let mut missing: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    for c in &policy_crates {
        if BUILTIN_PREFIXES.contains(&c.as_str()) {
            continue;
        }
        if !crates_dir.join(c).is_dir() {
            warnings.push(format!(
                "  policy lists `{c}` but `crates/{c}/` does not exist"
            ));
            continue;
        }
        if !allowlist_crates.contains(c) {
            missing.push(c.clone());
        }
    }

    for w in &warnings {
        eprintln!("WARN: policy-allowlist-parity:\n{w}");
    }

    if !missing.is_empty() {
        return Err(anyhow!(
            "policy-allowlist-parity: crates listed in data-storage-policy.v1.yaml \
             tiers.a_relational.allow_direct_access (or temporary_exceptions) but missing \
             from docs/agents/turso-import-allowlist.txt: {}. Add a `crates/<name>/` line \
             with a justification comment, OR remove from the policy if the crate no longer \
             needs direct turso access.",
            missing.join(", ")
        ));
    }

    let policy_count = policy_crates.len();
    let warn_count = warnings.len();
    if warn_count > 0 {
        println!(
            "policy-allowlist-parity OK ({policy_count} policy crates checked, \
             {warn_count} non-existent crate(s) warned)"
        );
    } else {
        println!("policy-allowlist-parity OK ({policy_count} policy crates checked)");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write(p: &std::path::Path, content: &str) {
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(p, content).unwrap();
    }

    #[test]
    fn fails_when_policy_lists_existing_crate_not_in_allowlist() {
        let td = tempdir().unwrap();
        let root = td.path();
        write(
            &root.join("contracts/db/data-storage-policy.v1.yaml"),
            "tiers:\n  a_relational:\n    allow_direct_access: [vox-db, vox-mystery]\n",
        );
        write(
            &root.join("docs/agents/turso-import-allowlist.txt"),
            "crates/vox-secrets/\n",
        );
        fs::create_dir_all(root.join("crates/vox-mystery/src")).unwrap();
        let err = run(root).unwrap_err().to_string();
        assert!(
            err.contains("vox-mystery"),
            "error must name the missing crate; got: {err}"
        );
    }

    #[test]
    fn passes_when_builtin_prefixes_cover_policy() {
        let td = tempdir().unwrap();
        let root = td.path();
        write(
            &root.join("contracts/db/data-storage-policy.v1.yaml"),
            "tiers:\n  a_relational:\n    allow_direct_access: [vox-db, vox-package, vox-compiler]\n",
        );
        write(&root.join("docs/agents/turso-import-allowlist.txt"), "");
        run(root).expect("builtin prefixes should satisfy parity");
    }

    #[test]
    fn warns_but_passes_when_policy_lists_nonexistent_crate() {
        let td = tempdir().unwrap();
        let root = td.path();
        write(
            &root.join("contracts/db/data-storage-policy.v1.yaml"),
            "tiers:\n  a_relational:\n    allow_direct_access: [vox-db, vox-aspirational]\n",
        );
        write(&root.join("docs/agents/turso-import-allowlist.txt"), "");
        run(root).expect("non-existent crate should warn, not fail");
    }
}
