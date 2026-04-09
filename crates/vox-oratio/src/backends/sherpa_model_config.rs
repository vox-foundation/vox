//! Model path resolution for the Sherpa-ONNX backend.
//!
//! Priority: `VOX_ORATIO_SHERPA_MODEL_DIR` env (local dir) → HF Hub download.

#![cfg(feature = "stt-sherpa")]

use std::path::PathBuf;
use anyhow::{Context, Result};

/// Resolved paths to Sherpa-ONNX model artifacts.
pub struct SherpaModelPaths {
    pub model_onnx: PathBuf,
    pub tokens_txt: PathBuf,
}

/// Default HF model ID for Sherpa download.
pub const DEFAULT_SHERPA_HF_MODEL: &str = "k2-fsa/sherpa-onnx-whisper-tiny.en";

/// Resolve model paths: env-set local dir OR HF Hub download.
pub fn resolve_sherpa_model_paths() -> Result<SherpaModelPaths> {
    if let Ok(dir) = std::env::var("VOX_ORATIO_SHERPA_MODEL_DIR") {
        let dir = PathBuf::from(dir.trim());
        // Basic resolution if local directory provided
        let model_onnx = dir.join("model.onnx");
        let tokens_txt = dir.join("tokens.txt");
        // Often sherpa model has tiny-encoder.onnx, tiny-decoder.onnx...
        // For simplicity:
        return Ok(SherpaModelPaths { model_onnx, tokens_txt });
    }

    // HF Hub download (hf-hub crate already in deps via stt-candle, re-use it)
    let model_id = std::env::var("VOX_ORATIO_SHERPA_HF_MODEL")
        .unwrap_or_else(|_| std::env::var("VOX_ORATIO_SHERPA_MODEL").unwrap_or_else(|_| DEFAULT_SHERPA_HF_MODEL.to_string()));
    let revision = "main";
    let api = hf_hub::api::sync::Api::new().context("HF API init")?;
    let repo = api.repo(hf_hub::Repo::with_revision(
        model_id.clone(), hf_hub::RepoType::Model, revision.to_string(),
    ));

    // Try whisper-tiny.en style mapping for Sherpa
    let model_onnx = repo.get("tiny.en-encoder.int8.onnx")
        .or_else(|_| repo.get("model.onnx"))
        .with_context(|| format!("fetch model.onnx/encoder from {model_id}"))?;
    let tokens_txt = repo.get("tiny.en-tokens.txt")
        .or_else(|_| repo.get("tokens.txt"))
        .with_context(|| format!("fetch tokens.txt from {model_id}"))?;
    Ok(SherpaModelPaths { model_onnx, tokens_txt })
}
