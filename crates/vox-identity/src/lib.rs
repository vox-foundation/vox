//! Vox identity: per-user keypair, signing challenges, trust ledger.

pub mod challenge;
/// Per-job ephemeral Ed25519 subkey for result attestation (P5-T6).
pub mod ephemeral;
pub mod identity;
pub mod storage;
pub mod trust;

pub use ephemeral::EphemeralSigner;
pub use identity::NodeIdentity;
pub use trust::{TrustedNode, TrustedNodeRegistry};
