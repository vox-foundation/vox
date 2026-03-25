//! `vox mens review` — delegates to `vox-dei-d` (`ai.review`).

use anyhow::Result;
use std::path::PathBuf;

/// Run AI review over `targets` via the DeI daemon.
#[allow(clippy::too_many_arguments)]
pub async fn run(
    targets: &[PathBuf],
    model: Option<&str>,
    format: Option<&str>,
    severity: Option<&str>,
    free_only: bool,
    use_diff: bool,
    ci: bool,
    pr_comment: bool,
    diff_base: Option<&str>,
) -> Result<()> {
    let targets: Vec<String> = if targets.is_empty() {
        vec![".".to_string()]
    } else {
        targets.iter().map(|p| p.display().to_string()).collect()
    };

    crate::dei_daemon::call(
        crate::dei_daemon::method::AI_REVIEW,
        serde_json::json!({
            "targets": targets,
            "model": model,
            "format": format,
            "severity": severity,
            "free_only": free_only,
            "diff": use_diff,
            "ci": ci,
            "pr_comment": pr_comment,
            "diff_base": diff_base,
        }),
        false,
    )
    .await?;

    Ok(())
}
