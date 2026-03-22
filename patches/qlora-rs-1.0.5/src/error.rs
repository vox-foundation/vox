//! Error types for qlora-rs.

use thiserror::Error;

/// Result type alias for qlora-rs operations.
pub type Result<T> = std::result::Result<T, QLoraError>;

/// Errors that can occur in qlora-rs operations.
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum QLoraError {
    /// Invalid quantization configuration.
    #[error("invalid quantization config: {0}")]
    InvalidConfig(String),

    /// Quantization error.
    #[error("quantization error: {0}")]
    Quantization(String),

    /// Shape mismatch.
    #[error("shape mismatch: expected {expected:?}, got {actual:?}")]
    ShapeMismatch {
        /// Expected shape
        expected: Vec<usize>,
        /// Actual shape
        actual: Vec<usize>,
    },

    /// GGUF export error.
    #[error("GGUF export error: {0}")]
    GgufExport(String),

    /// Native format export error.
    #[error("native format export error: {0}")]
    NativeExport(String),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// PEFT error.
    #[error("PEFT error: {0}")]
    Peft(#[from] peft_rs::PeftError),

    /// Candle error.
    #[error("candle error: {0}")]
    Candle(#[from] candle_core::Error),
}
