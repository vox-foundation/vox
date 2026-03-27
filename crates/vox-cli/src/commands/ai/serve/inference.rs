//! Legacy Burn-backed eval-local inference was removed when native Mens inference moved to the
//! Candle QLoRA stack (`vox mens serve`, `vox-populi` `candle_inference_serve`).
//!
//! This module stays behind **`broken_inference_stub`** so `cargo check -p vox-cli --all-features`
//! remains buildable; entrypoints return a clear error instead of linking obsolete Burn loader APIs.

#![allow(dead_code)]

use anyhow::{Result, bail};
use std::path::Path;

#[cfg(feature = "gpu")]
pub struct LoadedPopuliEvalModel;

#[cfg(feature = "gpu")]
impl LoadedPopuliEvalModel {
    pub fn load(_model_path: &Path) -> Result<Self> {
        bail!(
            "eval-local Burn inference is not available; use `vox mens serve` / Candle QLoRA (`vox-populi` `candle_inference_serve`)"
        )
    }

    pub fn generate(
        &mut self,
        _system_prompt: &str,
        _user_prompt: &str,
        _max_tokens: usize,
        _temperature: f32,
    ) -> Result<String> {
        bail!("LoadedPopuliEvalModel::generate is unreachable when load() always fails")
    }
}

/// Generate one completion from a checkpoint path (GPU / stub build).
#[cfg(feature = "gpu")]
pub fn generate_one(
    model_path: &Path,
    system_prompt: &str,
    user_prompt: &str,
    max_tokens: usize,
    temperature: f32,
) -> Result<String> {
    let mut session = LoadedPopuliEvalModel::load(model_path)?;
    session.generate(system_prompt, user_prompt, max_tokens, temperature)
}
