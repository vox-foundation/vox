//! Training hyperparameter presets: 4080, safe, A100-shaped profiles.

use crate::tensor::device::probe_gpu;

/// CLI numeric overrides for auto-tuning.
#[derive(Debug, Clone, Default)]
pub struct CliOverrides {
    pub rank: Option<usize>,
    pub alpha: Option<f32>,
    pub seq_len: Option<usize>,
    pub batch_size: Option<usize>,
    pub grad_accum: Option<usize>,
    pub epochs: Option<usize>,
    pub warmup: Option<usize>,
    pub lr: Option<f64>,
}

/// GPU-derived device profile.
#[derive(Debug, Clone)]
pub struct DeviceProfile {
    pub model_name: String,
    pub vram_mb: u64,
}

impl DeviceProfile {
    pub fn from_gpu_info(model_name: &str, vram_mb: u64) -> Self {
        Self {
            model_name: model_name.to_string(),
            vram_mb,
        }
    }
}

/// Effective training hyperparameters after preset + overrides + dataset scaling heuristics.
#[derive(Debug, Clone)]
pub struct TrainPresetProfile {
    pub rank: usize,
    pub alpha: f32,
    pub seq_len: usize,
    pub batch_size: usize,
    pub grad_accum: usize,
    pub epochs: usize,
    pub warmup: usize,
    pub lr: f64,
}

pub const DEFAULT_PRESET: &str = "4080";

pub const KNOWN_PRESETS: &[&str] = &[
    "tiny",
    "safe",
    "4080",
    "4080_safe",
    "qwen_4080_16g",
    "a100",
    "default",
    "distributed",
    "mobile_edge",
];

fn base_for_name(name: &str) -> TrainPresetProfile {
    match name {
        "tiny" => TrainPresetProfile {
            rank: 4,
            alpha: 8.0,
            seq_len: 128,
            batch_size: 1,
            grad_accum: 1,
            epochs: 1,
            warmup: 10,
            lr: 1e-4,
        },
        "safe" | "4080_safe" => TrainPresetProfile {
            rank: 8,
            alpha: 16.0,
            seq_len: 256,
            batch_size: 2,
            grad_accum: 8,
            epochs: 3,
            warmup: 50,
            lr: 2e-4,
        },
        // Conservative Qwen + Candle QLoRA on ~16GB (e.g. RTX 4080 Super).
        // `4080` is an alias of `qwen_4080_16g` so default preset matches 16G QLoRA, not generic LoRA.
        "4080" | "qwen_4080_16g" => TrainPresetProfile {
            rank: 16,
            alpha: 32.0,
            seq_len: 384,
            batch_size: 1,
            grad_accum: 8,
            epochs: 3,
            warmup: 80,
            lr: 1.5e-4,
        },
        "a100" => TrainPresetProfile {
            rank: 32,
            alpha: 64.0,
            seq_len: 1024,
            batch_size: 8,
            grad_accum: 2,
            epochs: 3,
            warmup: 200,
            lr: 2e-4,
        },
        "distributed" => TrainPresetProfile {
            rank: 16,
            alpha: 32.0,
            seq_len: 512,
            batch_size: 4,
            grad_accum: 8,
            epochs: 3,
            warmup: 150,
            lr: 1.5e-4,
        },
        "mobile_edge" => TrainPresetProfile {
            rank: 8,
            alpha: 16.0,
            seq_len: 256,
            batch_size: 1,
            grad_accum: 8,
            epochs: 3,
            warmup: 40,
            lr: 1.5e-4,
        },
        _ => TrainPresetProfile {
            rank: 16,
            alpha: 32.0,
            seq_len: 512,
            batch_size: 4,
            grad_accum: 4,
            epochs: 3,
            warmup: 100,
            lr: 2e-4,
        },
    }
}

/// Load optional YAML registry from `populi/config/train-presets.yaml` if present.
pub struct TrainPresetRegistry;

impl TrainPresetRegistry {
    pub fn load() -> Option<serde_yaml::Value> {
        let root = vox_corpus::training::contract::find_workspace_root()?;
        let p = root.join("populi/config/train-presets.yaml");
        let raw = std::fs::read_to_string(p).ok()?;
        serde_yaml::from_str(&raw).ok()
    }
}

pub fn load_registry() -> Option<serde_yaml::Value> {
    TrainPresetRegistry::load()
}

/// Resolve preset from `VOX_TRAIN_PROFILE` env, CLI `--preset`, device heuristics, and overrides.
pub fn resolve_effective_profile(
    preset: Option<&str>,
    device: DeviceProfile,
    sample_count: Option<usize>,
    overrides: CliOverrides,
) -> TrainPresetProfile {
    let env_p = std::env::var("VOX_TRAIN_PROFILE").ok();
    let name = preset.or(env_p.as_deref()).unwrap_or(DEFAULT_PRESET);

    let mut p = base_for_name(name);

    if device.vram_mb > 0
        && device.vram_mb < 12_000
        && !matches!(name, "tiny" | "safe" | "4080_safe" | "mobile_edge")
    {
        p = base_for_name("4080_safe");
    }

    if let Some(n) = sample_count
        && n < 500
    {
        p.epochs = p.epochs.clamp(2, 5);
        p.warmup = p.warmup.min(50);
    }

    if let Some(r) = overrides.rank {
        p.rank = r;
    }
    if let Some(a) = overrides.alpha {
        p.alpha = a;
    }
    if let Some(s) = overrides.seq_len {
        p.seq_len = s;
    }
    if let Some(b) = overrides.batch_size {
        p.batch_size = b;
    }
    if let Some(g) = overrides.grad_accum {
        p.grad_accum = g;
    }
    if let Some(e) = overrides.epochs {
        p.epochs = e;
    }
    if let Some(w) = overrides.warmup {
        p.warmup = w;
    }
    if let Some(l) = overrides.lr {
        p.lr = l;
    }

    let _ = probe_gpu();
    p
}

/// Back-compat alias used in older docs.
pub type DatasetProfile = TrainPresetProfile;

#[cfg(test)]
mod preset_tests {
    use super::*;

    #[test]
    fn preset_4080_matches_qwen_4080_16g() {
        let a = base_for_name("4080");
        let b = base_for_name("qwen_4080_16g");
        assert_eq!(a.seq_len, b.seq_len);
        assert_eq!(a.batch_size, b.batch_size);
        assert_eq!(a.grad_accum, b.grad_accum);
        assert_eq!(a.rank, b.rank);
        assert_eq!(a.lr, b.lr);
    }

    #[test]
    fn known_presets_include_4080_family() {
        assert!(KNOWN_PRESETS.contains(&"4080"));
        assert!(KNOWN_PRESETS.contains(&"qwen_4080_16g"));
    }

    #[test]
    fn mobile_edge_preset_is_single_batch() {
        let p = base_for_name("mobile_edge");
        assert_eq!(p.batch_size, 1);
        assert!(p.seq_len <= 512);
        assert!(p.rank <= 32);
    }
}
