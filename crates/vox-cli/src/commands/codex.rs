//! `vox codex` — verify Arca/Codex, export/import legacy JSONL, Socrates telemetry rollups.

use anyhow::Context;
use std::path::PathBuf;
use vox_db::codex_legacy::{
    LegacyVerification, export_legacy_jsonl, import_legacy_jsonl, verify_legacy_store,
};
use vox_db::{Codex, DbConfig, StoreError};

fn resolve_config() -> anyhow::Result<DbConfig> {
    DbConfig::resolve_standalone().map_err(anyhow::Error::msg)
}

/// Inspect schema version and Codex reactivity tables.
pub async fn verify() -> anyhow::Result<()> {
    let config = resolve_config()?;
    let db = match Codex::connect(config).await {
        Ok(db) => db,
        Err(StoreError::LegacySchemaChain { max_version }) => {
            anyhow::bail!(
                "legacy Arca schema chain detected (schema_version max={max_version}).\n\
                 Remediation:\n  1. Export: `vox codex export-legacy backup.jsonl` (export opens the DB without applying baseline migration).\n  2. Point VOX_DB_PATH at a new file or remove the old database file.\n  3. Open Codex once (e.g. `vox codex verify`) to apply baseline V1.\n  4. Import: `vox codex import-legacy backup.jsonl`."
            );
        }
        Err(e) => return Err(anyhow::anyhow!("{e}")),
    };
    let v: LegacyVerification = verify_legacy_store(db.store())
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("schema_version: {}", v.schema_version);
    println!(
        "codex_reactivity_tables_ok (manifest): {}",
        v.has_codex_reactivity
    );
    println!("legacy_multi_version_chain: {}", v.is_legacy_schema_chain);
    Ok(())
}

/// Write [`vox_db::codex_legacy::LEGACY_EXPORT_TABLES`] rows as JSONL.
///
/// Uses a connection that **skips** baseline migration so pre-baseline databases can still be dumped.
pub async fn export_legacy(out: &PathBuf) -> anyhow::Result<()> {
    let db = Codex::connect_legacy_export_only(resolve_config()?)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut buf = Vec::<u8>::new();
    let n = export_legacy_jsonl(db.store(), &mut buf)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    tokio::fs::write(out, buf)
        .await
        .with_context(|| format!("write {}", out.display()))?;
    println!("Exported {n} row(s) to {}", out.display());
    Ok(())
}

/// Restore rows from [`export_legacy`] into a **baseline V1** database (normal connect).
pub async fn import_legacy(path: &PathBuf) -> anyhow::Result<()> {
    let db = Codex::connect(resolve_config()?)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let file = std::fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    let reader = std::io::BufReader::new(file);
    let n = import_legacy_jsonl(db.store(), reader)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Imported {n} row(s) from {}", path.display());
    Ok(())
}

/// Print aggregated Socrates surface metrics (`research_metrics` / `socrates_surface`) as JSON.
pub async fn socrates_metrics(repository_id: Option<String>, limit: i64) -> anyhow::Result<()> {
    let db = Codex::connect(resolve_config()?)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let agg = db
        .aggregate_socrates_surface_metrics(repository_id.as_deref(), limit)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&agg).context("serialize SocratesSurfaceAggregate")?
    );
    Ok(())
}

/// Roll up recent Socrates events and append one `eval_runs` row (batch / scheduled jobs).
pub async fn socrates_eval_snapshot(
    eval_id: String,
    repository_id: Option<String>,
    limit: i64,
) -> anyhow::Result<()> {
    let db = Codex::connect(resolve_config()?)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let row_id = db
        .record_socrates_eval_summary(&eval_id, repository_id.as_deref(), limit)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Recorded eval_runs id={row_id} eval_id={eval_id}");
    Ok(())
}
