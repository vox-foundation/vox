//! Signed audit-log writer — used by every destructive mesh action (P4-T7).
//!
//! Signs the canonical JSON of (action, target, actor, ts_micros) with an
//! Ed25519 key held by the dashboard process. The signature is hex-encoded
//! (128 chars) so it is printable in JSON without padding ambiguity.
//!
//! The in-memory sink accumulates entries for the lifetime of the process;
//! the audit-log scrubber (P4-T5) can read them back via the oplog route.

use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use vox_crypto::{SigningKey, generate_signing_keypair, sign};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    pub audit_id: String,
    pub action: String,
    pub target: String,
    pub actor: String,
    pub reason: String,
    pub ts_micros: u64,
    /// Hex-encoded Ed25519 signature over the canonical JSON payload.
    pub signature: String,
}

pub struct AuditWriter {
    key: SigningKey,
    entries: Mutex<Vec<AuditEntry>>,
}

impl AuditWriter {
    pub fn new(key: SigningKey) -> Self {
        Self {
            key,
            entries: Mutex::new(vec![]),
        }
    }

    /// Generate a fresh ephemeral keypair — suitable for tests.
    pub fn ephemeral() -> Self {
        let (sk, _vk) = generate_signing_keypair();
        Self::new(sk)
    }

    pub async fn record(
        &self,
        action: &str,
        target: &str,
        actor: &str,
        reason: &str,
    ) -> AuditEntry {
        let ts_micros = ts_micros_now();
        let canon = format!(
            r#"{{"action":"{action}","target":"{target}","actor":"{actor}","ts_micros":{ts_micros}}}"#
        );
        let sig_bytes: [u8; 64] = sign(&self.key, canon.as_bytes());
        let audit_id = format!("audit-{ts_micros}");
        let entry = AuditEntry {
            audit_id,
            action: action.into(),
            target: target.into(),
            actor: actor.into(),
            reason: reason.into(),
            ts_micros,
            signature: hex::encode(sig_bytes),
        };
        self.entries.lock().unwrap().push(entry.clone());
        entry
    }

    pub fn all_entries(&self) -> Vec<AuditEntry> {
        self.entries.lock().unwrap().clone()
    }
}

fn ts_micros_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_micros() as u64)
        .unwrap_or(0)
}
