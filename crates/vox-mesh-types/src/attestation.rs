use serde::{Deserialize, Serialize};

/// Signed result attestation produced by a worker for each completed task.
///
/// The worker signs a canonical JSON payload containing all fields except
/// `signature_b64` itself, using a per-job ephemeral Ed25519 key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attestation {
    /// Task identifier this attestation covers.
    pub task_id: String,
    /// BLAKE3 hex digest of the raw task input payload.
    pub input_hash_blake3_hex: String,
    /// BLAKE3 hex digest of the raw output payload.
    pub output_hash_blake3_hex: String,
    /// Measured GPU wall-clock time in fractional seconds.
    pub gpu_seconds: f64,
    /// BLAKE3 hex digest of the full execution trace, if available.
    pub trace_blake3_hex: Option<String>,
    /// Hex-encoded compressed Ed25519 public key of the ephemeral signing key.
    pub ephemeral_pubkey_hex: String,
    /// Base64-encoded Ed25519 signature over the canonical signed payload.
    pub signature_b64: String,
    /// Unix epoch milliseconds when this attestation was produced.
    pub signed_at_unix_ms: u64,
}
