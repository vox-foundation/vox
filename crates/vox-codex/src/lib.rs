//! Compatibility facade: historical `vox_codex` crate name for **`vox_db::Codex`** (Arca/Turso).
//!
//! **Deprecation (aggressive reorg):** prefer **`vox_db`** in all new workspace code. This crate
//! remains for external consumers and legacy modules until a release notes a removal date; see
//! `docs/src/architecture/crate-build-lanes-migration.md` and `docs/src/architecture/crate-topology-buckets.md`.
//!
//! New code should depend on `vox-db` directly; this crate exists so unwired CLI modules and tools
//! keep a stable import path.

pub use vox_db::*;

/// Historical name for [`vox_db::DbConfig`].
pub type CodexConfig = vox_db::DbConfig;

/// Re-exports [`vox_db::paths`] (Arca filesystem layout helpers).
pub mod paths {
    pub use vox_db::paths::*;
}

/// Re-exports [`vox_db::learning`] (telemetry / learning hooks).
pub mod learning {
    pub use vox_db::learning::*;
}

/// Turso sync invocable engine alias.
pub mod sync_invocables {
    pub use vox_db::InvocableSyncEngine;
}

/// Re-exports [`vox_db::secrets`] (OS keyring helpers).
pub mod secrets {
    pub use vox_db::secrets::*;
}
