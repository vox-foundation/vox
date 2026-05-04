//! Mens tensor surface: LoRA transformer, device helpers, training loop, manifests.

#![allow(clippy::module_inception)]

pub mod data;
pub mod device;
pub mod hf_keymap;
pub mod hf_load;
pub mod manifest;
pub mod model_card;
pub mod telemetry;
pub mod telemetry_schema;

#[cfg(feature = "mens-candle-qlora")]
pub mod candle_inference_serve;
#[cfg(feature = "mens-candle-qlora")]
pub mod candle_model_qwen;
pub mod populi_train;
pub mod train_log;
#[cfg(feature = "mens-train")]
pub mod training_text;

pub mod lora;
pub mod vram_autodetect;

#[cfg(feature = "mens-gpu")]
pub mod burn_stack;
#[cfg(feature = "mens-gpu")]
pub mod optim;
#[cfg(feature = "mens-gpu")]
pub mod tensor;
#[cfg(feature = "mens-gpu")]
pub mod train;

#[cfg(feature = "mens-gpu")]
pub extern crate burn;

#[cfg(feature = "mens-gpu")]
pub use device::make_wgpu_device;
pub use device::{
    DeviceKind, GpuInfo, TrainProfile, apply_backend_env, detect_gpu_vendor,
    estimate_training_vram_mb, estimate_training_vram_mb_qlora, normalize_device, oom_guidance,
    print_gpu_summary, print_gpu_summary_for, probe_gpu, recommend_config,
    recommend_config_for_profile, sample_vram_used_mb,
};

#[cfg(feature = "mens-gpu")]
pub use lora::{LoraAttentionKvCache, LoraLinear, LoraTransformerKvCache, LoraVoxTransformer};
pub use lora::{LoraConfig, lora_memory_estimate};

#[cfg(feature = "mens-gpu")]
pub use burn_stack::{IGNORE_INDEX, Sequential, VoxTransformer, cross_entropy_loss};
#[cfg(feature = "mens-gpu")]
pub use tensor::{ElementType, Tensor, TensorShape};

#[cfg(feature = "mens-train")]
pub mod adapter_schema_v3;
#[cfg(feature = "mens-train")]
pub mod artifact_bridge;
#[cfg(feature = "mens-train")]
pub mod backend;

#[cfg(feature = "mens-train")]
mod backend_candle_qlora;
#[cfg(feature = "mens-train")]
pub mod checkpoint_state;
// SP3-D: candle_qlora_train, candle_qlora_weights, qlora_preflight, candle_qlora_graph extracted
// to vox-plugin-mens-candle-cuda. candle_qlora_merge and candle_inference_serve also extracted
// (Unit 1 follow-up) but kept here because adapter_schema_v3 + lora/part_block + lora/part_vox
// depend on the types. TODO: rewire those callers through the plugin host.
#[cfg(feature = "mens-train")]
pub mod candle_qlora_merge;
#[cfg(feature = "mens-train")]
pub mod domain_profiles;
pub mod domain_router;
#[cfg(feature = "mens-train")]
pub mod execution_planner;
#[cfg(feature = "mens-train")]
pub mod external_serving_handoff;
#[cfg(feature = "mens-train")]
pub mod finetune_contract;
#[cfg(feature = "mens-train")]
pub mod finetune_registry;
#[cfg(feature = "mens-train")]
pub mod lora_train;
#[cfg(feature = "mens-train")]
pub mod operator_messages;
#[cfg(feature = "mens-train")]
pub mod preflight_train;
#[cfg(any(feature = "mens-train", feature = "mens-cloud"))]
pub mod preset_schema;
#[cfg(feature = "mens-train")]
pub mod train_backend;
#[cfg(feature = "mens-train")]
pub mod train_jsonl_preflight;
#[cfg(feature = "mens-train")]
pub mod training_config;

// Private backend dispatch; anchor for unwired-module scans.
#[cfg(feature = "mens-train")]
#[allow(unused_imports)]
use self::backend_candle_qlora as _;

#[cfg(feature = "mens-train")]
pub use execution_planner::{ExecutionPlan, ExecutionPlanner};
#[cfg(feature = "mens-train")]
pub use finetune_contract::FineTuneContract;
#[cfg(feature = "mens-train")]
pub use lora_train::run_mens_training;
#[cfg(feature = "mens-train")]
pub use preflight_train::{
    TrainingPreflightRecord, preflight_for_contract, write_training_preflight_json,
};
#[cfg(feature = "mens-train")]
pub use preset_schema::{
    CliOverrides, DEFAULT_PRESET, DatasetProfile, DeviceProfile, KNOWN_PRESETS, TrainPresetProfile,
    TrainPresetRegistry, load_registry, resolve_effective_profile,
};
#[cfg(feature = "mens-train")]
pub use train_backend::{ExecutionKernel, PopuliTrainBackend};
#[cfg(feature = "mens-train")]
pub use training_config::{
    LoraTrainingConfig, MensTokenizerMode, OptimizerExperimentMode, TrainingDeploymentTarget,
};
