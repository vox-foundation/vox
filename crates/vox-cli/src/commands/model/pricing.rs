use clap::{Parser, Subcommand};
use comfy_table::Table;
use comfy_table::modifiers::UTF8_ROUND_CORNERS;
use comfy_table::presets::UTF8_FULL;
use owo_colors::OwoColorize;

#[derive(Parser)]
pub struct PricingArgs {
    #[command(subcommand)]
    pub cmd: PricingCmd,
}

#[derive(Subcommand)]
pub enum PricingCmd {
    /// Show the model pricing catalog (observed vs catalog costs)
    Show {
        #[arg(long)]
        model: Option<String>,
    },
    /// Perform batch aggregation of telemetry into the pricing catalog
    Rollup,
}

pub async fn run(args: PricingArgs) -> anyhow::Result<()> {
    match args.cmd {
        PricingCmd::Show { model } => run_show(model).await,
        PricingCmd::Rollup => run_rollup().await,
    }
}

async fn run_show(model_filter: Option<String>) -> anyhow::Result<()> {
    let db = vox_db::VoxDb::open_default().await?;
    let pricing = db.get_pricing_catalog().await?;

    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS);
    table.set_header(vec![
        "Model",
        "Provider",
        "Obs $/1K",
        "Cat $/1K",
        "Confidence",
        "Samples",
    ]);

    for row in pricing {
        if let Some(ref m) = model_filter {
            if !row.model_id.contains(m) {
                continue;
            }
        }

        let obs_str = match row.observed_blended_per_1k {
            Some(v) => format!("{:.6}", v),
            None => "-".to_string(),
        };

        let cat_str = format!("{:.6}", row.catalog_input_per_1k); // assuming blended ~ input for simple display

        let conf_colored = match row.confidence.as_str() {
            "high" => "high".green().to_string(),
            "medium" => "medium".yellow().to_string(),
            _ => "low".dimmed().to_string(),
        };

        table.add_row(vec![
            row.model_id,
            row.provider,
            obs_str,
            cat_str,
            conf_colored,
            row.n_provider_reported.to_string(),
        ]);
    }

    if table.row_iter().count() == 0 {
        println!("No pricing data available. Try running `vox model pricing rollup` first.");
    } else {
        println!("{table}");
    }

    Ok(())
}

async fn run_rollup() -> anyhow::Result<()> {
    let db = vox_db::VoxDb::open_default().await?;
    println!("Rolling up observed telemetry into pricing catalog...");

    let changes = db.rollup_pricing_catalog().await?;
    println!("{} {} updated.", changes.to_string().green(), "rows");

    Ok(())
}
