//! `vox ci exec-policy-contract` — validate `exec-policy.v1.yaml` against JSON Schema,
//! then run smoke checks via both the pure-Rust fallback path (always) and the
//! PowerShell AST path (when `pwsh` is available).
//!
//! The hardcoded `SMOKE_PAYLOADS` / `REJECT_PAYLOADS` form the floor; an
//! optional disk corpus at `contracts/terminal/exec-policy.test-corpus.yaml`
//! is appended when present, so security-regression payloads can be added
//! without recompiling.

use std::path::Path;

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;

use crate::commands::ci::bounded_read::read_utf8_path_capped;
use crate::commands::runtime::shell::check_terminal;

/// Smoke payloads exercised by both the Rust-fallback and pwsh paths.
const SMOKE_PAYLOADS: &[(&str, &str)] = &[
    ("Get-Location", "bare cmdlet"),
    ("Write-Output 1 | ConvertTo-Json -Compress", "pipeline + cmdlet"),
    ("git status", "allowed binary"),
    ("cargo build --release", "allowed binary with long flag"),
];

/// Payloads that must be *rejected* by both paths.
const REJECT_PAYLOADS: &[(&str, &str)] = &[
    ("Get-ChildItem -Recurse", "blocked -Recurse parameter"),
    ("Totally-Fake-Cmdlet", "unknown command"),
];

/// Disk-loaded extension corpus path (relative to repo root).
const DISK_CORPUS_REL: &str = "contracts/terminal/exec-policy.test-corpus.yaml";

#[derive(Debug, Deserialize, Default)]
struct DiskCorpus {
    #[serde(default)]
    allow: Vec<DiskCorpusEntry>,
    #[serde(default)]
    deny: Vec<DiskCorpusEntry>,
}

#[derive(Debug, Deserialize)]
struct DiskCorpusEntry {
    payload: String,
    #[serde(default)]
    label: String,
}

/// Load the optional disk corpus.  Returns `Ok(None)` when the file is absent
/// (intentional — corpus is opt-in).
fn load_disk_corpus(repo_root: &Path) -> Result<Option<DiskCorpus>> {
    let path = repo_root.join(DISK_CORPUS_REL);
    if !path.is_file() {
        return Ok(None);
    }
    let src = read_utf8_path_capped(&path)
        .with_context(|| format!("read {}", path.display()))?;
    let corpus: DiskCorpus = serde_yaml::from_str(&src)
        .with_context(|| format!("parse {}", path.display()))?;
    Ok(Some(corpus))
}

/// Validate policy YAML + schema, then smoke-test both the pure-Rust fallback
/// path (always runs) and the PowerShell AST path (when `pwsh` is available).
pub fn run(repo_root: &Path) -> Result<()> {
    let policy = repo_root.join(check_terminal::DEFAULT_POLICY_REL);
    if !policy.is_file() {
        return Err(anyhow!("missing {}", policy.display()));
    }
    check_terminal::validate_policy_file(repo_root, &policy)
        .with_context(|| format!("validate {}", policy.display()))?;

    let disk = load_disk_corpus(repo_root)?;
    let disk_count = disk
        .as_ref()
        .map(|c| c.allow.len() + c.deny.len())
        .unwrap_or(0);

    // ── Rust-fallback path (always exercised, even on Windows with pwsh) ──
    for (payload, label) in SMOKE_PAYLOADS {
        check_terminal::run_check_for_ci(payload, Some(policy.as_path()))
            .with_context(|| format!("rust fallback smoke (allow): {label} — {payload:?}"))?;
    }
    for (payload, label) in REJECT_PAYLOADS {
        let result = check_terminal::run_check_for_ci(payload, Some(policy.as_path()));
        if result.is_ok() {
            return Err(anyhow!(
                "rust fallback should have rejected {label} ({payload:?}) but allowed it"
            ));
        }
    }
    if let Some(ref c) = disk {
        for entry in &c.allow {
            check_terminal::run_check_for_ci(&entry.payload, Some(policy.as_path()))
                .with_context(|| {
                    format!(
                        "rust fallback disk-corpus (allow): {} — {:?}",
                        entry.label, entry.payload
                    )
                })?;
        }
        for entry in &c.deny {
            let result = check_terminal::run_check_for_ci(&entry.payload, Some(policy.as_path()));
            if result.is_ok() {
                return Err(anyhow!(
                    "rust fallback (disk-corpus) should have rejected {} ({:?}) but allowed it",
                    entry.label,
                    entry.payload
                ));
            }
        }
    }
    println!(
        "exec-policy-contract: rust fallback OK ({} hardcoded + {} disk)",
        SMOKE_PAYLOADS.len() + REJECT_PAYLOADS.len(),
        disk_count
    );

    // ── PowerShell AST path (exercised when pwsh is available) ──
    match which::which("pwsh").or_else(|_| which::which("powershell")) {
        Ok(_) => {
            for (payload, label) in SMOKE_PAYLOADS {
                check_terminal::run_check(payload, Some(policy.as_path()))
                    .with_context(|| format!("pwsh AST smoke (allow): {label} — {payload:?}"))?;
            }
            for (payload, label) in REJECT_PAYLOADS {
                let result = check_terminal::run_check(payload, Some(policy.as_path()));
                if result.is_ok() {
                    return Err(anyhow!(
                        "pwsh AST path should have rejected {label} ({payload:?}) but allowed it"
                    ));
                }
            }
            if let Some(ref c) = disk {
                for entry in &c.allow {
                    check_terminal::run_check(&entry.payload, Some(policy.as_path()))
                        .with_context(|| {
                            format!(
                                "pwsh AST disk-corpus (allow): {} — {:?}",
                                entry.label, entry.payload
                            )
                        })?;
                }
                for entry in &c.deny {
                    let result = check_terminal::run_check(&entry.payload, Some(policy.as_path()));
                    if result.is_ok() {
                        return Err(anyhow!(
                            "pwsh AST (disk-corpus) should have rejected {} ({:?}) but allowed it",
                            entry.label,
                            entry.payload
                        ));
                    }
                }
            }
            println!("exec-policy-contract: pwsh AST OK");
            println!("exec-policy-contract OK (schema + rust fallback + pwsh smoke)");
        }
        Err(_) => {
            println!(
                "exec-policy-contract OK (schema + rust fallback; pwsh not on PATH — pwsh path skipped)"
            );
        }
    }
    Ok(())
}
