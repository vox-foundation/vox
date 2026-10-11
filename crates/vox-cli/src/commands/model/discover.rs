use clap::Parser;
use owo_colors::OwoColorize;
use vox_orchestrator::orchestrator::catalog_refresh::run_unified_catalog_refresh;

/// Refresh the model catalog from all sources.
#[derive(Parser)]
pub struct DiscoverArgs {
    /// Force refresh even if cache is warm.
    #[arg(long)]
    pub force: bool,
    /// After refresh, evaluate the discovered-but-unconfirmed (shadowed) models.
    /// Prints the backlog + the exact `vox model eval` command; runs the batch
    /// eval only when an OpenRouter key is configured (otherwise prints + exits).
    #[arg(long)]
    pub eval_shadowed: bool,
    /// After refresh, bench what changed: new `-latest` alias targets first, then new models
    /// (`models::adaptation`). Results feed routing, so a better newcomer is adopted.
    #[arg(long)]
    pub bench_new: bool,
    /// Most models to bench with `--bench-new`.
    #[arg(long, default_value_t = 5)]
    pub bench_limit: usize,
    /// Spend cap in USD for `--bench-new`.
    #[arg(long, default_value_t = 1.0)]
    pub max_usd: f64,
}

async fn bench_new_models(
    before: &[vox_orchestrator::models::ModelSpec],
    limit: usize,
    max_usd: f64,
) -> anyhow::Result<()> {
    use vox_orchestrator::models::ModelRegistry;
    use vox_orchestrator::models::adaptation::{bench_queue, catalog_delta};
    let mut registry = ModelRegistry::from_cache();
    if let Some(db) =
        vox_db::connect_workspace_journey_optional(vox_db::DbConnectSurface::CliWorkspace, true)
            .await
    {
        registry.inject_evidence_from_db(&db).await;
    }
    let delta = catalog_delta(before, &registry.list_models());
    let reachable = |m: &vox_orchestrator::models::ModelSpec| {
        vox_orchestrator::models::key_guard::inference_wire_for(&m.provider_type).is_some()
    };
    let queue = bench_queue(&delta, &registry, &reachable, limit);
    println!(
        "\n{} {} new model(s), {} new -latest target(s); benching {}",
        " ADAPT ".on_blue().white().bold(),
        delta.new_models.len(),
        delta.new_alias_targets.len(),
        queue.len()
    );
    if queue.is_empty() {
        return Ok(());
    }
    crate::commands::model::bench::run(crate::commands::model::bench::BenchArgs {
        models: queue,
        provisional: 0,
        latest: 0,
        suites: Vec::new(),
        repeats: 1,
        max_usd,
        concurrency: 4,
        dry_run: false,
        no_write: false,
        report: None,
        timeout_ms: 120_000,
    })
    .await
}

/// Render the shadow-eval backlog plan: the pending ids + the exact command to
/// evaluate them. Pure (no I/O) so it is unit-testable.
fn render_eval_plan(pending: &[String]) -> String {
    if pending.is_empty() {
        return "No shadowed models awaiting evaluation.".to_string();
    }
    let mut s = format!("{} shadowed model(s) awaiting evaluation:\n", pending.len());
    for id in pending {
        s.push_str(&format!("  - {id}\n"));
    }
    s.push_str(&format!(
        "\nEvaluate with: vox model eval --model {}",
        pending.join(" --model ")
    ));
    s
}

pub async fn run(args: DiscoverArgs) -> anyhow::Result<()> {
    println!(
        "{} Discovering models...",
        " INFO ".on_blue().white().bold()
    );

    let before = vox_orchestrator::models::ModelRegistry::from_cache().list_models();
    let report = run_unified_catalog_refresh(args.force).await?;

    println!(
        "  ✅ OpenRouter: {} models",
        report.openrouter_count.to_string().green()
    );
    println!(
        "  ✅ Ollama: {} models",
        report.ollama_count.to_string().green()
    );
    println!(
        "  ✅ Hugging Face: {} models",
        report.huggingface_count.to_string().green()
    );
    println!(
        "  ✅ Populi mesh: {} models",
        report.mesh_count.to_string().green()
    );
    println!(
        "  ✅ MENS local: {} models",
        report.mens_count.to_string().green()
    );
    if !report.new_discovery_ids.is_empty() {
        println!(
            "    {} {} new model id(s) emitted to telemetry",
            "↳".cyan(),
            report.new_discovery_ids.len().to_string().yellow().bold()
        );
    }

    println!(
        "\n✅ Total catalog models written: {} → {}",
        report.total_written.to_string().green().bold(),
        report.cache_path.display()
    );

    if args.eval_shadowed {
        println!("\n{}", render_eval_plan(&report.pending_eval_ids));
        if !report.pending_eval_ids.is_empty() {
            // Real inference requires the OpenRouter key (resolved through the
            // model-agnostic secrets facade). Without it we print the plan and
            // exit 0 — never fabricate eval results.
            if vox_secrets::resolve_secret(vox_secrets::SecretId::OpenRouterApiKey).is_present() {
                println!(
                    "\n{} Running batch eval over the backlog...",
                    " INFO ".on_blue().white().bold()
                );
                crate::commands::model::eval::run(crate::commands::model::eval::EvalArgs {
                    models: report.pending_eval_ids.clone(),
                    category: "general".to_string(),
                    no_write_back: false,
                    output: None,
                })
                .await?;
            } else {
                println!(
                    "\n{} OPENROUTER_API_KEY not configured — skipping live eval; run the command above when a key is set.",
                    " NOTE ".on_yellow().black().bold()
                );
            }
        }
    }

    if args.bench_new {
        bench_new_models(&before, args.bench_limit, args.max_usd).await?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::render_eval_plan;

    #[test]
    fn render_eval_plan_empty_backlog() {
        assert_eq!(
            render_eval_plan(&[]),
            "No shadowed models awaiting evaluation."
        );
    }

    #[test]
    fn render_eval_plan_lists_ids_and_command() {
        let plan = render_eval_plan(&["acme/a".to_string(), "acme/b".to_string()]);
        assert!(plan.contains("2 shadowed model(s)"));
        assert!(plan.contains("acme/a"));
        assert!(plan.contains("acme/b"));
        assert!(plan.contains("vox model eval --model acme/a --model acme/b"));
    }
}
