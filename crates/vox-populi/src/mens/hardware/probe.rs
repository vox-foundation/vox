use crate::mens::hardware::types::HardwareSummary;
use async_trait::async_trait;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProbeError {
    #[error("library unavailable: {0}")]
    LibraryUnavailable(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    #[error("device-side error: {0}")]
    DeviceError(String),
    #[error("other: {0}")]
    Other(String),
}

#[async_trait]
pub trait HardwareProbe: Send + Sync {
    fn name(&self) -> &'static str;
    fn applicable(&self) -> bool;
    async fn probe(&self) -> Result<Option<HardwareSummary>, ProbeError>;
}

#[derive(Debug, Clone)]
pub enum ProbeOutcome {
    NotApplicable,
    NoDevice,
    Found(Box<HardwareSummary>),
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct ProbeAttempt {
    pub probe_name: &'static str,
    pub outcome: ProbeOutcome,
    pub duration_ms: u64,
}

#[derive(Debug, Clone)]
pub struct ProbeReport {
    pub summary: HardwareSummary,
    pub attempts: Vec<ProbeAttempt>,
}
