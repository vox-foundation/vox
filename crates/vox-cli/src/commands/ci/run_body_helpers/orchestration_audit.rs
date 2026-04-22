//! Orchestration SSOT Audit: confirms parity between telemetry-based decisioning and the canonical routing architecture.

use anyhow::Result;
use std::path::Path;

#[cfg(feature = "dei")]
pub async fn run_ssot_audit(_root: &Path) -> Result<()> {
    use anyhow::anyhow;
    use vox_db::{DbConfig, VoxDb};
    use vox_orchestrator::config::CostPreference;
    use vox_orchestrator::models::ModelRegistry;
    use vox_orchestrator::types::{AgentTask, TaskCategory, TaskId, TaskPriority};

    println!("Orchestration SSOT Audit: verifying routing parity with telemetry...");

    // 1. Initialize Registry with default config
    let mut registry = ModelRegistry::new();

    // 2. Connect to VoxDb to get latest scoreboard
    let db_config = DbConfig::resolve_canonical().map_err(anyhow::Error::msg)?;
    let db = VoxDb::connect(db_config)
        .await
        .map_err(|e| anyhow!("Failed to open VoxDb: {}", e))?;

    let scores = match db.get_model_scoreboard(30).await {
        Ok(s) => s,
        Err(e) => {
            println!(
                "  [WARN] Failed to fetch model scoreboard: {}. Continuing with static config only.",
                e
            );
            vec![]
        }
    };

    let score_map = scores
        .into_iter()
        .map(|s| (s.model_id.clone(), s.into()))
        .collect();
    registry.inject_scoreboard(score_map);

    println!(
        "  Loaded scoreboard for {} models.",
        registry.scoreboard_len()
    );

    // 3. Perform Test Routings
    let test_categories = [
        TaskCategory::CodeGen,
        TaskCategory::Research,
        TaskCategory::Planning,
    ];

    let mut violations = 0;

    for cat in test_categories {
        let mut task = AgentTask::new(
            TaskId(0),
            format!("Audit task for {:?}", cat),
            TaskPriority::Normal,
            vec![],
        );
        task.task_category = cat;
        task.estimated_complexity = 5;

        // Performance Routing
        let perf_model = registry.best_for_task(&task, CostPreference::Performance);
        if let Some(m) = perf_model {
            println!("  [OK] Performance Routing ({}): selected {}", cat, m.id);
        } else {
            println!("  [FAIL] Performance Routing ({}): no model selected!", cat);
            violations += 1;
        }

        // Economy Routing (should stay within budget if task has one)
        let mut budget_task = task.clone();
        budget_task.budget = Some(vox_orchestrator::types::Budget {
            max_cost_usd: Some(0.05), // Tight budget
            max_latency_ms: None,
        });
        // Add some complexity to increase token count
        budget_task.estimated_complexity = 8;

        let econ_model = registry.best_for_task(&budget_task, CostPreference::Economy);
        if let Some(m) = econ_model {
            let est_tokens = budget_task.estimated_token_count();
            let cost_basis = registry
                .get_score(&m.id)
                .and_then(|s| s.cost_per_success_usd)
                .unwrap_or(m.cost_per_1k);
            let est_cost = (est_tokens as f64 / 1000.0) * cost_basis;

            if est_cost > 0.05 {
                println!(
                    "  [FAIL] Economy Routing ({}): selected {} at estimated cost ${:.4}, exceeding budget!",
                    cat, m.id, est_cost
                );
                violations += 1;
            } else {
                println!(
                    "  [OK] Economy Routing ({}): selected {} at estimated cost ${:.4}",
                    cat, m.id, est_cost
                );
            }
        } else {
            // It's possible no model fits a tight budget, but we should have free models as fallbacks
            println!(
                "  [WARN] Economy Routing ({}): no model selected for tight budget ($0.05). Ensure free models are available.",
                cat
            );
        }
    }

    if violations > 0 {
        return Err(anyhow!(
            "Orchestration SSOT Audit failed with {} violations.",
            violations
        ));
    }

    println!("Orchestration SSOT Audit PASSED.");
    Ok(())
}

#[cfg(not(feature = "dei"))]
pub async fn run_ssot_audit(_root: &Path) -> Result<()> {
    println!("Orchestration SSOT Audit skipped: requires --features dei.");
    Ok(())
}
