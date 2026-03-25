use crate::migration::{Migration, validate_migrations};
use crate::{StoreError, store};

impl crate::VoxDb {
    /// Apply ordered migrations that have not yet been executed (same `schema_version` table as Arca).
    ///
    /// Returns versions that were newly applied.
    ///
    /// # SQL constraints
    ///
    /// Each [`Migration::up_sql`] is run with [`turso::Connection::execute_batch`]. It must **not**
    /// contain row-returning statements (no standalone `SELECT`; use DDL/DML only). See crate-level
    /// docs.
    pub async fn apply_migrations(&self, migrations: &[Migration]) -> Result<Vec<i64>, StoreError> {
        validate_migrations(migrations)?;
        self.connection()
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS schema_version (
                    version INTEGER PRIMARY KEY,
                    applied_at TEXT NOT NULL DEFAULT (datetime('now'))
                );",
            )
            .await?;

        let current = self.schema_version().await?;
        let mut applied = Vec::new();
        for migration in migrations {
            if migration.version <= current {
                continue;
            }
            self.connection().execute_batch(&migration.up_sql).await?;
            self.connection()
                .execute(
                    "INSERT INTO schema_version (version) VALUES (?1)",
                    (migration.version,),
                )
                .await?;
            applied.push(migration.version);
        }
        Ok(applied)
    }

    /// Append a training telemetry event to `agent_events` for orchestrator visibility.
    ///
    /// `event_kind` matches telemetry_schema constants (e.g. `"train_start"`, `"train_step"`, `"train_complete"`).
    /// `payload` is a JSON string of the event body.
    pub async fn record_training_event(
        &self,
        run_id: &str,
        event_kind: &str,
        payload: serde_json::Value,
    ) -> Result<(), store::StoreError> {
        let store = self;
        store
            .record_agent_event(
                &format!("populi_train:{run_id}"),
                event_kind,
                &payload.to_string(),
                env!("CARGO_PKG_VERSION"),
            )
            .await?;
        Ok(())
    }

    /// Record a checkpoint write event (adapter path, step, epoch) in `agent_events`.
    pub async fn record_training_checkpoint(
        &self,
        run_id: &str,
        epoch: u32,
        global_step: u32,
        adapter_path: &str,
    ) -> Result<(), store::StoreError> {
        self.record_training_event(
            run_id,
            "checkpoint_saved",
            serde_json::json!({
                "run_id": run_id,
                "epoch": epoch,
                "global_step": global_step,
                "adapter_path": adapter_path,
            }),
        )
        .await
    }

    /// Run `f` between `BEGIN` and `COMMIT` on this connection; `ROLLBACK` on error.
    ///
    /// **Caveat:** `f` is `.await`ed without holding a guard; avoid spawning work that uses the
    /// same `VoxDb` concurrently inside `f`. Prefer short, sequential async blocks.
    pub async fn transaction<F, T>(&self, f: F) -> Result<T, StoreError>
    where
        F: std::future::Future<Output = Result<T, StoreError>>,
    {
        let _ = self.conn.execute("BEGIN", ()).await?;
        match f.await {
            Ok(val) => {
                let _ = self.conn.execute("COMMIT", ()).await?;
                Ok(val)
            }
            Err(e) => {
                let _ = self.conn.execute("ROLLBACK", ()).await.ok();
                Err(e)
            }
        }
    }
}
