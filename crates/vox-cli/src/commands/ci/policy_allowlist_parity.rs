//! `vox ci policy-allowlist-parity` — verifies invariants between
//! `contracts/db/data-storage-policy.v1.yaml` (the SSOT) and the transitional
//! `docs/agents/turso-import-allowlist.txt`.
//!
//! Since Phase 3 of the data-storage audit (2026-05), the YAML is the single
//! source of truth: `load_turso_import_allowlist` auto-derives crates from
//! `tiers.a_relational.{owners, allow_direct_access, temporary_exceptions}`,
//! so the txt no longer needs to mirror them. This check therefore enforces:
//!   1. The policy YAML is parseable.
//!   2. Every txt entry points at an existing crate directory.
//!   3. The txt does not redundantly list crates that are already policy
//!      entries (the YAML wins; a redundant txt entry is dead config).
//!   4. Policy crates that don't exist on disk are warned (not failed) so the
//!      policy can list aspirational/scheduled-for-removal crates.

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::run_body::run_body_helpers::TURSO_BUILTIN_CRATES;

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
    owners: Vec<String>,
    #[serde(default)]
    allow_direct_access: Vec<String>,
    #[serde(default)]
    temporary_exceptions: Vec<String>,
}

pub fn run(root: &Path) -> Result<()> {
    let policy_path = root.join("contracts/db/data-storage-policy.v1.yaml");
    let yaml = fs::read_to_string(&policy_path)
        .with_context(|| format!("read {}", policy_path.display()))?;
    let policy: Policy = serde_yaml::from_str(&yaml).with_context(|| {
        format!(
            "parse data-storage policy at {} (expected `tiers.a_relational.{{allow_direct_access, temporary_exceptions}}`)",
            policy_path.display()
        )
    })?;

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
    policy_crates.extend(policy.tiers.a_relational.owners.iter().cloned());

    let crates_dir = root.join("crates");

    // New invariant: the txt allowlist must not duplicate policy owners (the
    // merged guard list already covers them via the policy YAML). Also, every
    // txt entry must point at an existing crate directory.
    for c in &allowlist_crates {
        if !crates_dir.join(c).is_dir() {
            return Err(anyhow!(
                "policy-allowlist-parity: docs/agents/turso-import-allowlist.txt lists \
                 `crates/{c}/` but that directory does not exist."
            ));
        }
        if policy_crates.contains(c) {
            return Err(anyhow!(
                "policy-allowlist-parity: docs/agents/turso-import-allowlist.txt lists \
                 `crates/{c}/` redundantly — `{c}` is already a policy owner. \
                 Remove the txt entry; the YAML is the source of truth."
            ));
        }
    }

    let mut warnings: Vec<String> = Vec::new();
    for c in &policy_crates {
        if TURSO_BUILTIN_CRATES.contains(&c.as_str()) {
            continue;
        }
        if !crates_dir.join(c).is_dir() {
            warnings.push(format!(
                "  policy lists `{c}` but `crates/{c}/` does not exist"
            ));
        }
    }

    for w in &warnings {
        eprintln!("WARN: policy-allowlist-parity:\n{w}");
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
    fn passes_when_policy_lists_crate_not_in_txt() {
        // Under the SSOT model, policy crates auto-populate the guard via
        // load_turso_import_allowlist; the txt no longer needs to mirror them.
        let td = tempdir().unwrap();
        let root = td.path();
        write(
            &root.join("contracts/db/data-storage-policy.v1.yaml"),
            "tiers:\n  a_relational:\n    allow_direct_access: [vox-db, vox-mystery]\n",
        );
        write(&root.join("docs/agents/turso-import-allowlist.txt"), "");
        fs::create_dir_all(root.join("crates/vox-mystery/src")).unwrap();
        run(root).expect("policy crate not in txt should be fine — YAML is SSOT");
    }

    #[test]
    fn fails_when_txt_redundantly_lists_policy_crate() {
        let td = tempdir().unwrap();
        let root = td.path();
        write(
            &root.join("contracts/db/data-storage-policy.v1.yaml"),
            "tiers:\n  a_relational:\n    owners: [vox-db, vox-secrets]\n    allow_direct_access: [vox-db, vox-secrets]\n",
        );
        write(
            &root.join("docs/agents/turso-import-allowlist.txt"),
            "crates/vox-secrets/\n",
        );
        fs::create_dir_all(root.join("crates/vox-secrets/src")).unwrap();
        let err = run(root).unwrap_err().to_string();
        assert!(
            err.contains("vox-secrets") && err.contains("redundantly"),
            "error must flag redundant txt entry; got: {err}"
        );
    }

    #[test]
    fn fails_when_txt_lists_nonexistent_crate() {
        let td = tempdir().unwrap();
        let root = td.path();
        write(
            &root.join("contracts/db/data-storage-policy.v1.yaml"),
            "tiers:\n  a_relational:\n    allow_direct_access: [vox-db]\n",
        );
        write(
            &root.join("docs/agents/turso-import-allowlist.txt"),
            "crates/vox-imaginary/\n",
        );
        let err = run(root).unwrap_err().to_string();
        assert!(
            err.contains("vox-imaginary") && err.contains("does not exist"),
            "error must flag nonexistent txt entry; got: {err}"
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
