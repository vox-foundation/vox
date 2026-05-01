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

/// List all workflows and activities defined in a .vox file.
///
/// Compiles the file, then reports all functions with `DurabilityKind::Workflow`
/// or `DurabilityKind::Activity`.
pub async fn list(file: &Path) -> Result<()> {
    use vox_compiler::hir::nodes::DurabilityKind;

    let result = crate::pipeline::run_frontend(file, false).await?;
    let errors = result.error_count();
    crate::pipeline::print_diagnostics(&result, file, false);
    if errors > 0 {
        anyhow::bail!("{} type error(s) found", errors);
    }

    let workflows: Vec<_> = result
        .hir
        .functions
        .iter()
        .filter(|f| f.durability == Some(DurabilityKind::Workflow))
        .map(|f| f.name.as_str())
        .collect();
    let activities: Vec<_> = result
        .hir
        .functions
        .iter()
        .filter(|f| f.durability == Some(DurabilityKind::Activity))
        .map(|f| f.name.as_str())
        .collect();

    if workflows.is_empty() && activities.is_empty() {
        println!("No workflows or activities found in {}", file.display());
    } else {
        for name in &workflows {
            println!("workflow  {name}");
        }
        for name in &activities {
            println!("activity  {name}");
        }
    }
    Ok(())
}

/// Show type-checked info about a specific workflow.
pub async fn inspect(file: &Path, workflow_name: &str) -> Result<()> {
    use vox_compiler::hir::nodes::DurabilityKind;

    let result = crate::pipeline::run_frontend(file, false).await?;
    let errors = result.error_count();
    crate::pipeline::print_diagnostics(&result, file, false);
    if errors > 0 {
        anyhow::bail!("{} type error(s) found", errors);
    }
    let wf = result
        .hir
        .functions
        .iter()
        .find(|f| f.durability == Some(DurabilityKind::Workflow) && f.name == workflow_name)
        .ok_or_else(|| anyhow::anyhow!("workflow `{workflow_name}` not found in {}", file.display()))?;
    println!("workflow  {}", wf.name);
    println!("params    {}", wf.params.len());
    if let Some(ref rt) = wf.return_type {
        println!("returns   {:?}", rt);
    }
    Ok(())
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
        println!("✓ {} — {} warning(s)", file.display(), warnings);
        Ok(())
    } else {
        anyhow::bail!("{} type error(s) found", errors)
    }
}

/// Execute a workflow by name from a .vox source file.
pub async fn run_workflow(
    file: &Path,
    workflow_name: &str,
    _args_json: &str,
    _requested_run_id: Option<&str>,
    _mesh: bool,
) -> Result<()> {
    // Full durable execution requires the workflow-runtime feature and a
    // running Vox scheduler. For now, validate that the workflow exists and
    // bail with a clear message rather than silently returning success.
    let result = crate::pipeline::run_frontend(file, false).await?;
    let errors = result.error_count();
    if errors > 0 {
        anyhow::bail!("{} type error(s) found", errors);
    }
    use vox_compiler::hir::nodes::DurabilityKind;
    let found = result
        .hir
        .functions
        .iter()
        .any(|f| f.durability == Some(DurabilityKind::Workflow) && f.name == workflow_name);
    if !found {
        anyhow::bail!(
            "workflow `{workflow_name}` not found in {}",
            file.display()
        );
    }
    anyhow::bail!(
        "workflow execution requires the `workflow-runtime` feature and a running Vox scheduler; \
         found workflow `{workflow_name}` — compile with `--features workflow-runtime` to enable execution"
    );
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
