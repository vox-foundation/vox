//! vox-plugin-mens-candle-metal — Candle + Metal ML backend plugin.
//!
//! Implements `MlBackend` from vox-plugin-api. The model handle stores
//! a `Box<CandleModel>`; methods recover it via the opaque pointer field.
//!
//! # SP3 sub-batch C — QLoRA training fully ported from vox-populi
//!
//! `candle_qlora_train/` contains the full training loop extracted from
//! `vox-populi/src/mens/tensor/candle_qlora_train/`.  `training.rs` now wires
//! through to `candle_qlora_train::run_candle_qlora_train` via a `TrainRequest`
//! JSON envelope.
//!
//! See: docs/src/architecture/plugin-system-redesign-2026.md

mod backend;
pub mod candle_qlora_train;
mod checkpoint;
pub mod device;
pub mod inference;
mod model;
mod training;

// Moved to vox-plugin-mens-candle-core — byte-for-byte identical to the CUDA
// plugin's copies (config/manifest/train_log unified there with a small
// launch_argv/stderr-echo delta; the rest were exact duplicates). See that
// crate's lib.rs docs for the full rationale.
pub use vox_plugin_mens_candle_core::{
    adapter_schema_v3, checkpoint_state, config, external_serving_handoff, finetune_contract,
    hf_keymap, hf_layout, manifest, merge, model_card, operator_messages, qlora_preflight,
    qlora_weights, telemetry, telemetry_schema, train_jsonl_preflight, train_log, training_summary,
    training_text,
};

use abi_stable::{
    erased_types::TD_Opaque, export_root_module, prefix_type::PrefixTypeTrait, sabi_extern_fn,
    std_types::*,
};
use vox_plugin_api::VOX_PLUGIN_ABI_VERSION;
use vox_plugin_api::abi::{VoxPlugin_TO, VoxPluginRef, VoxPluginRoot, VoxPluginRootRef};
use vox_plugin_api::host::VoxHost_TO;

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
    RString::from(r#"{"id":"mens-candle-metal","version":"0.1.0"}"#)
}

#[sabi_extern_fn]
fn init(_host: VoxHost_TO<'static, RBox<()>>) -> RResult<VoxPluginRef, RBoxError> {
    let plugin = backend::CandleMetalPlugin::new();
    let to = VoxPlugin_TO::from_value(plugin, TD_Opaque);
    RResult::ROk(to)
}
