//! Populi tensor surface: LoRA transformer, device helpers, training loop, manifests.

#![allow(clippy::module_inception)]

pub mod data;
pub mod device;
pub mod hf_keymap;
pub mod hf_load;
pub mod manifest;
pub mod model_card;
pub mod telemetry;
pub mod telemetry_schema;

#[cfg(feature = "candle-qlora")]
pub mod candle_inference_serve;
#[cfg(feature = "candle-qlora")]
pub mod candle_model_qwen;
pub mod mesh_train;
pub mod train_log;
pub mod training_text;

pub mod lora;


#[cfg(feature = "gpu")]
pub mod burn_stack;
#[cfg(feature = "gpu")]
pub mod optim;
#[cfg(feature = "gpu")]
pub mod tensor;
#[cfg(feature = "gpu")]
pub mod train;

#[cfg(feature = "gpu")]
pub extern crate burn;

#[cfg(feature = "gpu")]
pub use device::make_wgpu_device;
pub use device::{
    DeviceKind, GpuInfo, TrainProfile, apply_backend_env, detect_gpu_vendor,
    estimate_training_vram_mb, estimate_training_vram_mb_qlora, normalize_device, oom_guidance,
    print_gpu_summary, print_gpu_summary_for, probe_gpu, recommend_config,
    recommend_config_for_profile, sample_vram_used_mb,
};

#[cfg(feature = "gpu")]
pub use lora::{LoraAttentionKvCache, LoraLinear, LoraTransformerKvCache, LoraVoxTransformer};
pub use lora::{LoraConfig, lora_memory_estimate};


#[cfg(feature = "gpu")]
pub use burn_stack::{IGNORE_INDEX, Sequential, VoxTransformer, cross_entropy_loss};
#[cfg(feature = "gpu")]
pub use tensor::{ElementType, Tensor, TensorShape};

#[cfg(feature = "train")]
pub mod adapter_schema_v3;
#[cfg(feature = "train")]
pub mod artifact_bridge;
#[cfg(feature = "train")]
pub mod backend;


#[cfg(feature = "train")]
mod backend_candle_qlora;
#[cfg(feature = "train")]
pub mod checkpoint_state;
// QLoRA stack needs `LoraTrainingConfig` / preflight; `train` implies `candle-qlora` in this crate.
#[cfg(feature = "train")]
mod candle_qlora_graph;
#[cfg(feature = "train")]
pub mod candle_qlora_merge;
#[cfg(feature = "train")]
mod candle_qlora_train;
#[cfg(feature = "train")]
mod candle_qlora_weights;
#[cfg(feature = "train")]
pub mod execution_planner;
#[cfg(feature = "train")]
pub mod finetune_contract;
#[cfg(feature = "train")]
pub mod finetune_registry;
#[cfg(feature = "train")]
pub mod lora_train;
#[cfg(feature = "train")]
pub mod operator_messages;
#[cfg(feature = "train")]
pub mod preflight_train;
#[cfg(feature = "train")]
pub mod preset_schema;
#[cfg(feature = "train")]
mod qlora_preflight;
#[cfg(feature = "train")]
pub mod train_backend;
#[cfg(feature = "train")]
pub mod train_jsonl_preflight;
#[cfg(feature = "train")]
pub mod training_config;

#[cfg(feature = "train")]
pub use execution_planner::{ExecutionPlan, ExecutionPlanner};
#[cfg(feature = "train")]
pub use finetune_contract::FineTuneContract;
#[cfg(feature = "train")]
pub use preflight_train::preflight_for_contract;
#[cfg(feature = "train")]
pub use preset_schema::{
    CliOverrides, DEFAULT_PRESET, DatasetProfile, DeviceProfile, KNOWN_PRESETS, TrainPresetProfile,
    TrainPresetRegistry, load_registry, resolve_effective_profile,
};
#[cfg(feature = "train")]
pub use train_backend::{ExecutionKernel, PopuliTrainBackend};
#[cfg(feature = "train")]
pub use training_config::{LoraTrainingConfig, PopuliTokenizerMode, TrainingDeploymentTarget};
#[cfg(feature = "train")]
pub use lora_train::run_populi_training;
