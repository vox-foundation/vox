use clap::Parser;
use comfy_table::Table;
use owo_colors::OwoColorize;
use vox_db::{DbConfig, VoxDb};

/// Show the model scoreboard.
#[derive(Parser)]
pub struct ScoreboardArgs {
    /// Time window in days (default: 7).
    #[arg(long, default_value_t = 7)]
    pub window: i64,
    /// Output format (default: table).
    #[arg(long, default_value = "table")]
    pub format: String,
}

pub async fn run(args: ScoreboardArgs) -> anyhow::Result<()> {
    let db_config = DbConfig::resolve_canonical().map_err(anyhow::Error::msg)?;
    let db = VoxDb::connect(db_config).await?;

    let rows = db.get_model_scoreboard(args.window).await?;

    if args.format == "json" {
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }

    let mut table = Table::new();
    table.set_header(vec![
        "Model ID",
        "Category",
        "Strength",
        "Calls",
        "Success %",
        "p50 ms",
        "p99 ms",
        "Cost/Succ",
        "Quality",
    ]);

    for row in rows {
        let success_rate = format!("{:.1}%", row.success_rate * 100.0);
        let success_color = if row.success_rate > 0.95 {
            success_rate.green().to_string()
        } else if row.success_rate > 0.8 {
            success_rate.yellow().to_string()
        } else {
            success_rate.red().to_string()
        };

        table.add_row(vec![
            row.model_id,
            row.task_category,
            row.strength_tag,
            row.n_calls.to_string(),
            success_color,
            row.p50_latency_ms
                .map(|v| v.to_string())
                .unwrap_or_default(),
            row.p99_latency_ms
                .map(|v| v.to_string())
                .unwrap_or_default(),
            row.cost_per_success_usd
                .map(|v| format!("${:.4}", v))
                .unwrap_or_default(),
            format!("{:.2}", row.quality_score),
        ]);
    }

    println!("{}", table);
    Ok(())
}
