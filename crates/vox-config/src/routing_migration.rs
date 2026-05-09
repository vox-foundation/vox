//! Cutover + deprecation helpers for secrets-first LLM routing.
//!
//! `VOX_SECRETS_CUTOVER_PHASE` is defined in `vox-secrets`; routing code uses this module to decide
//! when legacy env-only configuration should hard-fail vs warn.

use std::sync::atomic::{AtomicBool, Ordering};

use vox_secrets::{OPERATOR_SECRETS_CUTOVER_PHASE, OPERATOR_SECRETS_MIGRATION_PHASE, SecretId};

static OPENROUTER_CHAT_ENV_WARN_EMITTED: AtomicBool = AtomicBool::new(false);

/// Returns true when `raw` is an enforce-style secrets cutover phase (`enforce`, `decommission`).
#[must_use]
pub fn secrets_cutover_blocks_legacy_env_raw(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "enforce" | "decommission"
    )
}

/// Whether the operator has selected an **enforce**-style secrets cutover (legacy env reads should fail).
#[must_use]
pub fn secrets_cutover_blocks_legacy_env() -> bool {
    let raw = std::env::var(OPERATOR_SECRETS_CUTOVER_PHASE)
        .or_else(|_| std::env::var(OPERATOR_SECRETS_MIGRATION_PHASE))
        .unwrap_or_default();
    secrets_cutover_blocks_legacy_env_raw(&raw)
}

/// One-time deprecation trace when `OPENROUTER_CHAT_MODEL` is present in the environment.
///
/// Values still resolve via secrets (which reads env among its sources); this is a migration nudge
/// toward `vox secrets set` for multi-machine sync.
pub fn trace_openrouter_chat_env_migration_once() {
    if OPENROUTER_CHAT_ENV_WARN_EMITTED.swap(true, Ordering::Relaxed) {
        return;
    }
    if std::env::var("OPENROUTER_CHAT_MODEL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .is_none()
    {
        return;
    }
    if secrets_cutover_blocks_legacy_env() {
        tracing::error!(
            target: "vox_config::deprecation",
            env = "OPENROUTER_CHAT_MODEL",
            secret_id = ?SecretId::VoxOpenRouterChatModel,
            "raw OPENROUTER_CHAT_MODEL is set while secrets cutover is in enforce/decommission — unset the env var and use `vox secrets set`"
        );
    } else {
        tracing::warn!(
            target: "vox_config::deprecation",
            env = "OPENROUTER_CHAT_MODEL",
            secret_id = ?SecretId::VoxOpenRouterChatModel,
            "OPENROUTER_CHAT_MODEL is set via environment; prefer `vox secrets set` / vault for synced routing preferences"
        );
    }
}
