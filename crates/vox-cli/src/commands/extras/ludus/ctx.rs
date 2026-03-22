//! `LudusContext` — shared context helper that eliminates 20-line boilerplate
//! from every `vox ludus` CLI command.
//!
//! ## Before
//! ```ignore
//! let db = db_util::get_db().await?;
//! let user_id = vox_db::paths::local_user_id();
//! let mut profile = db::get_profile(&db, &user_id)
//!     .await.unwrap_or_default()
//!     .unwrap_or_else(|| LudusProfile::new_default(&user_id));
//! ```
//!
//! ## After
//! ```ignore
//! let ctx = LudusContext::load().await?;
//! ```

use anyhow::Result;
use vox_db::Codex;
use vox_ludus::{LudusProfile, db};

use super::db_util;

/// All the context a `vox ludus` command typically needs, loaded in one call.
pub struct LudusContext {
    /// Live database handle.
    pub db: Codex,
    /// Canonical local user ID.
    pub user_id: String,
    /// Loaded (or freshly created) player profile.
    pub profile: LudusProfile,
}

impl LudusContext {
    /// Open the DB, resolve user ID, and load (or create) the player profile.
    ///
    /// Energy is regenerated and the profile is re-persisted on every load
    /// so that time-based regeneration is always current.
    pub async fn load() -> Result<Self> {
        let db = db_util::get_db().await?;
        let user_id = vox_db::paths::local_user_id();
        let mut profile = db::get_profile(&db, &user_id)
            .await
            .unwrap_or_default()
            .unwrap_or_else(|| LudusProfile::new_default(&user_id));
        profile.regen_energy();
        let _ = db::upsert_profile(&db, &profile).await;
        Ok(Self {
            db,
            user_id,
            profile,
        })
    }

    /// Like `load()` but returns `Ok(None)` instead of an error when the DB is
    /// unavailable — useful for fire-and-forget background tasks.
    pub async fn try_load() -> Option<Self> {
        Self::load().await.ok()
    }

    /// Re-save the profile after mutations.
    pub async fn save_profile(&self) -> Result<()> {
        db::upsert_profile(&self.db, &self.profile).await
    }
}
