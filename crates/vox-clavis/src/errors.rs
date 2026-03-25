use thiserror::Error;

#[derive(Debug, Error, Clone)]
pub enum SecretError {
    #[error("backend unavailable: {0}")]
    BackendUnavailable(String),
    #[error("backend misconfigured: {0}")]
    BackendMisconfigured(String),
    #[error("backend query failed: {0}")]
    BackendQueryFailed(String),
    #[error("io failure: {0}")]
    Io(String),
    #[error("serialization failure: {0}")]
    Serialization(String),
}
