//! VoxDB CRUD operations for Oratio ASR evaluations.
//!
//! Provides persistence for Word Error Rate (WER) evaluations and testing
//! runs against datasets like LibriSpeech and Vox-Code.

use crate::{StoreError, VoxDb};

pub use vox_db_types::store_types::oratio::*;

fn unix_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

impl VoxDb {
    /// Ensure the tables exist (usually handled by auto-migration or baseline, but DDL is embedded here).
    async fn ensure_oratio_eval_tables(&self) -> Result<(), StoreError> {
        self.conn
            .execute(
                "CREATE TABLE IF NOT EXISTS oratio_eval_run (
                run_id             TEXT    NOT NULL PRIMARY KEY,
                run_type           TEXT    NOT NULL,
                backend            TEXT    NOT NULL,
                model_id           TEXT,
                dataset_name       TEXT    NOT NULL,
                sample_count       INTEGER NOT NULL DEFAULT 0,
                total_ref_words    INTEGER NOT NULL DEFAULT 0,
                total_wer_errors   INTEGER NOT NULL DEFAULT 0,
                global_wer         REAL,
                global_cer         REAL,
                avg_latency_ms     REAL,
                avg_timing_offset_ms REAL,
                status             TEXT    NOT NULL DEFAULT 'running',
                notes              TEXT,
                created_at         INTEGER NOT NULL,
                updated_at         INTEGER NOT NULL
            )",
                (),
            )
            .await
            .map_err(|e| StoreError::Db(e.to_string()))?;

        self.conn
            .execute(
                "CREATE TABLE IF NOT EXISTS oratio_eval_sample (
                id                 INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id             TEXT    NOT NULL REFERENCES oratio_eval_run(run_id),
                audio_path         TEXT    NOT NULL,
                reference_text     TEXT    NOT NULL,
                hypothesis_text    TEXT    NOT NULL,
                wer                REAL    NOT NULL,
                cer                REAL    NOT NULL,
                latency_ms         INTEGER,
                segment_count      INTEGER,
                no_speech_dropped  INTEGER DEFAULT 0,
                created_at         INTEGER NOT NULL
            )",
                (),
            )
            .await
            .map_err(|e| StoreError::Db(e.to_string()))?;

        Ok(())
    }

    /// Persist a new evaluation run at status `running`.
    pub async fn record_oratio_eval_run_start(
        &self,
        params: &OratioEvalRunStartParams,
    ) -> Result<(), StoreError> {
        self.ensure_oratio_eval_tables().await?;
        let now = unix_now();
        self.conn.execute(
            "INSERT OR REPLACE INTO oratio_eval_run
             (run_id, run_type, backend, model_id, dataset_name, sample_count, total_ref_words, total_wer_errors, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, 0, 0, 'running', ?6, ?6)",
            turso::params![
                params.run_id.as_str(),
                params.run_type.as_str(),
                params.backend.as_str(),
                params.model_id.as_deref(),
                params.dataset_name.as_str(),
                now
            ],
        ).await.map_err(|e| StoreError::NotFound(format!("record_oratio_eval_run_start: {e}")))?;
        Ok(())
    }

    /// Append a completed sample to a running evaluation.
    #[allow(clippy::too_many_arguments)]
    pub async fn append_oratio_eval_sample(
        &self,
        run_id: &str,
        audio_path: &str,
        reference_text: &str,
        hypothesis_text: &str,
        wer: f32,
        cer: f32,
        latency_ms: Option<i64>,
        segment_count: Option<i32>,
        no_speech_dropped: i32,
    ) -> Result<(), StoreError> {
        self.ensure_oratio_eval_tables().await?;
        let now = unix_now();
        self.conn.execute(
            "INSERT INTO oratio_eval_sample
             (run_id, audio_path, reference_text, hypothesis_text, wer, cer, latency_ms, segment_count, no_speech_dropped, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            turso::params![
                run_id,
                audio_path,
                reference_text,
                hypothesis_text,
                wer as f64,
                cer as f64,
                latency_ms,
                segment_count.map(|s| s as i64),
                no_speech_dropped as i64,
                now
            ],
        ).await.map_err(|e| StoreError::NotFound(format!("append_oratio_eval_sample: {e}")))?;

        Ok(())
    }

    /// Mark run complete and compute global statistics.
    pub async fn complete_oratio_eval_run(
        &self,
        run_id: &str,
        global_wer: Option<f32>,
        global_cer: Option<f32>,
        avg_latency_ms: Option<f32>,
        avg_timing_offset_ms: Option<f32>,
    ) -> Result<(), StoreError> {
        self.ensure_oratio_eval_tables().await?;
        let now = unix_now();

        let mut rows = self
            .conn
            .query(
                "SELECT COUNT(*), SUM(wer) FROM oratio_eval_sample WHERE run_id = ?1",
                turso::params![run_id],
            )
            .await
            .map_err(|e| StoreError::NotFound(e.to_string()))?;

        let (sample_count, total_wer_errors) = if let Some(row) = rows.next().await.unwrap_or(None)
        {
            (
                row.get::<i64>(0).unwrap_or(0),
                row.get::<f64>(1).unwrap_or(0.0) as i64,
            )
        } else {
            (0, 0)
        };

        self.conn
            .execute(
                "UPDATE oratio_eval_run
             SET status = 'complete',
                 sample_count = ?2,
                 total_wer_errors = ?3,
                 global_wer = ?4,
                 global_cer = ?5,
                 avg_latency_ms = ?6,
                 avg_timing_offset_ms = ?7,
                 updated_at = ?8
             WHERE run_id = ?1",
                turso::params![
                    run_id,
                    sample_count,
                    total_wer_errors,
                    global_wer.map(|f| f as f64),
                    global_cer.map(|f| f as f64),
                    avg_latency_ms.map(|f| f as f64),
                    avg_timing_offset_ms.map(|f| f as f64),
                    now
                ],
            )
            .await
            .map_err(|e| StoreError::NotFound(format!("complete_oratio_eval_run: {e}")))?;
        Ok(())
    }

    /// Retrieve the recent evaluation runs.
    pub async fn get_recent_oratio_eval_runs(
        &self,
        limit: u32,
    ) -> Result<Vec<OratioEvalRunRecord>, StoreError> {
        self.ensure_oratio_eval_tables().await?;
        let mut rows = self.conn.query(
            "SELECT run_id, run_type, backend, model_id, dataset_name, sample_count, total_ref_words, total_wer_errors, global_wer, global_cer, avg_latency_ms, avg_timing_offset_ms, status, notes, created_at, updated_at
             FROM oratio_eval_run
             ORDER BY created_at DESC LIMIT ?1",
            turso::params![limit],
        ).await.map_err(|e| StoreError::NotFound(e.to_string()))?;

        let mut runs = Vec::new();
        while let Some(r) = rows.next().await.unwrap_or(None) {
            runs.push(OratioEvalRunRecord {
                run_id: r.get(0).unwrap_or_default(),
                run_type: r.get(1).unwrap_or_default(),
                backend: r.get(2).unwrap_or_default(),
                model_id: r.get(3).unwrap_or_default(),
                dataset_name: r.get(4).unwrap_or_default(),
                sample_count: r.get(5).unwrap_or(0),
                total_ref_words: r.get(6).unwrap_or(0),
                total_wer_errors: r.get(7).unwrap_or(0),
                global_wer: r.get::<Option<f64>>(8).unwrap_or(None).map(|v| v as f32),
                global_cer: r.get::<Option<f64>>(9).unwrap_or(None).map(|v| v as f32),
                avg_latency_ms: r.get::<Option<f64>>(10).unwrap_or(None).map(|v| v as f32),
                avg_timing_offset_ms: r.get::<Option<f64>>(11).unwrap_or(None).map(|v| v as f32),
                status: r.get(12).unwrap_or_default(),
                notes: r.get(13).unwrap_or(None),
                created_at: r.get(14).unwrap_or(0),
                updated_at: r.get(15).unwrap_or(0),
            });
        }
        Ok(runs)
    }
}
