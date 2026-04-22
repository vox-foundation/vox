use clap::Parser;
use owo_colors::OwoColorize;
use vox_db::{DbConfig, VoxDb};

/// Perform batch aggregation of telemetry into the scoreboard.
#[derive(Parser)]
pub struct RollupArgs {
    /// Time windows to compute (comma-separated, default: 7,30,90).
    #[arg(long, default_value = "7,30,90")]
    pub windows: String,
}

pub async fn run(args: RollupArgs) -> anyhow::Result<()> {
    let db_config = DbConfig::resolve_canonical().map_err(anyhow::Error::msg)?;
    let db = VoxDb::connect(db_config).await?;

    let windows: Vec<i64> = args
        .windows
        .split(',')
        .filter_map(|s| s.trim().parse::<i64>().ok())
        .collect();

    println!(
        "{} Performing model scoreboard rollup...",
        " INFO ".on_blue().white().bold()
    );

    for window in windows {
        print!("  Processing {} day window... ", window);
        match db.rollup_model_scoreboard(window).await {
            Ok(count) => println!("{} ({} rows updated)", "DONE".green().bold(), count),
            Err(e) => println!("{} ({})", "FAILED".red().bold(), e),
        }
    }

    Ok(())
}
