//! vox-plugin-mens-candle-cuda — Candle + CUDA ML backend plugin.
//!
//! Implements `MlBackend` from vox-plugin-api. The model handle stores
//! a Box<CandleModel>; methods recover it via the opaque pointer field.
//!
//! # Extraction status (SP3)
//!
//! `model.rs` contains the full Qwen3.5 transformer block implementation copied
//! verbatim from `vox-populi/src/mens/tensor/candle_model_qwen.rs` (pure candle/qlora,
//! no vox-populi type dependencies).
//!
//! `training.rs` and `checkpoint.rs` are stubs. The actual training loop in vox-populi
//! (`candle_qlora_train/mod.rs`, `training_loop/`) is deeply tangled with:
//!   - `LoraTrainingConfig` and `QloraEmbedBundle` (vox-populi types)
//!   - `vox-tensor::data::TrainingPair` and `CheckpointState`
//!   - `vox_clavis` secret resolution, `VoxDB` async channel, `vox_corpus` data loader
//!
//! These stubs will be filled in during batch 3 (vox-populi rewire) when the JSON
//! batch wire format between caller and plugin is defined.
//!
//! See: docs/src/architecture/plugin-system-redesign-2026.md

mod backend;
mod checkpoint;
pub mod config;
mod model;
mod training;

use abi_stable::{
    erased_types::TD_Opaque, export_root_module, prefix_type::PrefixTypeTrait, sabi_extern_fn,
    std_types::*,
};
use vox_plugin_api::abi::{VoxPlugin_TO, VoxPluginRef, VoxPluginRoot, VoxPluginRootRef};
use vox_plugin_api::host::VoxHost_TO;
use vox_plugin_api::VOX_PLUGIN_ABI_VERSION;

#[export_root_module]
fn root_module() -> VoxPluginRootRef {
    VoxPluginRoot {
        abi_version: VOX_PLUGIN_ABI_VERSION,
        manifest_json,
        init,
    }
    .leak_into_prefix()
}

#[sabi_extern_fn]
fn manifest_json() -> RString {
    RString::from(r#"{"id":"mens-candle-cuda","version":"0.1.0"}"#)
}

#[sabi_extern_fn]
fn init(_host: VoxHost_TO<'static, RBox<()>>) -> RResult<VoxPluginRef, RBoxError> {
    let plugin = backend::CandleCudaPlugin::new();
    let to = VoxPlugin_TO::from_value(plugin, TD_Opaque);
    RResult::ROk(to)
}
