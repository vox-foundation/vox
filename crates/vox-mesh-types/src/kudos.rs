use serde::{Deserialize, Serialize};

/// Primitives for the contribution reward system.
/// Collapses compute donation and code contribution into one system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RewardPrimitive {
    /// 1ms of GPU compute (adjusted by VRAM weight).
    GpuComputeMs,
    /// 1ms of CPU compute.
    CpuComputeMs,
    /// One successful result attestation.
    ResultAttestation,
    /// One peer-reviewed code contribution.
    CodeContribution,
    /// One peer-reviewed bug fix.
    BugFix,
    /// One peer-reviewed documentation improvement.
    DocsContribution,
}

impl RewardPrimitive {
    /// Return the human-readable slug for database storage.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::GpuComputeMs => "gpu_compute_ms",
            Self::CpuComputeMs => "cpu_compute_ms",
            Self::ResultAttestation => "result_attestation",
            Self::CodeContribution => "code_contribution",
            Self::BugFix => "bug_fix",
            Self::DocsContribution => "docs_contribution",
        }
    }
}

/// Request to credit a user for a contribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreditJobRequest {
    pub vox_user_id: String,
    pub node_id: String,
    pub primitive: RewardPrimitive,
    pub amount: u64,
    pub task_id: Option<String>,
    pub metadata_json: Option<String>,
}
