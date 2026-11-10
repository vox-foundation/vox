use clap::Parser;
use comfy_table::Table;
use vox_db::{DbConfig, VoxDb};

/// Show detailed cost reporting.
#[derive(Parser)]
pub struct CostsArgs {
    /// Time window in days (default: 7).
    #[arg(long, default_value_t = 7)]
    pub window: i64,
    /// Group by model id instead of displaying all details.
    #[arg(long)]
    pub by_model: bool,
    /// Group by provider.
    #[arg(long)]
    pub by_provider: bool,
    /// Output format (default: table).
    #[arg(long, default_value = "table")]
    pub format: String,
}

pub async fn run(args: CostsArgs) -> anyhow::Result<()> {
    let db_config = DbConfig::resolve_canonical().map_err(anyhow::Error::msg)?;
    let db = VoxDb::connect(db_config).await?;

    let rows = db.get_model_scoreboard(args.window).await?;

    if args.format == "json" {
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }

    println!("COST REPORT (Last {} days)\n", args.window);

    let mut table = Table::new();
    table.set_header(vec![
        "Model ID",
        "Category",
        "Calls",
        "Successes",
        "Total Cost USD",
        "Cost/Success USD",
    ]);

    for row in rows {
        table.add_row(vec![
            row.model_id,
            row.task_category,
            row.n_calls.to_string(),
            row.success_count.to_string(),
            format!("${:.5}", row.cumulative_cost_usd),
            row.cost_per_success_usd
                .map(|v| format!("${:.5}", v))
                .unwrap_or_else(|| "$0.00000".to_string()),
        ]);
    }

    println!("{}", table);
    Ok(())
}
