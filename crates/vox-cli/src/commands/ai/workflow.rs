use crate::cli_actions::WorkflowAction;
use anyhow::{Context, Result};
use std::path::Path;

/// Dispatch workflow actions.
pub async fn run(action: WorkflowAction) -> Result<()> {
    match action {
        WorkflowAction::List { file } => list(&file).await,
        WorkflowAction::Inspect { file, name } => inspect(&file, &name).await,
        WorkflowAction::Check { file } => check(&file).await,
        WorkflowAction::Run { file, name, args } => {
            run_workflow(&file, &name, args.as_deref().unwrap_or("[]")).await
        }
    }
}

/// List all workflows and activities defined in a .vox file.
pub async fn list(file: &Path) -> Result<()> {
    let result = crate::pipeline::run_frontend(file, false).await?;
    let hir = &result.hir;

    if hir.activities.is_empty() && hir.workflows.is_empty() {
        println!("No workflows or activities found in {}", file.display());
        println!("  Add an 'activity' or 'workflow' block to your .vox file.");
        return Ok(());
    }

    if !hir.activities.is_empty() {
        println!("Activities ({}):", hir.activities.len());
        for act in &hir.activities {
            let params: Vec<String> = act
                .params
                .iter()
                .map(|p| format!("{}: {:?}", p.name, p.type_ann))
                .collect();
            let ret = act
                .return_type
                .as_ref()
                .map(|t| format!("{:?}", t))
                .unwrap_or_else(|| "Unit".to_string());
            println!("  activity {}({}) to {}", act.name, params.join(", "), ret);
        }
        println!();
    }

    if !hir.workflows.is_empty() {
        println!("Workflows ({}):", hir.workflows.len());
        for wf in &hir.workflows {
            let params: Vec<String> = wf
                .params
                .iter()
                .map(|p| format!("{}: {:?}", p.name, p.type_ann))
                .collect();
            let ret = wf
                .return_type
                .as_ref()
                .map(|t| format!("{:?}", t))
                .unwrap_or_else(|| "Unit".to_string());
            println!("  workflow {}({}) to {}", wf.name, params.join(", "), ret);
        }
    }

    Ok(())
}

/// Show type-checked info about a specific workflow.
pub async fn inspect(file: &Path, workflow_name: &str) -> Result<()> {
    let result = crate::pipeline::run_frontend(file, false).await?;
    let hir = &result.hir;

    let wf = hir
        .workflows
        .iter()
        .find(|w| w.name == workflow_name)
        .with_context(|| {
            format!(
                "Workflow '{}' not found in {}",
                workflow_name,
                file.display()
            )
        })?;

    let params: Vec<String> = wf
        .params
        .iter()
        .map(|p| format!("{}: {:?}", p.name, p.type_ann))
        .collect();
    let ret = wf
        .return_type
        .as_ref()
        .map(|t| format!("{:?}", t))
        .unwrap_or_else(|| "Unit".to_string());

    println!("Workflow: {}", wf.name);
    println!(
        "  Signature: workflow {}({}) to {}",
        wf.name,
        params.join(", "),
        ret
    );
    println!("  Activities in this file: {}", hir.activities.len());
    println!();

    if !hir.activities.is_empty() {
        println!("  Available activities:");
        for act in &hir.activities {
            let act_params: Vec<String> = act
                .params
                .iter()
                .map(|p| format!("{}: {:?}", p.name, p.type_ann))
                .collect();
            println!(
                "    {} ({}) \u{2014} callable with 'with {{ retries, timeout, activity_id }}'",
                act.name,
                act_params.join(", ")
            );
        }
        println!();
    }

    println!("  Tip: 'with' options supported:");
    println!("    retries: int         \u{2014} retry attempts on failure");
    println!("    timeout: str         \u{2014} e.g. \"30s\", \"5m\"");
    println!("    initial_backoff: str \u{2014} delay before first retry e.g. \"500ms\"");
    println!("    activity_id: str     \u{2014} unique ID for deduplication / idempotency");

    Ok(())
}

/// Type-check a workflow file through the full Vox compiler pipeline.
pub async fn check(file: &Path) -> Result<()> {
    let result: crate::pipeline::FrontendResult = crate::pipeline::run_frontend(file, false)
        .await
        .map_err(|e| anyhow::anyhow!("Workflow check failed: {}", e))?;

    let warnings = result.warning_count();
    let errors = result.error_count();

    crate::pipeline::print_diagnostics(&result, file, false);

    if errors == 0 {
        println!(
            "v {} \u{2014} {} activity(ies), {} workflow(s), {} warning(s)",
            file.display(),
            result.hir.activities.len(),
            result.hir.workflows.len(),
            warnings,
        );
        Ok(())
    } else {
        anyhow::bail!("{} type error(s) found", errors)
    }
}

/// Execute a workflow by building the project and running the generated binary in workflow mode.
///
/// When the project has workflows, the generated main checks `VOX_RUN_WORKFLOW` at startup.
/// If set, it runs the named workflow and exits instead of starting the HTTP server.
/// Use `--args '["a","b",42]'` to pass JSON-typed workflow arguments.
pub async fn run_workflow(file: &Path, workflow_name: &str, args_json: &str) -> Result<()> {
    let result = crate::pipeline::run_frontend(file, false).await?;
    let _wf = result
        .hir
        .workflows
        .iter()
        .find(|w| w.name == workflow_name)
        .with_context(|| {
            format!(
                "Workflow '{}' not found in {}",
                workflow_name,
                file.display()
            )
        })?;

    if let Err(e) = serde_json::from_str::<Vec<serde_json::Value>>(args_json) {
        anyhow::bail!(
            "Invalid --args JSON (must be array, e.g. [\"a\",42]): {}",
            e
        );
    }

    #[cfg(feature = "workflow-runtime")]
    {
        use crate::workflow_journal_codex;

        struct CliTracker {
            db: Option<vox_db::VoxDb>,
        }

        impl vox_workflow_runtime::WorkflowTracker for CliTracker {
            async fn is_activity_completed(&self, wf: &str, act_id: &str) -> anyhow::Result<bool> {
                if let Some(db) = &self.db {
                    return Ok(db.is_activity_completed(wf, act_id).await?);
                }
                Ok(false)
            }
            async fn on_workflow_started(&mut self, wf: &str, len: usize) -> anyhow::Result<()> {
                if let Some(db) = &self.db {
                    db.start_workflow_execution(wf, len as i64).await?;
                }
                Ok(())
            }
            async fn on_activity_started(
                &mut self,
                wf: &str,
                act: &str,
                act_id: &str,
            ) -> anyhow::Result<()> {
                if let Some(db) = &self.db {
                    let p = vox_pm::LogExecutionParams {
                        workflow_id: wf,
                        agent_id: None,
                        skill_id: None,
                        activity_name: act_id,
                        status: "running",
                        attempt: 1,
                        duration_ms: 0,
                        output_size: 0,
                        input: None,
                        output: None,
                        error: None,
                        options: Some(act),
                    };
                    db.log_execution(&p).await?;
                }
                Ok(())
            }
            async fn on_activity_completed(
                &mut self,
                wf: &str,
                act: &str,
                act_id: &str,
                res: &serde_json::Value,
            ) -> anyhow::Result<()> {
                if let Some(db) = &self.db {
                    let out_json = res.to_string();
                    let p = vox_pm::LogExecutionParams {
                        workflow_id: wf,
                        agent_id: None,
                        skill_id: None,
                        activity_name: act_id,
                        status: "ok",
                        attempt: 1,
                        duration_ms: 0,
                        output_size: out_json.len() as i64,
                        input: None,
                        output: Some(out_json.as_bytes()),
                        error: None,
                        options: Some(act),
                    };
                    db.log_execution(&p).await?;
                }
                Ok(())
            }
            async fn on_workflow_completed(&mut self, wf: &str) -> anyhow::Result<()> {
                if let Some(db) = &self.db {
                    db.finish_workflow_execution(wf, "completed", 0).await?;
                }
                Ok(())
            }
        }

        let db = vox_db::VoxDb::connect_default()
            .await
            .context("Failed to connect to VoxDB for workflow tracking")?;
        let mut tracker = CliTracker { db: Some(db) };

        let journal = vox_workflow_runtime::interpret_workflow_durable(
            &result.hir,
            workflow_name,
            &mut tracker,
        )
        .await?;
        for entry in &journal {
            workflow_journal_codex::persist_workflow_journal_entry_opt(workflow_name, entry).await;
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&journal).unwrap_or_else(|_| "[]".to_string())
        );
        println!(
            "Workflow '{}' completed (interpreted runtime).",
            workflow_name
        );
        Ok(())
    }

    #[cfg(not(feature = "workflow-runtime"))]
    {
        // Build the project (same as vox run)
        let out_dir = std::path::PathBuf::from("dist");
        crate::commands::build::run(file, &out_dir).await?;

        let generated_dir = std::path::PathBuf::from("target").join("generated");
        let shared_target = crate::fs_utils::run_target_dir_for_workspace(Some(&generated_dir));

        // Run workflow: set env so generated binary executes workflow and exits
        let extra_env: Vec<(String, String)> = vec![
            ("VOX_RUN_WORKFLOW".to_string(), workflow_name.to_string()),
            ("VOX_WORKFLOW_ARGS".to_string(), args_json.to_string()),
        ];

        let req = crate::build_service::CargoRequest::run(
            generated_dir,
            Some(shared_target),
            vec!["--".to_string()],
            extra_env,
        );
        let output = crate::build_service::run_cargo(&req)
            .context("Failed to execute workflow (cargo run in generated directory)")?;

        if !output.status.success() {
            std::io::Write::write_all(&mut std::io::stderr(), &output.stderr).ok();
            anyhow::bail!(
                "Workflow execution failed with exit code: {:?}",
                output.status.code()
            );
        }

        println!("Workflow '{}' completed successfully.", workflow_name);
        Ok(())
    }
}
