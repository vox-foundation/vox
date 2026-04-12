/// Errors emitted by the constrained generation engine.
#[derive(Debug, thiserror::Error)]
pub enum ConstrainedGenError {
    /// The sampler reached a state where no token is valid under the grammar.
    #[error("constrained-gen deadlock: no valid token at position {position}")]
    Deadlock {
        position: usize,
        partial_output: String,
    },

    /// A generation step exceeded the configured watchdog timeout.
    #[error(
        "constrained-gen timeout after {elapsed_ms}ms (limit {limit_ms}ms) at position {position}"
    )]
    Timeout {
        position: usize,
        elapsed_ms: u64,
        limit_ms: u64,
    },

    /// The grammar definition is malformed or empty.
    #[error("grammar error: {reason}")]
    GrammarError { reason: String },

    /// The revision sampler triggered a backtrack beyond the allowed depth.
    #[error("revision depth exceeded: {depth} > {max_depth}")]
    RevisionDepthExceeded { depth: usize, max_depth: usize },

    /// Catch-all for internal errors.
    #[error("internal: {0}")]
    Internal(String),
}

/// Alias for convenience.
pub type Result<T> = std::result::Result<T, ConstrainedGenError>;
