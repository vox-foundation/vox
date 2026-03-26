use turso::params;

use crate::store::types::{SkillManifestEntry, StoreError};

impl crate::VoxDb {
    // ── Skill Manifests (skill_manifests) ─────────────────────────────────────

    /// Upsert a row in `skill_manifests`. Returns `()` on success.
    ///
    /// Called from `vox-skills/src/registry.rs` `SkillRegistry::install`.
    pub async fn publish_skill(
        &self,
        id: &str,
        version: &str,
        manifest_json: &str,
        skill_md: &str,
    ) -> Result<(), StoreError> {
        self.conn
            .execute(
                "INSERT OR REPLACE INTO skill_manifests (id, version, manifest_json, skill_md, created_at)
                 VALUES (?1, ?2, ?3, ?4, datetime('now'))",
                params![id, version, manifest_json, skill_md],
            )
            .await?;
        Ok(())
    }

    /// Delete the `skill_manifests` row for `id`. No-op if absent.
    ///
    /// Called from `vox-skills/src/registry.rs` `SkillRegistry::uninstall`.
    pub async fn unpublish_skill(&self, id: &str) -> Result<(), StoreError> {
        self.conn
            .execute("DELETE FROM skill_manifests WHERE id = ?1", params![id])
            .await?;
        Ok(())
    }

    /// Return all rows from `skill_manifests`, ordered by `id`.
    ///
    /// Called from `vox-skills/src/registry.rs` `SkillRegistry::hydrate_from_db`.
    pub async fn list_skill_manifests(&self) -> Result<Vec<SkillManifestEntry>, StoreError> {
        let mut rows = self
            .conn
            .query(
                "SELECT id, version, manifest_json, COALESCE(skill_md,'') FROM skill_manifests ORDER BY id ASC",
                (),
            )
            .await?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().await? {
            out.push(SkillManifestEntry {
                id: row.get(0).map_err(|e| StoreError::Db(e.to_string()))?,
                version: row.get(1).map_err(|e| StoreError::Db(e.to_string()))?,
                manifest_json: row.get(2).map_err(|e| StoreError::Db(e.to_string()))?,
                skill_md: row.get(3).map_err(|e| StoreError::Db(e.to_string()))?,
            });
        }
        Ok(out)
    }
}
