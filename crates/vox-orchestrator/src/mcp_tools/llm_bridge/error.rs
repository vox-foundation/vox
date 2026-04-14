//! HTTP inference errors for LLM providers.

/// Non-success or transport error from an LLM HTTP call.
#[derive(Debug)]
pub(crate) struct HttpInferError {
    /// HTTP status, or `0` for transport / configuration errors.
    pub status: u16,
    pub message: String,
}

impl std::fmt::Display for HttpInferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.status == 0 {
            return write!(f, "{}", self.message);
        }
        write!(f, "LLM API error {}: {}", self.status, self.message)
    }
}
