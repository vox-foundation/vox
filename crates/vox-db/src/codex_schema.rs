//! Baseline-V1 schema readiness derived from the `vox-pm` manifest (capabilities, not numeric chains).
//!
//! The Arca store records a single `schema_version` row (**1**) after baseline DDL; readiness checks
//! required tables instead of comparing to historical V8…V15 numbers.

use turso::params;

use vox_pm::schema::{self, BASELINE_VERSION};

use crate::StoreError;
use crate::VoxDb;

/// Result of [`evaluate_codex_api_readiness`] for HTTP `/ready` and diagnostics.
#[derive(Debug, Clone)]
pub struct CodexApiReadiness {
    /// Highest applied built-in schema version (baseline Codex uses **1** only).
    pub schema_version: i64,
    /// Keccak-256 hex digest of manifest baseline SQL (`0x` + 64 hex digits).
    pub baseline_digest_hex: String,
    /// Subset of [`schema::CODEX_API_REQUIRED_TABLES`] absent from `sqlite_master`.
    pub missing_tables: Vec<String>,
    /// Whether the DB matches baseline version **and** exposes all API-required tables.
    pub ready: bool,
}

async fn table_present(db: &VoxDb, name: &str) -> Result<bool, StoreError> {
    let rows = db
        .query_all(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1 LIMIT 1",
            params![name],
        )
        .await?;
    Ok(!rows.is_empty())
}

/// Evaluate whether the Codex HTTP API surface can run (named queries, mutate, SSE, search status).
pub async fn evaluate_codex_api_readiness(db: &VoxDb) -> Result<CodexApiReadiness, StoreError> {
    let schema_version = db.schema_version().await?;
    let baseline_digest_hex = schema::schema_baseline_digest_hex();
    let mut missing_tables = Vec::new();
    for t in schema::CODEX_API_REQUIRED_TABLES {
        if !table_present(db, t).await? {
            missing_tables.push((*t).to_string());
        }
    }
    let ready = schema_version == BASELINE_VERSION && missing_tables.is_empty();
    Ok(CodexApiReadiness {
        schema_version,
        baseline_digest_hex,
        missing_tables,
        ready,
    })
}

/// Returns names from [`schema::CODEX_REACTIVITY_TABLES`] that are missing.
pub async fn missing_codex_reactivity_tables(db: &VoxDb) -> Result<Vec<String>, StoreError> {
    let mut missing = Vec::new();
    for t in schema::CODEX_REACTIVITY_TABLES {
        if !table_present(db, t).await? {
            missing.push((*t).to_string());
        }
    }
    Ok(missing)
}
