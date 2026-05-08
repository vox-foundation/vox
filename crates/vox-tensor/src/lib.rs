//! Pure-CPU data loaders and replay buffers for Vox training pipelines.
//!
//! Used by:
//! - `vox-corpus` for corpus extraction
//! - `vox-ml-cli` for training data preparation
//! - `vox-plugin-mens-candle-cuda` for QLoRA training data ingestion

/// Pure-Rust tokenizer and JSONL DataLoader — always compiled, no GPU required.
pub mod data;

/// Experience replay buffer for catastrophic forgetting mitigation — always compiled, no GPU required.
pub mod replay;
