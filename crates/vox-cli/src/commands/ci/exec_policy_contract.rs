//! `vox ci exec-policy-contract` — validate `exec-policy.v1.yaml` against JSON Schema and optionally smoke `vox shell check`.

use std::path::Path;

use anyhow::{Context, Result, anyhow};

use crate::commands::runtime::shell::check_terminal;

/// Validate policy YAML + schema; when `pwsh` is available, run AST check on `Get-Location`.
pub fn run(repo_root: &Path) -> Result<()> {
    let policy = repo_root.join(check_terminal::DEFAULT_POLICY_REL);
    if !policy.is_file() {
        return Err(anyhow!("missing {}", policy.display()));
    }
    let _loaded = check_terminal::validate_policy_file(repo_root, &policy)
        .with_context(|| format!("validate {}", policy.display()))?;

    match which::which("pwsh").or_else(|_| which::which("powershell")) {
        Ok(_) => {
            check_terminal::run_check("Get-Location", Some(policy.as_path()))
                .context("pwsh AST smoke check (Get-Location)")?;
            check_terminal::run_check(
                "Write-Output 1 | ConvertTo-Json -Compress",
                Some(policy.as_path()),
            )
            .context("pwsh AST smoke check (pipeline + ConvertTo-Json)")?;
            println!("exec-policy-contract OK (schema + pwsh smoke)");
        }
        Err(_) => {
            println!("exec-policy-contract OK (schema only; pwsh not on PATH — skipped AST smoke)");
        }
    }
    Ok(())
}
