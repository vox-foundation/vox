//! Cutover + deprecation helpers for Clavis-first LLM routing.
//!
//! `VOX_CLAVIS_CUTOVER_PHASE` is defined in `vox-clavis`; routing code uses this module to decide
//! when legacy env-only configuration should hard-fail vs warn.

use std::sync::atomic::{AtomicBool, Ordering};

use vox_clavis::{OPERATOR_CLAVIS_CUTOVER_PHASE, OPERATOR_CLAVIS_MIGRATION_PHASE, SecretId};

static OPENROUTER_CHAT_ENV_WARN_EMITTED: AtomicBool = AtomicBool::new(false);

/// Returns true when `raw` is an enforce-style Clavis cutover phase (`enforce`, `decommission`).
#[must_use]
pub fn clavis_cutover_blocks_legacy_env_raw(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "enforce" | "decommission"
    )
}

/// Whether the operator has selected an **enforce**-style Clavis cutover (legacy env reads should fail).
#[must_use]
pub fn clavis_cutover_blocks_legacy_env() -> bool {
    let raw = std::env::var(OPERATOR_CLAVIS_CUTOVER_PHASE)
        .or_else(|_| std::env::var(OPERATOR_CLAVIS_MIGRATION_PHASE))
        .unwrap_or_default();
    clavis_cutover_blocks_legacy_env_raw(&raw)
}

/// One-time deprecation trace when `OPENROUTER_CHAT_MODEL` is present in the environment.
///
/// Values still resolve via Clavis (which reads env among its sources); this is a migration nudge
/// toward `vox clavis set` for multi-machine sync.
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
    if clavis_cutover_blocks_legacy_env() {
        tracing::error!(
            target: "vox_config::deprecation",
            env = "OPENROUTER_CHAT_MODEL",
            secret_id = ?SecretId::VoxOpenRouterChatModel,
            "raw OPENROUTER_CHAT_MODEL is set while Clavis cutover is in enforce/decommission — unset the env var and use `vox clavis set`"
        );
    } else {
        tracing::warn!(
            target: "vox_config::deprecation",
            env = "OPENROUTER_CHAT_MODEL",
            secret_id = ?SecretId::VoxOpenRouterChatModel,
            "OPENROUTER_CHAT_MODEL is set via environment; prefer `vox clavis set` / vault for synced routing preferences"
        );
    }
}
