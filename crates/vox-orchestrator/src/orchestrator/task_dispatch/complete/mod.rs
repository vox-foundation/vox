//! Task completion / attestation path. CI-side completion-quality rules and tiers live in
//! `contracts/operations/completion-policy.v1.yaml` (`vox ci completion-audit|gates`); this module
//! enforces runtime harness expectations and [`crate::types::CompletionAttestation`] adequacy.

mod fail;
mod harness;
mod impl_support;
mod success;
