//! VoxDB CRUD operations for training run tracking.
//!
//! Provides persistence for Mens QLoRA training runs so that progress
//! survives crashes and can be queried from external tooling. The table is
//! created lazily on first write (no migration required).
//!
//! ## Table schema (created on first write)
//!
//! ```sql
//! CREATE TABLE IF NOT EXISTS populi_training_run (
//!     run_id             TEXT    NOT NULL PRIMARY KEY,
//!     adapter_tag        TEXT,
//!     model_name         TEXT,
//!     output_dir         TEXT    NOT NULL,
//!     data_dir           TEXT    NOT NULL,
//!     status             TEXT    NOT NULL DEFAULT 'running',
//!     epoch              INTEGER NOT NULL DEFAULT 0,
//!     global_step        INTEGER NOT NULL DEFAULT 0,
//!     planned_steps      INTEGER,
//!     last_loss          REAL,
//!     last_checkpoint_path TEXT,
//!     created_at         INTEGER NOT NULL,
//!     updated_at         INTEGER NOT NULL
//! );
//! ```
//!
//! `status` is one of: `running`, `paused`, `complete`, `failed`.

use serde::{Deserialize, Serialize};

use crate::{StoreError, VoxDb};

/// DDL executed lazily to ensure the table exists before any write.
const CREATE_TABLE: &str = "
CREATE TABLE IF NOT EXISTS populi_training_run (
    run_id               TEXT    NOT NULL PRIMARY KEY,
    adapter_tag          TEXT,
    model_name           TEXT,
    output_dir           TEXT    NOT NULL,
    data_dir             TEXT    NOT NULL,
    status               TEXT    NOT NULL DEFAULT 'running',
    epoch                INTEGER NOT NULL DEFAULT 0,
    global_step          INTEGER NOT NULL DEFAULT 0,
    planned_steps        INTEGER,
    last_loss            REAL,
    last_checkpoint_path TEXT,
    created_at           INTEGER NOT NULL,
    updated_at           INTEGER NOT NULL
);";

/// Row returned from [`VoxDb::get_training_run`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingRunRecord {
    /// Unique run identifier (matches `LoraTrainingConfig::run_id`).
    pub run_id: String,
    /// Optional adapter tag (`--adapter-tag` CLI flag).
    pub adapter_tag: Option<String>,
    /// Base model identifier (e.g. `Qwen/Qwen2.5-Coder-3B-Instruct`).
    pub model_name: Option<String>,
    /// Absolute path to the output directory.
    pub output_dir: String,
    /// Absolute path to the data directory.
    pub data_dir: String,
    /// One of: `running`, `paused`, `complete`, `failed`.
    pub status: String,
    /// Last completed epoch (0 = not yet started first epoch).
    pub epoch: u32,
    /// Total gradient steps completed.
    pub global_step: u32,
    /// Estimated total steps (upper bound from pre-flight count).
    pub planned_steps: Option<u32>,
    /// Training loss at the last recorded checkpoint.
    pub last_loss: Option<f32>,
    /// Absolute path to the latest safetensors checkpoint file.
    pub last_checkpoint_path: Option<String>,
    /// Unix timestamp (seconds) when this run was created.
    pub created_at: i64,
    /// Unix timestamp (seconds) of the latest update.
    pub updated_at: i64,
}

/// Parameters for [`VoxDb::record_training_run_start`].
#[derive(Debug, Clone)]
pub struct TrainingRunStartParams {
    /// Unique run identifier.
    pub run_id: String,
    /// Optional `--adapter-tag`.
    pub adapter_tag: Option<String>,
    /// Base model path or HF hub identifier.
    pub model_name: Option<String>,
    /// Absolute output directory.
    pub output_dir: String,
    /// Absolute data directory.
    pub data_dir: String,
    /// Upper-bound step count from pre-flight planning.
    pub planned_steps: Option<u32>,
}

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

impl VoxDb {
    /// Ensure the `populi_training_run` table exists.
    async fn ensure_training_run_table(&self) -> Result<(), StoreError> {
        self
            .conn
            .execute_batch(CREATE_TABLE)
            .await
            .map_err(|e: turso::Error| StoreError::NotFound(format!("ensure training_run table: {e}")))?;
        Ok(())
    }

    /// **CREATE** — persist a new training run at status `running`.
    ///
    /// Idempotent: if `run_id` already exists (e.g. a previous incomplete run)
    /// the row is replaced to reset status to `running` with the new params.
    pub async fn record_training_run_start(
        &self,
        params: &TrainingRunStartParams,
    ) -> Result<(), StoreError> {
        self.ensure_training_run_table().await?;
        let now = unix_now();
        self
            .conn
            .execute(
                "INSERT OR REPLACE INTO populi_training_run
                 (run_id, adapter_tag, model_name, output_dir, data_dir,
                  status, epoch, global_step, planned_steps,
                  last_loss, last_checkpoint_path, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, 'running', 0, 0, ?6, NULL, NULL, ?7, ?7)",
                turso::params![
                    params.run_id.as_str(),
                    params.adapter_tag.as_deref(),
                    params.model_name.as_deref(),
                    params.output_dir.as_str(),
                    params.data_dir.as_str(),
                    params.planned_steps.map(|s| s as i64),
                    now,
                ],
            )
            .await
            .map_err(|e: turso::Error| StoreError::NotFound(format!("record_training_run_start: {e}")))?;
        Ok(())
    }

    /// **READ** — fetch a training run by `run_id`. Returns `None` if not found.
    pub async fn get_training_run(
        &self,
        run_id: &str,
    ) -> Result<Option<TrainingRunRecord>, StoreError> {
        self.ensure_training_run_table().await?;
        let rows = self
            .connection()
            .query(
                "SELECT run_id, adapter_tag, model_name, output_dir, data_dir,
                        status, epoch, global_step, planned_steps,
                        last_loss, last_checkpoint_path, created_at, updated_at
                 FROM populi_training_run
                 WHERE run_id = ?1",
                turso::params![run_id],
            )
            .await
            .map_err(|e: turso::Error| StoreError::NotFound(format!("get_training_run query: {e}")))?;

        let mut rows = rows;
        if let Some(row) = rows.next().await.map_err(|e: turso::Error| {
            StoreError::NotFound(format!("get_training_run row: {e}"))
        })? {
            Ok(Some(row_to_record(&row)?))
        } else {
            Ok(None)
        }
    }

    /// **UPDATE** — record a mid-epoch or epoch-boundary checkpoint.
    pub async fn update_training_checkpoint(
        &self,
        run_id: &str,
        epoch: u32,
        global_step: u32,
        last_loss: Option<f32>,
        checkpoint_path: Option<&str>,
    ) -> Result<(), StoreError> {
        self.ensure_training_run_table().await?;
        let now = unix_now();
        self
            .conn
            .execute(
                "UPDATE populi_training_run
                 SET epoch = ?2, global_step = ?3, last_loss = ?4,
                     last_checkpoint_path = ?5, updated_at = ?6
                 WHERE run_id = ?1",
                turso::params![
                    run_id,
                    epoch as i64,
                    global_step as i64,
                    last_loss.map(|f| f as f64),
                    checkpoint_path,
                    now,
                ],
            )
            .await
            .map_err(|e: turso::Error| StoreError::NotFound(format!("update_training_checkpoint: {e}")))?;
        Ok(())
    }

    /// **UPDATE** — mark a run as successfully completed.
    pub async fn mark_training_complete(
        &self,
        run_id: &str,
        global_step: u32,
        final_adapter_path: Option<&str>,
    ) -> Result<(), StoreError> {
        self.ensure_training_run_table().await?;
        let now = unix_now();
        self
            .conn
            .execute(
                "UPDATE populi_training_run
                 SET status = 'complete', global_step = ?2,
                     last_checkpoint_path = COALESCE(?3, last_checkpoint_path),
                     updated_at = ?4
                 WHERE run_id = ?1",
                turso::params![
                    run_id,
                    global_step as i64,
                    final_adapter_path,
                    now,
                ],
            )
            .await
            .map_err(|e: turso::Error| StoreError::NotFound(format!("mark_training_complete: {e}")))?;
        Ok(())
    }

    /// **UPDATE** — mark a run as failed with an optional error message in the status.
    pub async fn mark_training_failed(
        &self,
        run_id: &str,
        global_step: u32,
    ) -> Result<(), StoreError> {
        self.ensure_training_run_table().await?;
        let now = unix_now();
        self
            .conn
            .execute(
                "UPDATE populi_training_run
                 SET status = 'failed', global_step = ?2, updated_at = ?3
                 WHERE run_id = ?1",
                turso::params![run_id, global_step as i64, now],
            )
            .await
            .map_err(|e: turso::Error| StoreError::NotFound(format!("mark_training_failed: {e}")))?;
        Ok(())
    }

    /// **READ** — list the most recent N training runs (ordered by `created_at DESC`).
    pub async fn list_training_runs(
        &self,
        limit: u32,
    ) -> Result<Vec<TrainingRunRecord>, StoreError> {
        self.ensure_training_run_table().await?;
        let rows = self
            .connection()
            .query(
                "SELECT run_id, adapter_tag, model_name, output_dir, data_dir,
                        status, epoch, global_step, planned_steps,
                        last_loss, last_checkpoint_path, created_at, updated_at
                 FROM populi_training_run
                 ORDER BY created_at DESC
                 LIMIT ?1",
                turso::params![limit as i64],
            )
            .await
            .map_err(|e: turso::Error| StoreError::NotFound(format!("list_training_runs query: {e}")))?;

        let mut out = Vec::new();
        let mut rows = rows;
        while let Some(row) = rows.next().await.map_err(|e: turso::Error| {
            StoreError::NotFound(format!("list_training_runs row: {e}"))
        })? {
            out.push(row_to_record(&row)?);
        }
        Ok(out)
    }
}

fn row_to_record(row: &turso::Row) -> Result<TrainingRunRecord, StoreError> {
    Ok(TrainingRunRecord {
        run_id: row
            .get::<String>(0)
            .map_err(|e: turso::Error| StoreError::NotFound(format!("row run_id: {e}")))?,
        adapter_tag: row
            .get::<Option<String>>(1)
            .map_err(|e: turso::Error| StoreError::NotFound(format!("row adapter_tag: {e}")))?,
        model_name: row
            .get::<Option<String>>(2)
            .map_err(|e: turso::Error| StoreError::NotFound(format!("row model_name: {e}")))?,
        output_dir: row
            .get::<String>(3)
            .map_err(|e: turso::Error| StoreError::NotFound(format!("row output_dir: {e}")))?,
        data_dir: row
            .get::<String>(4)
            .map_err(|e: turso::Error| StoreError::NotFound(format!("row data_dir: {e}")))?,
        status: row
            .get::<String>(5)
            .map_err(|e: turso::Error| StoreError::NotFound(format!("row status: {e}")))?,
        epoch: row
            .get::<i64>(6)
            .map_err(|e: turso::Error| StoreError::NotFound(format!("row epoch: {e}")))? as u32,
        global_step: row
            .get::<i64>(7)
            .map_err(|e: turso::Error| StoreError::NotFound(format!("row global_step: {e}")))? as u32,
        planned_steps: row
            .get::<Option<i64>>(8)
            .map_err(|e: turso::Error| StoreError::NotFound(format!("row planned_steps: {e}")))?
            .map(|v| v as u32),
        last_loss: row
            .get::<Option<f64>>(9)
            .map_err(|e: turso::Error| StoreError::NotFound(format!("row last_loss: {e}")))?
            .map(|v| v as f32),
        last_checkpoint_path: row
            .get::<Option<String>>(10)
            .map_err(|e: turso::Error| StoreError::NotFound(format!("row last_checkpoint_path: {e}")))?,
        created_at: row
            .get::<i64>(11)
            .map_err(|e: turso::Error| StoreError::NotFound(format!("row created_at: {e}")))?,
        updated_at: row
            .get::<i64>(12)
            .map_err(|e: turso::Error| StoreError::NotFound(format!("row updated_at: {e}")))?,
    })
}
