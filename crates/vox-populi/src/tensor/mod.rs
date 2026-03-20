//! Native ML Tensor operations for Vox (merged from vox-tensor).
//!
//! Wraps the `burn` framework to provide PyTorch-like `Tensor` ergonomics
//! using native Rust cross-platform GPU capabilities (NdArray/WGPU) and autograd.
#![allow(clippy::module_inception)]

#[cfg(feature = "bpe")]
pub mod bpe;
/// Pure-Rust tokenizer and JSONL DataLoader — always compiled, no GPU required.
pub mod data;
/// GPU capability detection and device/backend selection.
pub mod device;
#[cfg(feature = "hf_load")]
pub mod hf_load;
/// Training manifest schema and loading.
pub mod manifest;
/// Human-readable run report (MODEL_CARD.md).
pub mod model_card;
/// Training telemetry writer.
pub mod telemetry;
/// Debug logging with flush for training diagnostics.
pub mod train_log;

/// LoRA (Low-Rank Adaptation) — parameter-efficient fine-tuning.
pub mod lora;
/// Neural network primitives (layers, sequential, loss functions).
#[cfg(feature = "gpu")]
pub mod nn;
/// Optimizers and learning rate schedulers.
#[cfg(feature = "gpu")]
pub mod optim;
/// Tensor abstraction and backend definitions.
#[cfg(feature = "gpu")]
pub mod tensor;
/// Training loops, callbacks, and metrics.
#[cfg(feature = "gpu")]
pub mod train;

#[cfg(feature = "gpu")]
pub extern crate burn;

#[cfg(feature = "gpu")]
pub use device::make_wgpu_device;
pub use device::{
    DeviceKind, GpuInfo, TrainProfile, apply_backend_env, detect_gpu_vendor,
    estimate_training_vram_mb, normalize_device, oom_guidance, print_gpu_summary, probe_gpu,
    recommend_config, recommend_config_for_profile, sample_vram_used_mb,
};
#[cfg(feature = "gpu")]
pub use lora::LoraLinear;
#[cfg(feature = "gpu")]
pub use lora::{LoraAttentionKvCache, LoraTransformerKvCache};
pub use lora::{LoraConfig, lora_memory_estimate};
#[cfg(feature = "gpu")]
pub use nn::{
    IGNORE_INDEX, Module, Sequential, cross_entropy_loss, cross_entropy_loss_masked,
    cross_entropy_loss_unmasked,
};
#[cfg(feature = "gpu")]
pub use tensor::{ElementType, Tensor, TensorShape};

#[cfg(feature = "train")]
pub mod lora_train;
#[cfg(feature = "train")]
pub mod preset_schema;
#[cfg(feature = "train")]
pub mod training_preflight;
#[cfg(feature = "train")]
pub use lora_train::{LoraTrainingConfig, run_lora_training};
#[cfg(feature = "train")]
pub use preset_schema::{
    CliOverrides, DEFAULT_PRESET, DatasetProfile, DeviceProfile, KNOWN_PRESETS, TrainPresetProfile,
    TrainPresetRegistry, load_registry, resolve_effective_profile,
};
#[cfg(feature = "train")]
pub use training_preflight::{
    CONTRACT_PATH, FALLBACK_TRAIN_FILE, PRIMARY_TRAIN_FILE, ResolveSource, ResolvedTrainInput,
    find_workspace_root, load_contract, resolve_train_input, validate_train_preflight,
};
