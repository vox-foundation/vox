use crate::task::TaskKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DonationSlot {
    pub task_kind: TaskKind,
    pub max_concurrent: u8,
    pub weight_pct: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkerDonationPolicy {
    pub slots: Vec<DonationSlot>,
    pub nsfw_allowed: bool,
    pub max_job_duration_secs: u64,
    pub public_mesh_opt_in: bool,
    /// Minimum priority required to accept a job from the public mesh.
    pub min_priority: u8,
    /// Optional whitelist of scopes this node is willing to donate to.
    /// If None, and public_mesh_opt_in is true, it accepts from any scope.
    pub allowed_scopes: Option<Vec<String>>,
    /// Optional whitelist of user IDs allowed to run tasks on this node.
    pub allowed_users: Option<Vec<String>>,
    /// Optional blacklist of user IDs explicitly denied from running tasks.
    pub denied_users: Option<Vec<String>>,
    /// Optional list of federated mesh networks (scope IDs) to explicitly allow.
    pub allowed_mesh_networks: Option<Vec<String>>,
    /// If `true`, this node is willing to accept workloads marked as handling
    /// sensitive data (e.g. PII, health records). Defaults to `false` for
    /// backwards compatibility with serialized policies that lack this field.
    #[serde(default)]
    pub accept_sensitive_workloads: bool,
}
