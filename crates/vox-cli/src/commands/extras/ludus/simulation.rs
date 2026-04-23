use anyhow::Result;
use std::path::PathBuf;
use vox_ludus::run::run_monte_carlo_battle_sweep;

pub async fn run_monte_carlo_sweep(iterations: u32, output_dir: PathBuf) -> Result<()> {
    let _report = run_monte_carlo_battle_sweep(iterations, output_dir)?;
    Ok(())
}
