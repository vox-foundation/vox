use crate::task::TaskKind;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshDirectoryEntry {
    pub scope_id: String,
    pub control_url: String,
    pub region_label: Option<String>,
    pub task_kinds: Vec<TaskKind>,
    pub public: bool,
    pub current_queue_depth: Option<usize>,
    pub supported_priorities: Option<Vec<u8>>,
    /// Optional Ed25519 signature of the canonical entry representation.
    pub signature: Option<Vec<u8>>,
    /// Ed25519 public key used to verify the signature.
    pub public_key: Option<[u8; 32]>,
}

impl MeshDirectoryEntry {
    /// Returns the canonical JSON representation for signing.
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut clone = self.clone();
        clone.signature = None;
        clone.public_key = None;
        serde_json::to_vec(&clone).unwrap_or_default()
    }
}
