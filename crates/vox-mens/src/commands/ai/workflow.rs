use anyhow::Result;
use std::path::Path;
#[cfg(feature = "workflow-runtime")]
use std::sync::Arc;
#[cfg(feature = "workflow-runtime")]
use uuid::Uuid;
use vox_cli_core::cli_actions::WorkflowAction;

/// Dispatch workflow actions.
pub async fn run(action: WorkflowAction) -> Result<()> {
    match action {
        WorkflowAction::List { file } => list(&file).await,
        WorkflowAction::Inspect { file, name } => inspect(&file, &name).await,
        WorkflowAction::Check { file } => check(&file).await,
        WorkflowAction::Run {
            file,
            name,
            args,
            run_id,
            mesh,
        } => {
            run_workflow(
                &file,
                &name,
                args.as_deref().unwrap_or("[]"),
                run_id.as_deref(),
                mesh,
            )
            .await
        }
    }
}

const RETIRED_MSG: &str = "The `workflow` and `activity` primitives are retired (TASK-2.6). \
Use `@durable fn` decorator on a standard `fn` declaration instead. \
See AGENTS.md §Grammar Unification for details.";

/// List all workflows and activities defined in a .vox file.
pub async fn list(file: &Path) -> Result<()> {
    // Workflows and activities are retired; parser rejects source-level use.
    // This command now performs a type-check and reports the retirement notice.
    let result = crate::pipeline::run_frontend(file, false).await?;
    let errors = result.error_count();
    crate::pipeline::print_diagnostics(&result, file, false);
    if errors > 0 {
        anyhow::bail!("{} type error(s) found", errors);
    }
    println!("No workflows or activities found in {} — {RETIRED_MSG}", file.display());
    Ok(())
}

/// Show type-checked info about a specific workflow (retired).
pub async fn inspect(_file: &Path, _workflow_name: &str) -> Result<()> {
    anyhow::bail!("{RETIRED_MSG}");
}

/// Type-check a workflow file through the full Vox compiler pipeline.
pub async fn check(file: &Path) -> Result<()> {
    let result: crate::pipeline::FrontendResult = crate::pipeline::run_frontend(file, false)
        .await
        .map_err(|e| anyhow::anyhow!("Check failed: {}", e))?;

    let warnings = result.warning_count();
    let errors = result.error_count();

    crate::pipeline::print_diagnostics(&result, file, false);

    if errors == 0 {
        println!(
            "✓ {} — {} warning(s). Note: {RETIRED_MSG}",
            file.display(),
            warnings,
        );
        Ok(())
    } else {
        anyhow::bail!("{} type error(s) found", errors)
    }
}

/// Execute a workflow (retired — workflows are no longer a source-level primitive).
pub async fn run_workflow(
    _file: &Path,
    _workflow_name: &str,
    _args_json: &str,
    _requested_run_id: Option<&str>,
    _mesh: bool,
) -> Result<()> {
    anyhow::bail!("{RETIRED_MSG}");
}

#[cfg(feature = "workflow-runtime")]
fn resolve_plan_context_from_env() -> Option<(String, String, i64)> {
    let plan_session_id = std::env::var("VOX_PLAN_SESSION_ID")
        .ok()?
        .trim()
        .to_string();
    let plan_node_id = std::env::var("VOX_PLAN_NODE_ID").ok()?.trim().to_string();
    if plan_session_id.is_empty() || plan_node_id.is_empty() {
        return None;
    }
    let plan_version = std::env::var("VOX_PLAN_VERSION")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(1);
    Some((plan_session_id, plan_node_id, plan_version.max(1)))
}

#[cfg(feature = "workflow-runtime")]
fn resolve_workflow_run_id(workflow_name: &str, requested_run_id: Option<&str>) -> Result<String> {
    if let Some(run_id) = requested_run_id {
        let trimmed = run_id.trim();
        if trimmed.is_empty() {
            anyhow::bail!("--run-id must not be empty");
        }
        return Ok(trimmed.to_string());
    }
    Ok(format!("wf-{workflow_name}-{}", Uuid::new_v4()))
}
