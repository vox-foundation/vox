/// Errors from the memory subsystem.
#[derive(Debug, thiserror::Error)]
pub enum MemoryError {
    /// Underlying filesystem error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON or other serialization failed.
    #[error("Serialization error: {0}")]
    Serialize(String),
}
