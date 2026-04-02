//! `vox codex` — verify Arca/Codex, export/import legacy JSONL, Socrates telemetry rollups.

use anyhow::Context;
use std::path::PathBuf;
use vox_db::legacy::codex::{
    LegacyVerification, export_legacy_jsonl, import_legacy_jsonl, verify_legacy_store,
};
use vox_db::legacy::import_extras::{
    import_orchestrator_memory_dir, import_skill_bundle_json_file,
};
use vox_db::{Codex, DbConfig, StoreError};

fn resolve_config() -> anyhow::Result<DbConfig> {
    DbConfig::resolve_canonical().map_err(anyhow::Error::msg)
}

/// Inspect schema version and Codex reactivity tables.
pub async fn verify() -> anyhow::Result<()> {
    let config = resolve_config()?;
    let db = match Codex::connect(config).await {
        Ok(db) => db,
        Err(StoreError::LegacySchemaChain { max_version }) => {
            anyhow::bail!(
                "non-baseline Arca schema detected (schema_version max={max_version}).\n\
                 Remediation:\n  1. Export: `vox codex export-legacy backup.jsonl` (export opens the DB without applying baseline migration).\n  2. Point VOX_DB_PATH at a new file or remove the old database file.\n  3. Open Codex once (e.g. `vox codex verify`) to apply the current baseline.\n  4. Import: `vox codex import-legacy backup.jsonl`."
            );
        }
        Err(e) => return Err(anyhow::anyhow!("{e}")),
    };
    let v: LegacyVerification = verify_legacy_store(&db)
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

/// Write [`vox_db::legacy::codex::LEGACY_EXPORT_TABLES`] rows as JSONL.
///
/// Uses a connection that **skips** baseline migration so pre-baseline databases can still be dumped.
pub async fn export_legacy(out: &PathBuf) -> anyhow::Result<()> {
    let db = Codex::connect_legacy_export_only(resolve_config()?)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let mut buf = Vec::<u8>::new();
    let n = export_legacy_jsonl(&db, &mut buf)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    tokio::fs::write(out, buf)
        .await
        .with_context(|| format!("write {}", out.display()))?;
    println!("Exported {n} row(s) to {}", out.display());
    Ok(())
}

/// Restore rows from [`export_legacy`] into a **baseline** database (normal connect).
///
/// **Semantics:** `import-legacy` is **replace**, not append: every allowlisted user table is
/// cleared first, then rows from the JSONL stream are inserted. Use a fresh target DB or expect a
/// full overwrite of imported domains.
pub async fn import_legacy(path: &PathBuf) -> anyhow::Result<()> {
    let db = Codex::connect(resolve_config()?)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let file = std::fs::File::open(path).with_context(|| format!("open {}", path.display()))?;
    let reader = std::io::BufReader::new(file);
    let n = import_legacy_jsonl(&db, reader)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Imported {n} row(s) from {}", path.display());
    Ok(())
}

/// Export legacy-chain DB from disk → JSONL + sidecar, create `target_db`, import, verify baseline.
pub async fn cutover(
    artifact_dir: Option<PathBuf>,
    target_db: PathBuf,
    source_db: Option<PathBuf>,
    force: bool,
) -> anyhow::Result<()> {
    let source_cfg = match source_db {
        Some(p) => vox_db::DbConfig::local(p.to_string_lossy().to_string()),
        None => {
            let c = resolve_config()?;
            match &c {
                vox_db::DbConfig::Local { .. } => c,
                other => {
                    anyhow::bail!(
                        "vox codex cutover needs a local legacy SQLite path.\n\
                         Pass `--source-db <path.db>` or set `VOX_DB_PATH` to the legacy file.\n\
                         Got config: {other:?}"
                    );
                }
            }
        }
    };

    let export_db = Codex::connect_legacy_export_only(source_cfg.clone())
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let leg = verify_legacy_store(&export_db)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    if !leg.is_legacy_schema_chain {
        anyhow::bail!(
            "Source is not a legacy multi-version schema chain (schema_version max={}).\n\
             `cutover` is only for databases that still carry a pre-baseline migration chain.\n\
             Already-baseline stores: use `vox codex export-legacy` / `import-legacy` manually if you only want a copy.",
            leg.schema_version
        );
    }

    let dir = artifact_dir.unwrap_or(std::env::current_dir()?);
    let ts = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    let jsonl_name = format!("codex-cutover-{ts}.jsonl");
    let jsonl_path = dir.join(&jsonl_name);

    let mut buf = Vec::<u8>::new();
    let n = export_legacy_jsonl(&export_db, &mut buf)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    tokio::fs::write(&jsonl_path, &buf)
        .await
        .with_context(|| format!("write {}", jsonl_path.display()))?;

    let target_str = target_db.to_string_lossy().into_owned();
    if target_db.exists() && !force {
        anyhow::bail!(
            "target {} already exists; re-run with --force after backup",
            target_db.display()
        );
    }
    if target_db.exists() {
        tokio::fs::remove_file(&target_db)
            .await
            .with_context(|| format!("remove {}", target_db.display()))?;
    }
    if let Some(parent) = target_db.parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("create_dir_all {}", parent.display()))?;
        }
    }

    let fresh = Codex::connect(vox_db::DbConfig::local(target_str.clone()))
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let file = std::fs::File::open(&jsonl_path)
        .with_context(|| format!("open {}", jsonl_path.display()))?;
    let reader = std::io::BufReader::new(file);
    let imported = import_legacy_jsonl(&fresh, reader)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let v2 = verify_legacy_store(&fresh)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    if v2.is_legacy_schema_chain {
        anyhow::bail!("internal error: target still reports legacy schema chain after import");
    }

    let source_path = match &source_cfg {
        vox_db::DbConfig::Local { path } => path.clone(),
        _ => "unknown".to_string(),
    };
    let sidecar = serde_json::json!({
        "kind": "vox_codex_cutover_v1",
        "timestamp_utc": ts.to_string(),
        "source_db_path": source_path,
        "target_db_path": target_str,
        "artifact_jsonl": jsonl_path.to_string_lossy(),
        "exported_rows": n,
        "imported_rows": imported,
        "source_schema_version_max": leg.schema_version,
        "target_schema_version": v2.schema_version,
        "codex_reactivity_ok": v2.has_codex_reactivity,
    });
    let sidecar_path = dir.join(format!("codex-cutover-{ts}.sidecar.json"));
    tokio::fs::write(
        &sidecar_path,
        serde_json::to_string_pretty(&sidecar).context("sidecar json")?,
    )
    .await
    .with_context(|| format!("write {}", sidecar_path.display()))?;

    println!("Cutover complete.");
    println!("  JSONL   : {}", jsonl_path.display());
    println!("  Sidecar : {}", sidecar_path.display());
    println!("  Rows    : exported {n}, imported {imported}");
    println!("  Next    : export VOX_DB_PATH={}", target_str);
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
pub async fn import_orchestrator_memory(
    dir: PathBuf,
    agent_id: String,
    session_id: String,
) -> anyhow::Result<()> {
    let db = Codex::connect(resolve_config()?)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    let n = import_orchestrator_memory_dir(&db, &dir, &agent_id, &session_id)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!(
        "Imported {n} markdown file(s) from {} into `memories`.",
        dir.display()
    );
    Ok(())
}

pub async fn import_skill_bundle(file: PathBuf) -> anyhow::Result<()> {
    let db = Codex::connect(resolve_config()?)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    import_skill_bundle_json_file(&db, &file)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    println!("Upserted skill manifest from {}.", file.display());
    Ok(())
}

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
