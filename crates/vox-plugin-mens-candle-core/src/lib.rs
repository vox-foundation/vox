//! vox-plugin-mens-candle-core — device-agnostic Candle QLoRA code shared by
//! `vox-plugin-mens-candle-metal` and `vox-plugin-mens-candle-cuda`.
//!
//! # Why this crate exists
//!
//! The two device plugins started as independent ports of `vox-populi`'s
//! Candle QLoRA trainer/server and grew 44 file pairs, 22 of them byte-for-byte
//! identical (up to 720 lines each). `model_card.rs` and `manifest.rs` were
//! *also* forked a third time into `vox-populi` itself, but that third copy
//! is NOT folded into this crate (see [`model_card`]'s docs: `vox-populi`
//! sits at a lower dependency layer than this crate and depending on it would
//! be a layer-rule violation that needs a user-authorized exception). Per
//! `AGENTS.md` §Dependency Discipline rule 3, a helper under ~50 lines may be
//! duplicated with a marker comment — "Never fork 100+ line chunks." This
//! crate is the fold for the two plugins: device-agnostic training/inference
//! code lives here once; genuinely device-specific code (CUDA memory-pool
//! FFI, Metal's `objc2-metal` peak sampler, the gradient-checkpointing
//! feature CUDA has and Metal doesn't yet) stays in each plugin crate. No
//! `cdylib` entry point — this is an `rlib`-only shared library, not a
//! loadable plugin on its own.
//!
//! # Layout
//!
//! - [`config`] — `LoraTrainingConfig` and friends (previously forked, save a
//!   `launch_argv` field CUDA had and Metal hadn't).
//! - [`manifest`] — on-disk training manifest (previously forked the same way).
//! - [`model_card`] — human-readable `MODEL_CARD.md` writer (previously forked
//!   between the two plugins).
//! - [`inference_device`] / [`device`] — the device-dispatch seam: `DeviceKind`,
//!   `resolve_inference_device`, `compute_dtype_for_device`. `resolve_inference_device`
//!   is what Metal was missing entirely (see that module's docs for the bug).
//! - Everything else (`hf_layout`, `hf_keymap`, `merge`, `qlora_preflight`,
//!   `qlora_weights`, `telemetry*`, `train_log`, `train_jsonl_preflight`,
//!   `training_text`, `training_summary`, `operator_messages`,
//!   `finetune_contract`, `adapter_schema_v3`, `checkpoint_state`,
//!   `external_serving_handoff`, `candle_qlora_train::*`) — byte-identical
//!   code moved verbatim.

pub mod adapter_schema_v3;
pub mod candle_qlora_train;
pub mod checkpoint_state;
pub mod config;
pub mod device;
pub mod external_serving_handoff;
pub mod finetune_contract;
pub mod hf_keymap;
pub mod hf_layout;
pub mod inference_device;
pub mod manifest;
pub mod merge;
pub mod model_card;
pub mod operator_messages;
pub mod qlora_preflight;
pub mod qlora_weights;
pub mod rope;
pub mod telemetry;
pub mod telemetry_schema;
pub mod train_jsonl_preflight;
pub mod train_log;
pub mod training_summary;
pub mod training_text;
