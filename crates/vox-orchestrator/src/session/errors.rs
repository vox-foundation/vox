//! Session management errors.

/// Errors from session management.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Session '{0}' not found")]
    NotFound(String),
    #[error("Serialization error: {0}")]
    Serialize(serde_json::Error),
    #[error("Max sessions ({0}) reached")]
    MaxSessions(usize),
}
