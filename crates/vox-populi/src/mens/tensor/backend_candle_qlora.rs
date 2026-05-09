//! Candle **QLoRA** training dispatch via the `mens-candle-cuda` plugin.
//!
//! SP3-D: the training loop was extracted to `vox-plugin-mens-candle-cuda` in sub-batches A–C.
//! This module retains the [`TrainingBackend`] impl so that `lora_train::run_mens_training`
//! can dispatch `--backend qlora` without knowing about the plugin host directly.
//! At runtime the plugin must be installed; if not, a clear error is returned.

use std::path::Path;

use crate::mens::tensor::backend::TrainingBackend;
use crate::mens::tensor::device::DeviceKind;
use crate::mens::tensor::training_config::{LoraTrainingConfig, OptimizerExperimentMode};

/// Candle + HF tokenizer path (`--backend qlora`).
#[derive(Debug, Clone, Copy, Default)]
pub struct CandleQloraBackend;

/// JSON-wire summary returned by `run_full_training`.
#[derive(serde::Deserialize)]
struct TrainingSummaryWire {
    wall_secs: f64,
    total_steps: usize,
    total_tokens: usize,
    ms_per_step: f64,
}

impl TrainingBackend for CandleQloraBackend {
    fn run(
        &self,
        data_dir: &Path,
        output_dir: Option<&Path>,
        config: &LoraTrainingConfig,
        device_kind: DeviceKind,
        system_prompt: &str,
    ) -> anyhow::Result<crate::mens::tensor::backend::TrainingSummary> {
        tracing::debug!(
            backend = "candle_qlora",
            "Candle qlora backend run (plugin dispatch)"
        );
        if config.optimizer_experiment_mode != OptimizerExperimentMode::Off {
            let optimizer_resolved =
                vox_secrets::resolve_secret(vox_secrets::SecretId::VoxMensExperimentalOptimizer);
            let enabled = optimizer_resolved
                .expose()
                .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
            if !enabled {
                anyhow::bail!(
                    "optimizer experiment mode `{}` requires VOX_MENS_EXPERIMENTAL_OPTIMIZER=1",
                    match config.optimizer_experiment_mode {
                        OptimizerExperimentMode::Off => "off",
                        OptimizerExperimentMode::MuonClipLike => "muon_clip_like",
                    }
                );
            }
            tracing::warn!(
                mode = ?config.optimizer_experiment_mode,
                "experimental optimizer lane enabled"
            );
        }

        // ── Locate the plugin install directory ──────────────────────────────
        let plugins_dir = std::env::var("VOX_PLUGINS_DIR")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::data_local_dir()
                    .map(|p| p.join("vox").join("plugins"))
                    .unwrap_or_else(|| std::path::PathBuf::from("./vox-plugins"))
            });

        let registry = vox_plugin_host::discover(&plugins_dir)
            .map_err(|e| anyhow::anyhow!("plugin discovery failed at {plugins_dir:?}: {e}"))?;

        let loaded =
            vox_plugin_host::load_code_plugin(&registry, "mens-candle-cuda").map_err(|e| {
                anyhow::anyhow!(
                    "Could not load 'mens-candle-cuda' plugin from {plugins_dir:?}: {e}\n\
                 Install it with: vox plugin install mens-candle-cuda"
                )
            })?;

        let ml_backend = loaded.plugin.as_ml_backend().into_option().ok_or_else(|| {
            anyhow::anyhow!(
                "'mens-candle-cuda' plugin loaded but does not expose an MlBackend extension point"
            )
        })?;

        // ── Serialize the TrainRequest JSON envelope ─────────────────────────
        // The plugin expects { config, data_dir, output_dir, device_kind, system_prompt }.
        let request = serde_json::json!({
            "config": config,
            "data_dir": data_dir.to_str(),
            "output_dir": output_dir.and_then(|p| p.to_str()),
            "device_kind": device_kind_to_str(device_kind),
            "system_prompt": system_prompt,
        });
        let config_json = serde_json::to_string(&request)
            .map_err(|e| anyhow::anyhow!("failed to serialize TrainRequest: {e}"))?;

        // ── Dispatch ─────────────────────────────────────────────────────────
        let summary_json = ml_backend
            .run_full_training(config_json.as_str().into())
            .into_result()
            .map_err(|e| anyhow::anyhow!("mens-candle-cuda training failed: {e}"))?;

        // ── Parse summary ─────────────────────────────────────────────────────
        let wire: TrainingSummaryWire = serde_json::from_str(summary_json.as_str())
            .map_err(|e| anyhow::anyhow!("failed to parse training summary JSON: {e}"))?;

        Ok(crate::mens::tensor::backend::TrainingSummary {
            wall_secs: wire.wall_secs,
            total_steps: wire.total_steps,
            total_tokens: wire.total_tokens,
            ms_per_step: wire.ms_per_step,
        })
    }
}

fn device_kind_to_str(d: DeviceKind) -> &'static str {
    match d {
        DeviceKind::Cpu => "cpu",
        DeviceKind::Cuda => "cuda",
        DeviceKind::Metal => "metal",
        DeviceKind::Best => "best",
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::mens::tensor::backend::TrainingBackend;
    use crate::mens::tensor::device::DeviceKind;
    use crate::mens::tensor::training_config::{LoraTrainingConfig, OptimizerExperimentMode};

    use super::CandleQloraBackend;

    #[test]
    fn experimental_optimizer_requires_env_guard() {
        let cfg = LoraTrainingConfig {
            optimizer_experiment_mode: OptimizerExperimentMode::MuonClipLike,
            ..Default::default()
        };
        let err = CandleQloraBackend
            .run(Path::new("."), None, &cfg, DeviceKind::Cpu, "")
            .expect_err("guard should fail before plugin dispatch");
        assert!(err.to_string().contains("VOX_MENS_EXPERIMENTAL_OPTIMIZER"));
    }
}
