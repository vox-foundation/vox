//! Arca CRUD for build-observability tables (`build_run`, `build_crate_sample`, `build_warning`).

use crate::arca_store::types::StoreError;
use serde::Serialize;

/// Summary returned by [`VoxDb::query_build_health`].
#[derive(Debug, Clone, Serialize)]
pub struct BuildHealthSummary {
    /// Total wall-clock of the latest run in milliseconds.
    pub total_ms: i64,
    /// Number of crates compiled (not served from cache) in the latest run.
    pub compiled: i64,
    /// Number of crates served from cache in the latest run.
    pub cached: i64,
    /// Up to 5 slowest non-fresh crates in the latest run.
    pub slowest: Vec<CrateSample>,
    /// Number of warnings emitted during the latest run.
    pub warning_count: i64,
    /// Whether the dep-graph fingerprint changed vs the previous run.
    pub dep_changed: bool,
}

/// One crate timing sample (subset used in health/regression reports).
#[derive(Debug, Clone, Serialize)]
pub struct CrateSample {
    /// Crate name (package id short form).
    pub name: String,
    /// Elapsed compile time in milliseconds (`None` when only freshness is known).
    pub elapsed_ms: Option<i64>,
    /// Actionable hint for this crate, if any.
    pub hint: Option<String>,
}

/// One regression row returned by [`VoxDb::query_build_regressions`].
#[derive(Debug, Clone, Serialize)]
pub struct RegressionRow {
    /// Crate name.
    pub name: String,
    /// Elapsed time in the latest run (ms).
    pub elapsed_ms: i64,
    /// Rolling average across previous runs (ms).
    pub avg_ms: f64,
    /// `elapsed_ms / avg_ms` ratio (≥ 1.5 → regression).
    pub ratio: f64,
    /// Actionable hint for this crate.
    pub hint: Option<String>,
}

/// One warning row returned by [`VoxDb::query_build_warnings`].
#[derive(Debug, Clone, Serialize)]
pub struct WarningRow {
    /// Crate that generated the warning.
    pub crate_name: String,
    /// Warning code (e.g. `dead_code`, `E0433`).
    pub code: Option<String>,
    /// Number of times this (crate, code) pair has appeared across all tracked runs.
    pub occurrences: i64,
    /// Most recent warning message text.
    pub message: String,
    /// Actionable hint.
    pub hint: Option<String>,
}

impl crate::VoxDb {
    /// Insert a new build run row; returns the new `id`.
    pub async fn insert_build_run(
        &self,
        repository_id: &str,
        run_name: Option<&str>,
        rustc_version: Option<&str>,
        profile: &str,
        total_ms: i64,
        crate_count: i64,
        fresh_count: i64,
        dep_fingerprint: Option<&str>,
    ) -> Result<i64, StoreError> {
        self.conn.execute(
            "INSERT INTO build_run (repository_id, run_name, rustc_version, profile, total_ms,
             crate_count, fresh_count, dep_fingerprint)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            (
                repository_id,
                run_name,
                rustc_version,
                profile,
                total_ms,
                crate_count,
                fresh_count,
                dep_fingerprint,
            ),
        ).await?;
        let mut rows = self.conn.query("SELECT last_insert_rowid()", ()).await?;
        let id: i64 = rows.next().await?.and_then(|r| r.get(0).ok()).unwrap_or(0);
        Ok(id)
    }

    /// Bulk-insert crate samples for a run (best-effort; skips on error).
    pub async fn insert_crate_samples(
        &self,
        run_id: i64,
        samples: &[(&str, Option<&str>, Option<i64>, bool, Option<&str>)],
    ) -> Result<(), StoreError> {
        for (name, version, elapsed_ms, fresh, features) in samples {
            let _ = self.conn.execute(
                "INSERT INTO build_crate_sample (run_id, name, version, elapsed_ms, fresh, features)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                (run_id, *name, *version, *elapsed_ms, if *fresh { 1i64 } else { 0i64 }, *features),
            ).await;
        }
        Ok(())
    }

    /// Bulk-insert warnings for a run (best-effort; skips on error).
    pub async fn insert_build_warnings(
        &self,
        run_id: i64,
        warnings: &[(&str, &str, Option<&str>, &str)],
    ) -> Result<(), StoreError> {
        for (crate_name, level, code, message) in warnings {
            let _ = self.conn.execute(
                "INSERT INTO build_warning (run_id, crate_name, level, code, message)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                (run_id, *crate_name, *level, *code, *message),
            ).await;
        }
        Ok(())
    }

    /// Build health summary for the latest run in this repository.
    pub async fn query_build_health(
        &self,
        repository_id: &str,
    ) -> Result<Option<BuildHealthSummary>, StoreError> {
        // Latest run id
        let mut rows = self.conn.query(
            "SELECT id, total_ms, crate_count, fresh_count, dep_fingerprint
             FROM build_run WHERE repository_id = ?1
             ORDER BY recorded_at DESC LIMIT 1",
            (repository_id,),
        ).await?;

        let Some(row) = rows.next().await? else { return Ok(None) };
        let run_id: i64 = row.get(0)?;
        let total_ms: i64 = row.get(1)?;
        let crate_count: i64 = row.get(2)?;
        let fresh_count: i64 = row.get(3)?;
        let dep_fp: Option<String> = row.get(4).ok().flatten();

        // Previous fingerprint
        let mut prev_rows = self.conn.query(
            "SELECT dep_fingerprint FROM build_run WHERE repository_id = ?1
             ORDER BY recorded_at DESC LIMIT 1 OFFSET 1",
            (repository_id,),
        ).await?;
        let prev_fp: Option<String> = prev_rows.next().await?
            .and_then(|r| r.get(0).ok()).flatten();
        let dep_changed = dep_fp != prev_fp;

        // Slowest crates
        let mut s_rows = self.conn.query(
            "SELECT name, elapsed_ms FROM build_crate_sample
             WHERE run_id = ?1 AND fresh = 0
             ORDER BY elapsed_ms DESC LIMIT 5",
            (run_id,),
        ).await?;
        let mut slowest = Vec::new();
        while let Some(r) = s_rows.next().await? {
            let name: String = r.get(0)?;
            let elapsed_ms: Option<i64> = r.get(1).ok();
            let hint = elapsed_ms.and(
                crate::build_hints::lookup_hint(&name, None).map(|s| s.to_string())
            );
            slowest.push(CrateSample { name, elapsed_ms, hint });
        }

        // Warning count
        let mut w_rows = self.conn.query(
            "SELECT COUNT(*) FROM build_warning WHERE run_id = ?1",
            (run_id,),
        ).await?;
        let warning_count: i64 = w_rows.next().await?
            .and_then(|r| r.get(0).ok()).unwrap_or(0);

        Ok(Some(BuildHealthSummary {
            total_ms,
            compiled: crate_count - fresh_count,
            cached: fresh_count,
            slowest,
            warning_count,
            dep_changed,
        }))
    }

    /// Return crates whose latest compile time exceeds 1.5× their historical average.
    pub async fn query_build_regressions(
        &self,
        repository_id: &str,
        run_id: i64,
    ) -> Result<Vec<RegressionRow>, StoreError> {
        let mut rows = self.conn.query(
            "WITH baseline AS (
                SELECT cs.name, AVG(cs.elapsed_ms) AS avg_ms
                FROM build_crate_sample cs
                JOIN build_run br ON cs.run_id = br.id
                WHERE br.repository_id = ?1 AND br.id < ?2 AND cs.fresh = 0
                GROUP BY cs.name
            )
            SELECT * FROM (
                SELECT cs.name, cs.elapsed_ms, b.avg_ms,
                       CAST(cs.elapsed_ms AS REAL) / b.avg_ms AS ratio
                FROM build_crate_sample cs
                JOIN build_run br ON cs.run_id = br.id
                JOIN baseline b ON cs.name = b.name
                WHERE br.id = ?2
            )
            WHERE ratio >= 1.5
            ORDER BY ratio DESC",
            (repository_id, run_id),
        ).await?;

        let mut results = Vec::new();
        while let Some(r) = rows.next().await? {
            let name: String = r.get(0)?;
            let elapsed_ms: i64 = r.get(1).unwrap_or(0);
            let avg_ms: f64 = r.get(2).unwrap_or(0.0);
            let ratio: f64 = r.get(3).unwrap_or(0.0);
            let hint = crate::build_hints::lookup_hint(&name, None)
                .map(|s| s.to_string());
            results.push(RegressionRow { name, elapsed_ms, avg_ms, ratio, hint });
        }
        Ok(results)
    }

    /// Return the most-seen (crate, code) warning pairs across recent runs.
    pub async fn query_build_warnings(
        &self,
        repository_id: &str,
        limit: i64,
    ) -> Result<Vec<WarningRow>, StoreError> {
        let mut rows = self.conn.query(
            "SELECT bw.crate_name, COALESCE(bw.code, '') AS code_fixed, COUNT(*) as occ, MAX(bw.message)
             FROM build_warning bw
             JOIN build_run br ON bw.run_id = br.id
             WHERE br.repository_id = ?1
             GROUP BY bw.crate_name, code_fixed
             ORDER BY occ DESC
             LIMIT ?2",
            (repository_id, limit),
        ).await?;

        let mut results = Vec::new();
        while let Some(r) = rows.next().await? {
            let crate_name: String = r.get(0)?;
            let raw_code: String = r.get(1)?;
            let code = if raw_code.is_empty() { None } else { Some(raw_code) };
            let occurrences: i64 = r.get(2).unwrap_or(0);
            let message: String = r.get(3).unwrap_or_default();
            let hint = crate::build_hints::lookup_hint(
                &crate_name, code.as_deref()
            ).map(|s| s.to_string());
            results.push(WarningRow { crate_name, code, occurrences, message, hint });
        }
        Ok(results)
    }
}
