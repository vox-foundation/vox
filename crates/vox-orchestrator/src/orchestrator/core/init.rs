use crate::orchestrator::types::OrchestratorError;

impl crate::orchestrator::Orchestrator {
    /// Initialize the orchestrator database schema and set the DB handle.
    pub async fn init_db(
        &self,
        db: std::sync::Arc<vox_db::VoxDb>,
    ) -> Result<(), OrchestratorError> {
        db.sync_schema_from_digest(&crate::schema::orchestrator_schema())
            .await
            .map_err(|e| OrchestratorError::DatabaseError(format!("DB sync failed: {}", e)))?;

        crate::sync_lock::rw_write(&*self.db).replace(db.clone());
        match db.sqlite_capabilities_snapshot().await {
            Ok(p) => {
                tracing::debug!(
                    journal_mode = %p.journal_mode,
                    foreign_keys_on = p.foreign_keys_on,
                    fts5_reported = p.fts5_reported,
                    "sqlite capabilities (orchestrator init_db)"
                );
            }
            Err(e) => {
                tracing::debug!(error = %e, "sqlite capability probe failed during orchestrator init_db");
            }
        }
        Ok(())
    }

    /// Builder-style variant of [`Self::init_db`] (takes ownership, sets db, returns self).
    pub fn with_db(self, db: std::sync::Arc<vox_db::VoxDb>) -> Self {
        crate::sync_lock::rw_write(&*self.db).replace(db);
        self
    }

    /// Attach a database handle late (e.g. after async MCP connection).
    pub fn attach_db(&self, db: std::sync::Arc<vox_db::VoxDb>) {
        crate::sync_lock::rw_write(&*self.db).replace(db);
    }
}
