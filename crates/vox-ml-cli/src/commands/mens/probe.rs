//! `vox mens probe` — GPU capability detection and recommended config.

use std::path::PathBuf;

use anyhow::Result;

/// `vox mens probe --measure` — report this host's real accelerator memory
/// budget, the same measurement `memory_model::plan_for`/`sweep` consume.
/// This is the honest, always-available half of "measure it" (the driver's
/// own reported working-set / max-alloc bytes); it is NOT a substitute for a
/// real training-run calibration point (see `crates/vox-populi/src/mens/tensor/calibration.rs` —
/// `PeakSampler` already records those automatically during real training).
pub fn run_measure() -> Result<()> {
    #[cfg(not(feature = "mens-base"))]
    {
        anyhow::bail!("`vox mens probe --measure` requires --features mens-base")
    }
    #[cfg(feature = "mens-base")]
    {
        use owo_colors::OwoColorize;
        use vox_populi::mens::tensor::accel_budget::query_accel_budget;

        match query_accel_budget() {
            Some(b) => {
                println!("{}", "--- Accelerator Memory Budget ---".bold().cyan());
                println!("Device:            {}", b.device_name.green());
                println!("Working set:       {} bytes", b.working_set_bytes);
                println!("Max single alloc:  {} bytes", b.max_alloc_bytes);
                println!("Host key:          {}", b.host_key());
                println!();
                println!(
                    "This is the driver-reported budget, not a fitted activation \
                     coefficient. A real calibration point comes from an actual \
                     training run — `PeakSampler` records one automatically; see \
                     `crates/vox-populi/src/mens/tensor/calibration.rs` for how to \
                     fold a new `CalibrationRecord` into the memory model."
                );
                Ok(())
            }
            None => {
                anyhow::bail!(
                    "no accelerator budget available on this host (query_accel_budget() returned None)"
                )
            }
        }
    }
}

/// `vox mens probe --sweep --model-dir <dir>` — the largest batch size (at a
/// fixed `--seq-len`) that fits this host's real memory budget for the model
/// at `model_dir`, using the exact same `sweep`/`plan_for` entry point
/// `vox mens train`'s auto-sizing path consumes. Refuses (naming why) for an
/// uncalibrated lane rather than guessing — e.g. `candle-metal` has no fitted
/// row yet (see `memory_model`'s module doc comment).
pub fn run_sweep(
    model_dir: Option<PathBuf>,
    seq_len: u64,
    gradient_checkpointing: bool,
) -> Result<()> {
    #[cfg(not(feature = "mens-base"))]
    {
        let _ = (model_dir, seq_len, gradient_checkpointing);
        anyhow::bail!("`vox mens probe --sweep` requires --features mens-base")
    }
    #[cfg(feature = "mens-base")]
    {
        use owo_colors::OwoColorize;
        use vox_populi::mens::tensor::accel_budget::{BudgetSource, query_accel_budget};
        use vox_populi::mens::tensor::memory_model::{
            CalKey, DeviceBudget, Lane, MemoryModels, ModelShape, sweep,
        };

        let model_dir = model_dir.ok_or_else(|| {
            anyhow::anyhow!("--sweep requires --model-dir <local model directory>")
        })?;
        let accel = query_accel_budget()
            .ok_or_else(|| anyhow::anyhow!("no accelerator budget available on this host"))?;
        let lane = match accel.source {
            BudgetSource::Metal => Lane::CandleMetal,
            BudgetSource::Cuda => Lane::CandleCuda,
        };
        let key = CalKey::new(lane, gradient_checkpointing)?;
        let shape = ModelShape::from_model_dir(&model_dir)?;
        let models = MemoryModels::load_default()?;
        let budget = DeviceBudget {
            working_set_bytes: accel.working_set_bytes,
            operator_fraction: None,
        };

        let picked = sweep(&budget, &models, &key, &shape, seq_len)?;
        println!("{}", "--- Sweep result ---".bold().cyan());
        println!("Device:      {}", accel.device_name.green());
        println!("Model dir:   {}", model_dir.display());
        println!(
            "Picked:      batch_size={} seq_len={} ({} tokens/step)",
            picked.batch_size,
            picked.seq_len,
            picked.batch_size * picked.seq_len
        );
        println!(
            "  {} mens train --batch-size {} --seq-len {}",
            "vox".cyan(),
            picked.batch_size,
            picked.seq_len,
        );
        Ok(())
    }
}

pub async fn run_probe(verbose: bool) -> Result<()> {
    #[cfg(not(feature = "gpu"))]
    {
        let _ = verbose;
        anyhow::bail!("`vox mens probe` requires --features gpu");
    }
    #[cfg(feature = "gpu")]
    {
        use owo_colors::OwoColorize;
        use vox_populi::mens::hardware;
        use vox_populi::mens::tensor::device::recommend_config;

        // New SSOT: get summary and real-time telemetry
        let summary = hardware::HardwareRegistry::probe().await;
        let telemetry = hardware::HardwareRegistry::monitor();

        println!("{}", "--- GPU Discovery (SSOT) ---".bold().cyan());
        println!("Model:      {}", summary.model_name.green());
        println!("Vendor:     {:?}", summary.vendor);
        println!("VRAM:       {} MB", summary.vram_mb.yellow());
        println!("GPU Count:  {}", summary.gpu_count.yellow());
        println!("Backend:    {:?}", summary.backend);

        if let Some(t) = telemetry {
            println!("{}", "--- Real-time Telemetry ---".bold().blue());
            println!("Util:       {}%", t.utilization_pct);
            println!("Temp:       {}°C", t.temperature_c);
            println!("Power:      {}W", t.power_usage_w);
            println!("Used VRAM:  {} MB", t.memory_used_mb);

            // Record to DB if telemetry is enabled
            let repository_id =
                vox_repository::discover_repository_or_fallback(std::path::Path::new("."))
                    .repository_id;
            let node_id = vox_secrets::resolve_secret(vox_secrets::SecretId::VoxMeshNodeId)
                .expose()
                .map(|s| s.trim().to_string());
            let tel_json = serde_json::to_value(&t).unwrap();

            vox_db::populi_registry_telemetry::record_hardware_telemetry_opt(
                &repository_id,
                node_id.as_deref(),
                &tel_json,
            )
            .await;
        }

        if verbose {
            let profile = recommend_config(summary.vram_mb);
            println!();
            println!(
                "{}",
                format!(
                    "Recommended config for this hardware ({} profile):",
                    profile.label
                )
                .bold()
                .magenta()
            );
            println!(
                "  --rank {} --batch-size {} --seq-len {}",
                profile.suggested_rank, profile.suggested_batch, profile.max_seq_len
            );
            println!();
            println!("  Example training command:");
            println!(
                "    {} mens train --device {} --rank {} --batch-size {} --seq-len {}",
                "vox".cyan(),
                summary.backend.as_cli_flag(),
                profile.suggested_rank,
                profile.suggested_batch,
                profile.max_seq_len,
            );
        }

        Ok(())
    } // end #[cfg(feature = "gpu")]
}

#[cfg(all(test, feature = "mens-base"))]
mod tests {
    use super::*;

    /// Either `Ok` (a device was found) or a named `Err` — never a panic. A
    /// GPU-less host is a valid answer, not a test skip (same convention as
    /// `accel_budget::query_accel_budget`'s own same-file test).
    #[test]
    fn run_measure_does_not_panic_on_any_host() {
        let _ = run_measure();
    }

    #[test]
    fn run_sweep_without_model_dir_names_the_missing_flag() {
        let err = run_sweep(None, 512, false).expect_err("model_dir is required");
        assert!(
            err.to_string().contains("--model-dir"),
            "error must name the missing flag: {err}"
        );
    }
}
