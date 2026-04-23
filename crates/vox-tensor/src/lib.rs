//! Native ML Tensor operations for Vox.
//!
//! Wraps the `burn` framework to provide PyTorch-like `Tensor` ergonomics
//! using native Rust cross-platform GPU capabilities (NdArray/WGPU) and autograd.
//!
//! Enable the `gpu` feature to compile the burn-backed tensor and nn modules.
//!
//! The `data` module (tokenizer + JSONL dataloader) is always available
//! so that callers can prepare training data on CPU without a GPU dependency.
//!
//! GPU-only modules re-export `burn` types with minimal wrapping; see `tensor` / `vox_nn` for details.

#![allow(clippy::collapsible_if)]
// Burn slice/shape APIs trigger these; boxing every large nn variant would churn the public surface.
#![allow(clippy::new_without_default)]
#![allow(clippy::useless_conversion)]
#![allow(clippy::single_range_in_vec_init)]
#![allow(clippy::large_enum_variant)]
// Burn-backed modules are mostly thin wrappers; workspace `missing_docs` is relaxed here only when `gpu` is enabled.
#![cfg_attr(feature = "gpu", allow(missing_docs))]

/// Pure-Rust tokenizer and JSONL DataLoader — always compiled, no GPU required.
pub mod data;

/// GRPO reward, advantage computation, and training configuration — always compiled, no GPU required.
pub mod grpo;

/// Experience replay buffer for catastrophic forgetting mitigation — always compiled, no GPU required.
pub mod replay;

/// LoRA adapter configuration — always compiled, no GPU required.
/// The SSOT for [`LoraConfig`] and [`lora_memory_estimate`] across the workspace.
pub mod lora_config;
pub use lora_config::{LoraConfig, lora_memory_estimate};

/// LoRA (Low-Rank Adaptation) — parameter-efficient fine-tuning.
/// Phase 1 of the burn-lora strategy. See `lora::LoraLinear` for usage.
#[cfg(feature = "gpu")]
pub mod lora;
#[cfg(feature = "gpu")]
pub mod optim;
/// Burn-backed dynamic tensor wrapper — must load before `vox_nn` (`vox_nn` depends on `Tensor`).
#[cfg(feature = "gpu")]
pub mod tensor;
#[cfg(feature = "gpu")]
pub mod train;
#[cfg(feature = "gpu")]
pub mod vox_nn;

#[cfg(feature = "gpu")]
pub extern crate burn;

// Burn dynamic `Module` stack: `crate::vox_nn` (renamed from `nn` so `vox_tensor::nn` cannot be
// confused with `candle_nn` / `burn::nn` when a dependent also links Candle).
#[cfg(feature = "gpu")]
pub use lora::LoraLinear;
#[cfg(feature = "gpu")]
pub use tensor::{ElementType, Tensor, TensorShape};
