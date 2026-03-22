//! Errors from repository discovery.

/// I/O or path resolution failures while discovering a repository.
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    /// Underlying I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}
