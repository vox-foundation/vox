/// A validation error encountered when checking an orchestrator configuration.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigValidationError {
    /// `max_agents` was below one.
    #[error("max_agents must be >= 1 (got {0})")]
    InvalidMaxAgents(usize),
    /// File lock TTL was shorter than the minimum safe window.
    #[error("lock_timeout_ms must be >= 100 (got {0})")]
    InvalidLockTimeout(u64),
    /// Broadcast channel capacity was zero.
    #[error("bulletin_capacity must be >= 1 (got {0})")]
    InvalidBulletinCapacity(usize),
    /// Scaling bounds were inconsistent (`min_agents` > `max_agents`).
    #[error("min_agents ({0}) cannot be greater than max_agents ({1})")]
    InvalidScalingLimits(usize, usize),
    /// Planning toggles are inconsistent.
    #[error("invalid planning configuration: {0}")]
    PlanningInvalid(String),
}

/// Errors that can occur loading orchestrator configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Underlying filesystem error while reading or writing config files.
    #[error("I/O error reading config: {0}")]
    Io(#[from] std::io::Error),
    /// TOML syntax or schema mismatch on deserialize.
    #[error("TOML parse error: {0}")]
    Parse(#[from] toml::de::Error),
    /// TOML serialization failed (e.g., when persisting overrides).
    #[error("TOML serialize error: {0}")]
    Serialize(#[from] toml::ser::Error),
}
