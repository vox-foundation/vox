use serde::{Deserialize, Serialize};

/// Envelope for synchronizing Clavis secrets across the mesh.
///
/// Encapsulates a sealed (encrypted) secret payload along with metadata
/// required for routing and verification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClavisSyncEnvelope {
    /// The canonical secret identifier (e.g. `GEMINI_API_KEY`).
    pub secret_id: String,
    /// The encrypted secret value, sealed with the recipient's public key.
    /// Typically produced by `vox_crypto::seal`.
    pub sealed_payload: Vec<u8>,
    /// The node ID of the sender.
    pub sender_node_id: String,
    /// Millisecond timestamp when the sync was initiated.
    pub timestamp_unix_ms: u64,
}
