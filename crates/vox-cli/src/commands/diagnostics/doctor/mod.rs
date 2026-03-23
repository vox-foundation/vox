//! `vox doctor` — check the development environment is ready.

#[cfg(feature = "codex")]
mod checks_codex;
mod checks_standard;
mod common;
mod output;
mod provider_policy;

use anyhow::Result;

/// Run the `vox doctor` environment check and health audit.
pub async fn run(
    auto_heal: bool,
    test_health: bool,
    build_perf: bool,
    scope: bool,
    json: bool,
) -> Result<()> {
    #[cfg(not(feature = "codex"))]
    if build_perf || scope || json {
        anyhow::bail!(
            "`vox doctor` with --build-perf, --scope, or --json requires the extended build: \
             `cargo build -p vox-cli --features codex` (wires `commands::diagnostics::doctor`)."
        );
    }

    println!(
        "vox doctor — checking your environment{}",
        if auto_heal {
            " (auto-healing enabled)"
        } else {
            ""
        }
    );
    println!();

    let mut checks: Vec<common::Check> = Vec::new();

    #[cfg(feature = "codex")]
    if build_perf {
        checks_codex::run_build_perf(json).await?;
        return Ok(());
    }

    #[cfg(feature = "codex")]
    if scope {
        checks_codex::run_scope(json).await?;
        return Ok(());
    }

    checks_standard::run_checks(auto_heal, test_health, &mut checks).await;

    output::print_results(&checks, test_health, json);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[cfg(not(feature = "codex"))]
    async fn extended_doctor_flags_require_codex_build() {
        let err = run(false, false, true, false, false)
            .await
            .expect_err("build_perf without codex doctor should error");
        let s = err.to_string();
        assert!(
            s.contains("codex") && s.contains("doctor"),
            "unexpected message: {s}"
        );
    }

    #[tokio::test]
    #[cfg(feature = "codex")]
    async fn build_perf_runs_when_codex_enabled() {
        let r = run(false, false, true, false, false).await;
        assert!(r.is_ok(), "expected build_perf path to complete: {r:?}");
    }
}
