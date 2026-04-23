use clap::Parser;
use owo_colors::OwoColorize;
use std::collections::HashMap;
use vox_db::{DbConfig, VoxDb};
use vox_orchestrator::models::{ModelRegistry, ModelScore};
use vox_orchestrator::types::TaskCategory;

/// Explain model selection for a given task description.
#[derive(Parser)]
pub struct ExplainArgs {
    /// Task description or prompt.
    pub task: String,
    /// Explicit task category (optional).
    #[arg(long)]
    pub category: Option<String>,
    /// Estimated complexity (1-10).
    #[arg(long, default_value_t = 5)]
    pub complexity: u8,
}

pub async fn run(args: ExplainArgs) -> anyhow::Result<()> {
    // 1. Setup Registry
    let mut registry = ModelRegistry::new();

    // 2. Load Scoreboard from DB
    let db_config = DbConfig::resolve_canonical().map_err(anyhow::Error::msg)?;
    let db = VoxDb::connect(db_config).await?;
    let db_scores = db.get_model_scoreboard(7).await?;

    let mut scores = HashMap::new();
    for row in db_scores {
        scores.insert(row.model_id.clone(), ModelScore::from(row));
    }
    registry.inject_scoreboard(scores);

    // 3. Construct simulation parameters
    let category = if let Some(cat_str) = args.category {
        use std::str::FromStr;
        TaskCategory::from_str(&cat_str).unwrap_or(TaskCategory::General)
    } else {
        TaskCategory::General
    };

    let complexity = args.complexity;
    let description = args.task;

    // 4. Run Selection Explain
    println!(
        "{} Model Selection for task: \"{}\"",
        " EXPLAIN ".on_blue().white().bold(),
        description.italic()
    );
    println!("Category: {:?}, Complexity: {}", category, complexity);
    println!("---");

    let strength = vox_orchestrator::models::task_category_strength(category);
    let candidates = registry.explain_selection(
        category,
        strength,
        vox_orchestrator::config::CostPreference::Performance,
    );

    if candidates.is_empty() {
        println!("{}", "❌ No suitable models found in registry.".red());
        return Ok(());
    }

    println!(
        "{} Top Candidates (sorted by priority score):",
        " RANK ".on_green().black().bold()
    );
    for (i, entry) in candidates.iter().take(5).enumerate() {
        let prefix = if i == 0 {
            "🥇"
        } else if i == 1 {
            "🥈"
        } else if i == 2 {
            "🥉"
        } else {
            "  "
        };

        let mut details = Vec::new();
        details.push(format!("Tier: {:?}", entry.capabilities.tier));

        if let Some(score) = registry.get_score(&entry.id) {
            details.push(format!("Success: {:.1}%", score.success_rate * 100.0));
            details.push(format!("Quality: {:.2}", score.quality_score));
        }

        println!("{} {}: {}", prefix, entry.id.bold(), details.join(", "));
    }

    println!("\nSelection: {}", candidates[0].id.green().bold());

    // 5. Show most recent trace ID
    if let Ok(Some(tid)) = db
        .get_last_interaction_trace_id(&category.to_string())
        .await
    {
        println!("Recent Trace ID: {}", tid.dimmed());
    }

    Ok(())
}
