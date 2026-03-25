//! Mens — native Burn-based LoRA training and inference helpers.
//!
//! - **Preflight / prompts**: use [`vox_corpus::training`].
//! - **Tokenizer + JSONL**: re-exported from [`tensor::data`] (`vox-tensor`).
//!
//! Public surface is mostly CLI / training wiring; exhaustive per-field `///` is deferred (see
//! `docs/agents/doc-quality-verification.md`).

#![allow(missing_docs)]
// Burn/wgpu pulls deep generic graphs; default limit can overflow on `Send`/`Sync` inference.
#![recursion_limit = "256"]

pub mod tensor;

#[cfg(feature = "hf-hub")]
pub mod hub;

#[cfg(feature = "cloud")]
pub mod cloud;

/// Default HuggingFace model for Mens training and serving.
pub const DEFAULT_MODEL_ID: &str = "Qwen/Qwen2.5-Coder-3B-Instruct";

#[cfg(feature = "gpu")]
pub use burn;

pub use tensor::{
    DeviceKind, GpuInfo, apply_backend_env, detect_gpu_vendor, estimate_training_vram_mb,
    estimate_training_vram_mb_qlora, normalize_device, print_gpu_summary, print_gpu_summary_for,
    probe_gpu,
};

#[cfg(feature = "train")]
pub use tensor::artifact_bridge::MERGE_QLORA_REJECTS_BURN_BIN;
#[cfg(feature = "train")]
pub use tensor::operator_messages;
#[cfg(feature = "train")]
pub use tensor::{
    CliOverrides, DEFAULT_PRESET, DatasetProfile, DeviceProfile, KNOWN_PRESETS, TrainPresetProfile,
    TrainPresetRegistry, load_registry, resolve_effective_profile,
};
#[cfg(feature = "train")]
pub use tensor::{
    ExecutionKernel, FineTuneContract, LoraTrainingConfig, MensTokenizerMode, PopuliTrainBackend,
    TrainingDeploymentTarget, run_mens_training,
};
