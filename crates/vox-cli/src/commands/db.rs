//! `vox db` subcommand — inspect and manage the local VoxDB database.

use crate::commands::ci::bounded_read::{read_utf8_path_capped, read_utf8_path_capped_async};
use crate::commands::db_retention;
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Print current VoxDB schema version and connection path.
pub async fn status() -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let version = db.schema_version().await?;
    let data_dir = vox_db::VoxDb::data_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("VoxDB Status");
    println!("  Data directory : {data_dir}");
    println!("  Schema version : v{version}");
    Ok(())
}

fn sqlite_quote_ident(name: &str) -> String {
    let mut s = String::with_capacity(name.len() + 2);
    s.push('"');
    for c in name.chars() {
        if c == '"' {
            s.push_str("\"\"");
        } else {
            s.push(c);
        }
    }
    s.push('"');
    s
}

async fn sqlite_pragma_i64(conn: &turso::Connection, sql: &str) -> Result<i64> {
    let mut rows = conn.query(sql, ()).await?;
    let Some(row) = rows.next().await? else {
        return Ok(0i64);
    };
    Ok(row.get(0)?)
}

async fn sqlite_pragma_text(conn: &turso::Connection, sql: &str) -> Result<String> {
    let mut rows = conn.query(sql, ()).await?;
    let Some(row) = rows.next().await? else {
        return Ok(String::new());
    };
    Ok(row.get(0)?)
}

fn pick_time_audit_column(col_names: &[String]) -> Option<String> {
    const PREFERRED: &[&str] = &[
        "updated_at_ms",
        "created_at_ms",
        "updated_at",
        "created_at",
        "recorded_at_ms",
        "submitted_at_ms",
        "attempted_at_ms",
        "timestamp",
        "ts",
    ];
    for name in PREFERRED {
        if col_names.iter().any(|n| n == name) {
            return Some((*name).to_string());
        }
    }
    for n in col_names {
        let l = n.to_lowercase();
        if l.contains("at_ms") || l.ends_with("_at") || l.contains("timestamp") {
            return Some(n.clone());
        }
    }
    None
}

/// Read-only audit: table row counts + storage PRAGMAs (JSON to stdout).
pub async fn audit(timestamps: bool) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let version = db.schema_version().await?;
    let data_dir = vox_db::VoxDb::data_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "unknown".to_string());
    let db_path = vox_db::paths::default_db_path()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();
    let conn = db.connection();

    let page_count = sqlite_pragma_i64(conn, "PRAGMA page_count").await?;
    let page_size = sqlite_pragma_i64(conn, "PRAGMA page_size").await?;
    let freelist_count = sqlite_pragma_i64(conn, "PRAGMA freelist_count").await?;
    let journal_mode = sqlite_pragma_text(conn, "PRAGMA journal_mode").await?;

    let mut name_rows = conn
        .query(
            "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
            (),
        )
        .await?;
    let mut tables: Vec<serde_json::Value> = Vec::new();
    while let Some(row) = name_rows.next().await? {
        let name: String = row.get(0)?;
        let q = sqlite_quote_ident(&name);
        let sql = format!("SELECT COUNT(*) FROM {q}");
        let mut c = conn
            .query(&sql, ())
            .await
            .with_context(|| format!("count {name}"))?;
        let count: i64 = c
            .next()
            .await?
            .ok_or_else(|| anyhow::anyhow!("missing count for {name}"))?
            .get(0)?;
        let mut entry = serde_json::json!({"name": name, "row_count": count});
        if timestamps && count > 0 {
            let info_sql = format!("PRAGMA table_info({q})");
            let mut info_rows = conn.query(&info_sql, ()).await?;
            let mut col_names = Vec::new();
            while let Some(r) = info_rows.next().await? {
                col_names.push(r.get::<String>(1)?);
            }
            if let Some(tc) = pick_time_audit_column(&col_names) {
                let tq = sqlite_quote_ident(&tc);
                let rng_sql =
                    format!("SELECT MIN({tq}), MAX({tq}) FROM {q} WHERE {tq} IS NOT NULL");
                if let Ok(mut rng) = conn.query(&rng_sql, ()).await {
                    if let Some(rr) = rng.next().await? {
                        let vmin: Option<String> = rr.get(0).ok();
                        let vmax: Option<String> = rr.get(1).ok();
                        entry["time_column"] = serde_json::json!(tc);
                        entry["time_min"] = serde_json::json!(vmin);
                        entry["time_max"] = serde_json::json!(vmax);
                    }
                }
            }
        }
        tables.push(entry);
    }

    let out = serde_json::json!({
        "schema_version": version,
        "data_dir": data_dir,
        "db_path": db_path,
        "pragma": {
            "page_count": page_count,
            "page_size": page_size,
            "freelist_count": freelist_count,
            "journal_mode": journal_mode,
        },
        "table_count": tables.len(),
        "tables": tables,
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

/// Reset the database by dropping all tables and re-applying migrations.
pub async fn reset(file: Option<&PathBuf>) -> Result<()> {
    println!("Resetting database...");
    let path = vox_db::paths::default_db_path()
        .ok_or_else(|| anyhow::anyhow!("Could not resolve default Codex database path"))?;
    let path_str = path.to_string_lossy().into_owned();
    let _db = vox_db::VoxDb::open_local_reset_to_baseline(&path_str)
        .await
        .map_err(|e| anyhow::anyhow!("Reset failed: {e}"))?;
    println!("  Cleared {} and re-applied baseline.", path.display());

    println!("Re-migrating from .vox declarations...");
    migrate(file).await?;
    println!("Reset complete.");
    Ok(())
}

/// Print the current schema digest for LLM context.
pub async fn schema(file: Option<&PathBuf>) -> Result<()> {
    let path = file
        .cloned()
        .unwrap_or_else(|| PathBuf::from("src/main.vox"));

    if !path.exists() {
        anyhow::bail!(
            "No source file found at {}. Run `vox db schema --file <path>` to specify one.",
            path.display()
        );
    }

    let result = crate::pipeline::run_frontend(&path, false)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse source for schema: {}", e))?;

    let digest = vox_db::generate_schema_digest(&result.module, None);
    println!("{}", vox_db::format_llm_context(&digest));

    // Also print JSON for tool consumption
    println!("\n--- JSON DIGEST ---");
    println!("{}", serde_json::to_string_pretty(&digest)?);

    Ok(())
}

/// Print sample data from a table or collection.
pub async fn sample(table: &str, limit: i64) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    println!("Sample data from '{}' (limit {}):", table, limit);

    let conn = &db.connection();

    // Get column names via PRAGMA table_info since Row::column_name is missing.
    let info_sql = format!("PRAGMA table_info({})", table);
    let mut info_rows = conn.query(&info_sql, ()).await?;
    let mut col_names = Vec::new();
    while let Some(row) = info_rows.next().await? {
        col_names.push(row.get::<String>(1)?); // Column 1 is 'name' in PRAGMA table_info
    }

    let sql = format!("SELECT * FROM {} LIMIT {}", table, limit);
    let mut rows = conn.query(&sql, ()).await?;

    let mut count = 0;
    while let Some(row) = rows.next().await? {
        count += 1;
        // Print as JSON for simplicity in CLI
        let mut map = serde_json::Map::new();

        for (i, name) in col_names.iter().enumerate() {
            let val: serde_json::Value = match row.get_value(i) {
                Ok(v) => match v {
                    turso::Value::Null => serde_json::Value::Null,
                    turso::Value::Integer(i) => i.into(),
                    turso::Value::Real(f) => f.into(),
                    turso::Value::Text(s) => s.into(),
                    turso::Value::Blob(b) => format!("(blob {} bytes)", b.len()).into(),
                },
                Err(_) => "error".into(),
            };
            map.insert(name.to_string(), val);
        }
        println!("{}", serde_json::to_string(&map)?);
    }

    if count == 0 {
        println!("(no rows)");
    }

    Ok(())
}

/// Apply any pending schema migrations.
pub async fn migrate(file: Option<&PathBuf>) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let path = file
        .cloned()
        .unwrap_or_else(|| PathBuf::from("src/main.vox"));

    if !path.exists() {
        println!(
            "No source file found at {}. Run `vox db migrate --file <path>` to specify one.",
            path.display()
        );
        return Ok(());
    }

    let result = crate::pipeline::run_frontend(&path, false)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to parse source for migration: {}", e))?;
    let module = &result.module;

    let mut tables = Vec::new();
    let mut collections = Vec::new();
    let mut indexes = Vec::new();

    for decl in &module.declarations {
        match decl {
            vox_compiler::ast::decl::Decl::Table(t) => tables.push(t),
            vox_compiler::ast::decl::Decl::Collection(c) => collections.push(c),
            vox_compiler::ast::decl::Decl::Index(i) => indexes.push(i),
            _ => {}
        }
    }

    let migrator = vox_db::AutoMigrator::new(db.connection());
    let plan = migrator
        .sync_schema(&tables, &collections, &indexes)
        .await?;

    println!("{}", plan.describe());

    Ok(())
}

/// Export memory, patterns, and preferences for a user to JSON.
pub async fn export(user_id: &str, output: Option<&PathBuf>) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;

    let prefs = db.list_user_preferences(user_id, None).await?;
    let memories = db.recall_memory(user_id, None, 200, None).await?;
    let patterns = db.get_learned_patterns(user_id, 200).await?;

    let data = serde_json::json!({
        "user_id": user_id,
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "preferences": prefs.iter().map(|(k, v)| serde_json::json!({"key": k, "value": v})).collect::<Vec<_>>(),
        "memories": memories.iter().map(|m| serde_json::json!({
            "type": m.memory_type,
            "content": m.content,
            "importance": m.importance,
        })).collect::<Vec<_>>(),
        "learned_patterns": patterns.iter().map(|p| serde_json::json!({
            "pattern_type": p.pattern_type,
            "category": p.category,
            "description": p.description,
            "confidence": p.confidence,
        })).collect::<Vec<_>>(),
    });

    let json_str = serde_json::to_string_pretty(&data)?;
    match output {
        Some(path) => {
            tokio::fs::write(path, &json_str).await?;
            println!("Exported to {}", path.display());
        }
        None => println!("{json_str}"),
    }
    Ok(())
}

/// Import preferences and memory from a JSON file previously exported with `vox db export`.
pub async fn import(path: &Path) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let json_str = read_utf8_path_capped_async(path).await?;
    let data: serde_json::Value = serde_json::from_str(&json_str)?;

    let user_id = data["user_id"].as_str().unwrap_or("default");
    let mut pref_count = 0usize;
    let mut mem_count = 0usize;

    if let Some(prefs) = data["preferences"].as_array() {
        for pref in prefs {
            let key = pref["key"].as_str().unwrap_or("");
            let value = pref["value"].as_str().unwrap_or("");
            if !key.is_empty() {
                db.set_user_preference(user_id, key, value).await?;
                pref_count += 1;
            }
        }
    }

    if let Some(mems) = data["memories"].as_array() {
        for m in mems {
            let mtype = m["type"].as_str().unwrap_or("general");
            let content = m["content"].as_str().unwrap_or("");
            let importance = m["importance"].as_f64().unwrap_or(1.0);
            if !content.is_empty() {
                db.save_memory(vox_db::MemoryParams {
                    agent_id: user_id,
                    session_id: "import",
                    memory_type: mtype,
                    content,
                    metadata: None,
                    importance,
                    vcs_snapshot_id: None,
                })
                .await?;
                mem_count += 1;
            }
        }
    }

    println!("Imported from {}", path.display());
    println!("  Preferences : {pref_count}");
    println!("  Memory facts: {mem_count}");
    Ok(())
}

/// Run SQLite VACUUM to reclaim space and defragment the database.
pub async fn vacuum() -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    db.connection()
        .execute("VACUUM", ())
        .await
        .map_err(|e| anyhow::anyhow!("VACUUM failed: {e}"))?;
    println!("VACUUM complete. Database has been compacted.");
    Ok(())
}

/// Delete memory entries older than `days` days for a given agent/user.
pub async fn prune(user_id: &str, days: u32) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let threshold = chrono::Utc::now()
        .checked_sub_signed(chrono::Duration::days(days as i64))
        .unwrap_or(chrono::Utc::now());
    let threshold_str = threshold.format("%Y-%m-%d %H:%M:%S").to_string();
    let deleted = db
        .connection()
        .execute(
            "DELETE FROM memories WHERE agent_id = ?1 AND created_at < ?2",
            turso::params![user_id, threshold_str],
        )
        .await
        .map_err(|e| anyhow::anyhow!("Prune failed: {e}"))?;
    println!("Pruned {deleted} rows from `memories` older than {days} days for '{user_id}'.");
    Ok(())
}

fn retention_cutoff_sql(days: u32) -> String {
    format!("datetime('now', '-{days} day')")
}

/// Emit JSON plan for rows that would be deleted per `contracts/db/retention-policy.yaml`.
pub async fn prune_plan(policy: Option<&Path>) -> Result<()> {
    let path = policy
        .map(PathBuf::from)
        .unwrap_or_else(db_retention::default_policy_path);
    let pol = db_retention::load_policy(&path)?;
    let db = vox_db::VoxDb::connect_default().await?;
    let conn = db.connection();
    let mut rows_out = Vec::new();
    for (table, rule) in pol.tables.iter() {
        if rule.kind != "days" {
            rows_out.push(serde_json::json!({
                "table": table,
                "mode": rule.kind,
                "would_delete": serde_json::Value::Null,
            }));
            continue;
        }
        let (Some(days), Some(col)) = (rule.days, rule.time_column.as_deref()) else {
            anyhow::bail!(
                "retention policy: table `{table}` kind=days requires `days` and `time_column`"
            );
        };
        let tq = db_retention::sqlite_quote_ident(table);
        let cq = db_retention::sqlite_quote_ident(col);
        let cutoff = retention_cutoff_sql(days);
        let sql = format!("SELECT COUNT(*) FROM {tq} WHERE {cq} < {cutoff}");
        let mut r = conn
            .query(&sql, ())
            .await
            .with_context(|| format!("plan {table}"))?;
        let n: i64 = r
            .next()
            .await?
            .ok_or_else(|| anyhow::anyhow!("count {table}"))?
            .get(0)?;
        rows_out.push(serde_json::json!({
            "table": table,
            "mode": "days",
            "days": days,
            "time_column": col,
            "would_delete": n,
        }));
    }
    let out = serde_json::json!({
        "policy": path.display().to_string(),
        "tables": rows_out,
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

/// Execute `days` rules from the retention policy (DELETE).
pub async fn prune_apply(policy: Option<&Path>, i_understand: bool) -> Result<()> {
    if !i_understand {
        anyhow::bail!("refusing prune-apply without `--i-understand` (destructive deletes)");
    }
    let path = policy
        .map(PathBuf::from)
        .unwrap_or_else(db_retention::default_policy_path);
    let pol = db_retention::load_policy(&path)?;
    let db = vox_db::VoxDb::connect_default().await?;
    let conn = db.connection();
    let mut total: u64 = 0;
    for (table, rule) in pol.tables.iter() {
        if rule.kind != "days" {
            continue;
        }
        let (Some(days), Some(col)) = (rule.days, rule.time_column.as_deref()) else {
            anyhow::bail!(
                "retention policy: table `{table}` kind=days requires `days` and `time_column`"
            );
        };
        let tq = db_retention::sqlite_quote_ident(table);
        let cq = db_retention::sqlite_quote_ident(col);
        let cutoff = retention_cutoff_sql(days);
        let sql = format!("DELETE FROM {tq} WHERE {cq} < {cutoff}");
        let n = conn
            .execute(&sql, ())
            .await
            .with_context(|| format!("delete {table}"))?;
        total += n;
    }
    println!(
        "prune-apply: deleted {total} rows total (policy {}).",
        path.display()
    );
    Ok(())
}

/// Get a user preference by key.
pub async fn pref_get(user_id: &str, key: &str) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    match db.get_user_preference(user_id, key).await? {
        Some(val) => println!("{key} = {val}"),
        None => println!("(not set)"),
    }
    Ok(())
}

/// Set a user preference key/value.
pub async fn pref_set(user_id: &str, key: &str, value: &str) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    db.set_user_preference(user_id, key, value).await?;
    println!("Set '{key}' = '{value}' for user '{user_id}'.");
    Ok(())
}

/// List all preferences for a user.
pub async fn pref_list(user_id: &str, prefix: Option<&str>) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let prefs = db.list_user_preferences(user_id, prefix).await?;
    let filtered: Vec<_> = prefs
        .iter()
        .filter(|(k, _)| prefix.map(|p| k.starts_with(p)).unwrap_or(true))
        .collect();
    if filtered.is_empty() {
        println!("(no preferences)");
    } else {
        for (k, v) in &filtered {
            println!("{k} = {v}");
        }
    }
    Ok(())
}

/// Prepare (upsert) a canonical publication manifest from markdown body content.
pub async fn publication_prepare(
    publication_id: &str,
    content_type: &str,
    author: &str,
    title: &str,
    path: &Path,
    abstract_text: Option<&str>,
    citations_json_path: Option<&Path>,
    scholarly_metadata_json_path: Option<&Path>,
    preflight: bool,
    preflight_profile: vox_publisher::publication_preflight::PreflightProfile,
) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let body_markdown = read_utf8_path_capped(path)
        .with_context(|| format!("failed to read markdown body from {}", path.display()))?;
    let citations_json = if let Some(p) = citations_json_path {
        Some(
            read_utf8_path_capped(p)
                .with_context(|| format!("failed to read citations JSON from {}", p.display()))?,
        )
    } else {
        None
    };
    let scientific = if let Some(p) = scholarly_metadata_json_path {
        let raw = read_utf8_path_capped(p).with_context(|| {
            format!(
                "failed to read scholarly metadata JSON from {}",
                p.display()
            )
        })?;
        Some(
            serde_json::from_str::<vox_publisher::scientific_metadata::ScientificPublicationMetadata>(
                raw.trim(),
            )
            .with_context(|| {
                format!(
                    "invalid scholarly metadata JSON (see scientific_publication schema in vox-publisher): {}",
                    p.display()
                )
            })?,
        )
    } else {
        None
    };
    let metadata_json = vox_publisher::scientific_metadata::build_scientia_metadata_json(
        "vox db publication-prepare",
        None,
        scientific.as_ref(),
        None,
    )
    .context("build publication metadata_json")?;
    let manifest = vox_publisher::publication::PublicationManifest {
        publication_id: publication_id.to_string(),
        content_type: content_type.to_string(),
        source_ref: Some(path.display().to_string()),
        title: title.to_string(),
        author: author.to_string(),
        abstract_text: abstract_text.map(std::string::ToString::to_string),
        body_markdown,
        citations_json: citations_json.clone(),
        metadata_json: Some(metadata_json),
    };
    if preflight {
        let report =
            vox_publisher::publication_preflight::run_preflight(&manifest, preflight_profile);
        if !report.ok {
            anyhow::bail!(
                "publication preflight failed (readiness {}):\n{}",
                report.readiness_score,
                serde_json::to_string_pretty(&report)?
            );
        }
    }

    let digest = manifest.content_sha3_256();
    db.upsert_publication_manifest(vox_db::PublicationManifestParams {
        publication_id: &manifest.publication_id,
        content_type: &manifest.content_type,
        source_ref: manifest.source_ref.as_deref(),
        title: &manifest.title,
        author: &manifest.author,
        abstract_text: manifest.abstract_text.as_deref(),
        body_markdown: &manifest.body_markdown,
        citations_json: citations_json.as_deref(),
        metadata_json: manifest.metadata_json.as_deref(),
        content_sha3_256: &digest,
        state: "draft",
    })
    .await?;
    println!(
        "Prepared publication '{}' ({}) digest={}",
        publication_id, content_type, digest
    );
    Ok(())
}

/// Print a JSON preflight report for a manifest already in Codex (no DB writes).
pub async fn publication_preflight(
    publication_id: &str,
    profile: vox_publisher::publication_preflight::PreflightProfile,
    with_worthiness: bool,
) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let Some(row) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    let mut manifest = vox_publisher::publication::PublicationManifest {
        publication_id: row.publication_id.clone(),
        content_type: row.content_type.clone(),
        source_ref: row.source_ref.clone(),
        title: row.title.clone(),
        author: row.author.clone(),
        abstract_text: row.abstract_text.clone(),
        body_markdown: row.body_markdown.clone(),
        citations_json: row.citations_json.clone(),
        metadata_json: row.metadata_json.clone(),
    };
    let report = if with_worthiness {
        let root = vox_repository::resolve_repo_root_for_ci();
        manifest = super::scientia_worthiness_enrich::enrich_manifest_for_worthiness_preflight(
            manifest, &db, &root, None,
        )
        .await?;
        let contract_path =
            root.join(vox_publisher::publication_worthiness::DEFAULT_CONTRACT_REL_PATH);
        let yaml = read_utf8_path_capped(&contract_path).with_context(|| {
            format!(
                "read worthiness contract {} (repo root discovery required)",
                contract_path.display()
            )
        })?;
        let contract = vox_publisher::publication_worthiness::load_contract_from_str(&yaml)?;
        vox_publisher::publication_worthiness::validate_contract_invariants(&contract)?;
        vox_publisher::publication_preflight::run_preflight_with_worthiness(
            &manifest, profile, &contract,
        )
    } else {
        vox_publisher::publication_preflight::run_preflight(&manifest, profile)
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

/// Print Zenodo-oriented deposition metadata JSON (no network).
fn resolve_under_repo(root: &Path, p: &Path) -> PathBuf {
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        root.join(p)
    }
}

/// Print worthiness evaluation JSON using the repo contract + metrics inputs (no DB writes).
pub async fn publication_worthiness_evaluate(
    contract_yaml: Option<&PathBuf>,
    metrics_json: PathBuf,
) -> Result<()> {
    let root = vox_repository::resolve_repo_root_for_ci();
    let contract_path = match contract_yaml {
        Some(p) => resolve_under_repo(&root, p),
        None => root.join(vox_publisher::publication_worthiness::DEFAULT_CONTRACT_REL_PATH),
    };
    let yaml = read_utf8_path_capped(&contract_path)
        .with_context(|| format!("read contract {}", contract_path.display()))?;
    let contract = vox_publisher::publication_worthiness::load_contract_from_str(&yaml)?;
    vox_publisher::publication_worthiness::validate_contract_invariants(&contract)?;
    let metrics_path = resolve_under_repo(&root, &metrics_json);
    let m_src = read_utf8_path_capped(&metrics_path)
        .with_context(|| format!("read metrics {}", metrics_path.display()))?;
    let inputs: vox_publisher::publication_worthiness::WorthinessInputs =
        serde_json::from_str(&m_src).context("parse metrics JSON")?;
    let out = vox_publisher::publication_worthiness::evaluate_worthiness(&contract, &inputs);
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

pub async fn publication_zenodo_metadata(publication_id: &str) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let Some(row) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    let manifest = vox_publisher::publication::PublicationManifest {
        publication_id: row.publication_id.clone(),
        content_type: row.content_type.clone(),
        source_ref: row.source_ref.clone(),
        title: row.title.clone(),
        author: row.author.clone(),
        abstract_text: row.abstract_text.clone(),
        body_markdown: row.body_markdown.clone(),
        citations_json: row.citations_json.clone(),
        metadata_json: row.metadata_json.clone(),
    };
    let z = vox_publisher::zenodo_metadata::zenodo_deposition_metadata(&manifest);
    println!("{}", serde_json::to_string_pretty(&z)?);
    Ok(())
}

/// Record one digest-bound publication approval.
pub async fn publication_approve(publication_id: &str, approver: &str) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let Some(manifest) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    let approver = approver.trim();
    if approver.is_empty() {
        anyhow::bail!("approver must not be empty");
    }
    db.record_publication_approval_for_digest(publication_id, &manifest.content_sha3_256, approver)
        .await?;
    let count = db
        .count_publication_approvers_for_digest(publication_id, &manifest.content_sha3_256)
        .await?;
    if count >= 2 {
        db.set_publication_state(publication_id, "approved", None)
            .await?;
    }
    println!(
        "Recorded approval for '{}' digest={} distinct_approvers={}",
        publication_id, manifest.content_sha3_256, count
    );
    Ok(())
}

/// Submit to the first scholarly adapter integration (`local_ledger`).
pub async fn publication_submit_local(publication_id: &str) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let Some(row) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    let dual = db
        .has_dual_publication_approval_for_digest(publication_id, &row.content_sha3_256)
        .await?;
    if !dual {
        anyhow::bail!("publication requires two distinct digest-bound approvers before submission");
    }
    let manifest = vox_publisher::publication::PublicationManifest {
        publication_id: row.publication_id.clone(),
        content_type: row.content_type.clone(),
        source_ref: row.source_ref.clone(),
        title: row.title.clone(),
        author: row.author.clone(),
        abstract_text: row.abstract_text.clone(),
        body_markdown: row.body_markdown.clone(),
        citations_json: row.citations_json.clone(),
        metadata_json: row.metadata_json.clone(),
    };
    let receipt = vox_publisher::scholarly::submit_with_configured_adapter(&manifest)?;
    db.upsert_scholarly_submission(
        publication_id,
        &row.content_sha3_256,
        &receipt.adapter,
        &receipt.external_submission_id,
        &receipt.status,
        receipt.response_fingerprint.as_deref(),
        receipt.metadata_json.as_deref(),
    )
    .await?;
    println!(
        "Submitted '{}' via {} as {} ({})",
        publication_id, receipt.adapter, receipt.external_submission_id, receipt.status
    );
    Ok(())
}

/// Show publication state and scholarly submission rows.
pub async fn publication_status(publication_id: &str) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let Some(row) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    let approvals = db
        .count_publication_approvers_for_digest(publication_id, &row.content_sha3_256)
        .await?;
    let submissions = db.list_scholarly_submissions(publication_id).await?;
    let media_assets = db.list_publication_media_assets(publication_id).await?;
    let attempts = db.list_publication_attempts(publication_id).await?;
    let status_events = db.list_publication_status_events(publication_id).await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "publication_id": row.publication_id,
            "content_type": row.content_type,
            "state": row.state,
            "digest": row.content_sha3_256,
            "version": row.version,
            "approvals_for_digest": approvals,
            "scholarly_submissions": submissions,
            "media_assets": media_assets,
            "publication_attempts": attempts,
            "publication_status_events": status_events,
        }))?
    );
    Ok(())
}

/// Upsert one publication media asset row.
pub async fn publication_media_upsert(
    publication_id: &str,
    asset_ref: &str,
    media_type: &str,
    storage_uri: Option<&str>,
    status: &str,
    metadata_json_path: Option<&PathBuf>,
) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let metadata_json = if let Some(path) = metadata_json_path {
        Some(
            read_utf8_path_capped(path)
                .with_context(|| format!("failed to read metadata JSON from {}", path.display()))?,
        )
    } else {
        None
    };
    db.upsert_publication_media_asset(vox_db::PublicationMediaAssetParams {
        publication_id,
        asset_ref,
        media_type,
        storage_uri,
        status,
        metadata_json: metadata_json.as_deref(),
    })
    .await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "publication_id": publication_id,
            "asset_ref": asset_ref,
            "media_type": media_type,
            "storage_uri": storage_uri,
            "status": status,
            "metadata_json_present": metadata_json.is_some()
        }))?
    );
    Ok(())
}

/// List publication media assets for one publication id.
pub async fn publication_media_list(publication_id: &str) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let rows = db.list_publication_media_assets(publication_id).await?;
    println!("{}", serde_json::to_string_pretty(&rows)?);
    Ok(())
}

/// Delete one publication media asset by `publication_id + asset_ref`.
pub async fn publication_media_delete(publication_id: &str, asset_ref: &str) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    db.delete_publication_media_asset(publication_id, asset_ref)
        .await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "deleted": true,
            "publication_id": publication_id,
            "asset_ref": asset_ref
        }))?
    );
    Ok(())
}

fn publication_item_from_manifest(
    row: &vox_db::PublicationManifestRow,
) -> Result<vox_publisher::types::UnifiedNewsItem> {
    vox_publisher::switching::unified_news_item_from_manifest_parts(
        &row.publication_id,
        &row.title,
        &row.author,
        &row.body_markdown,
        row.metadata_json.as_deref(),
    )
}

fn publication_manifest_from_row(row: &vox_db::PublicationManifestRow) -> vox_publisher::publication::PublicationManifest {
    vox_publisher::publication::PublicationManifest {
        publication_id: row.publication_id.clone(),
        content_type: row.content_type.clone(),
        source_ref: row.source_ref.clone(),
        title: row.title.clone(),
        author: row.author.clone(),
        abstract_text: row.abstract_text.clone(),
        body_markdown: row.body_markdown.clone(),
        citations_json: row.citations_json.clone(),
        metadata_json: row.metadata_json.clone(),
    }
}

fn cli_social_worthiness_enforce() -> bool {
    std::env::var("VOX_SOCIAL_WORTHINESS_ENFORCE")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn cli_social_worthiness_score_min() -> f64 {
    std::env::var("VOX_SOCIAL_WORTHINESS_SCORE_MIN")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.85)
}

fn publisher_config_from_env(
    dry_run: bool,
    worthiness_score: Option<f64>,
) -> vox_publisher::PublisherConfig {
    let mut cfg = vox_publisher::PublisherConfig::from_operator_environment(
        dry_run,
        Some(vox_repository::resolve_repo_root_for_ci()),
        vox_publisher::NewsSiteConfig::from_default_with_operator_env(),
    );
    cfg.worthiness_score = worthiness_score;
    cfg
}

/// Simulate per-channel routing/policy outcomes using an existing DB handle (tests and in-process callers).
pub async fn publication_route_simulate_with_db(
    db: &vox_db::VoxDb,
    publication_id: &str,
) -> Result<vox_publisher::SyndicationResult> {
    let Some(row) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    let item = publication_item_from_manifest(&row)?;
    let manifest = publication_manifest_from_row(&row);
    let root = vox_repository::resolve_repo_root_for_ci();
    let worthiness =
        vox_publisher::publication_worthiness::worthiness_score_for_publication_manifest(
            &manifest, &root,
        )
        .ok();
    let publisher = vox_publisher::Publisher::new(publisher_config_from_env(true, worthiness));
    publisher.publish_all(&item).await
}

/// Simulate per-channel routing/policy outcomes for one prepared publication id.
///
/// When `json` is true, prints one line of compact JSON (stable key order from `serde_json`).
pub async fn publication_route_simulate(publication_id: &str, json: bool) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let result = publication_route_simulate_with_db(&db, publication_id).await?;
    if json {
        println!("{}", serde_json::to_string(&result)?);
    } else {
        println!("{}", serde_json::to_string_pretty(&result)?);
    }
    Ok(())
}

/// Publish one prepared publication to selected channels (default: all configured channels).
pub async fn publication_publish(
    publication_id: &str,
    channels_csv: Option<&str>,
    dry_run: bool,
    json: bool,
) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let Some(row) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    let allowed = channels_csv
        .map(vox_publisher::switching::parse_channels_csv)
        .filter(|v| !v.is_empty());
    let mut item = publication_item_from_manifest(&row)?;
    if let Some(allowlist) = allowed.as_deref() {
        vox_publisher::switching::apply_channel_allowlist(&mut item, allowlist);
    }
    let digest = row.content_sha3_256.as_str();
    let dual = db
        .has_dual_publication_approval_for_digest(publication_id, digest)
        .await?;
    let gate = vox_publisher::gate::evaluate_publish_gate(
        vox_publisher::gate::publish_gate_inputs_for_cli(dry_run, true, dual, &item),
    );
    if gate.has_blockers() {
        let detail = serde_json::json!({ "blocking_reasons": gate.blocking_reasons });
        anyhow::bail!(
            "live publish blocked by gate: {}",
            serde_json::to_string(&detail)?
        );
    }
    let manifest = publication_manifest_from_row(&row);
    let root = vox_repository::resolve_repo_root_for_ci();
    let worthiness =
        vox_publisher::publication_worthiness::worthiness_score_for_publication_manifest(
            &manifest, &root,
        )
        .ok();
    if cli_social_worthiness_enforce()
        && !dry_run
        && !item.syndication.dry_run
        && gate.live_publish_allowed
        && let Some(score) = worthiness
    {
        let floor = cli_social_worthiness_score_min();
        if score < floor {
            let detail = serde_json::json!({
                "error": "live publish blocked by worthiness floor",
                "worthiness_score": score,
                "floor": floor,
            });
            anyhow::bail!(
                "live publish blocked by worthiness: {}",
                serde_json::to_string(&detail)?
            );
        }
    }
    let publisher = vox_publisher::Publisher::new(publisher_config_from_env(dry_run, worthiness));
    let result = publisher.publish_all(&item).await?;
    let result_json = serde_json::to_string(&result)?;
    db.record_publication_attempt(publication_id, digest, "manual_cli", &result_json)
        .await?;
    if gate.live_publish_allowed {
        if result.all_enabled_channels_succeeded(&item) {
            let _ = db
                .set_publication_state(
                    publication_id,
                    "published",
                    Some(
                        &serde_json::json!({ "channel_group": "manual_cli" }).to_string(),
                    ),
                )
                .await;
        } else if result.has_failures() {
            let _ = db
                .set_publication_state(
                    publication_id,
                    "publish_failed",
                    Some(
                        &serde_json::json!({ "channel_group": "manual_cli" }).to_string(),
                    ),
                )
                .await;
        }
    }
    if json {
        println!("{}", result_json);
    } else {
        println!("{}", serde_json::to_string_pretty(&result)?);
    }
    Ok(())
}

/// Retry failed channels from the latest publication attempt.
pub async fn publication_retry_failed(
    publication_id: &str,
    channel: Option<&str>,
    dry_run: bool,
    json: bool,
) -> Result<()> {
    if let Some(ch) = channel {
        return publication_publish(publication_id, Some(ch), dry_run, json).await;
    }
    let db = vox_db::VoxDb::connect_default().await?;
    let Some(row) = db.get_publication_manifest(publication_id).await? else {
        anyhow::bail!("publication not found: {publication_id}");
    };
    let digest = row.content_sha3_256.as_str();
    let attempts = db.list_publication_attempts(publication_id).await?;
    let attempt_refs: Vec<vox_publisher::switching::AttemptOutcome<'_>> = attempts
        .iter()
        .map(|a| vox_publisher::switching::AttemptOutcome {
            content_sha3_256: a.content_sha3_256.as_str(),
            outcome_json: a.outcome_json.as_str(),
        })
        .collect();
    let Some(failed) = vox_publisher::switching::failed_channels_from_latest_digest_attempt(
        attempt_refs.as_slice(),
        digest,
    )?
    else {
        anyhow::bail!(
            "no syndication attempt outcome for current manifest digest; run `vox db publication-publish` first"
        );
    };
    if failed.is_empty() {
        println!(
            "{}",
            serde_json::to_string_pretty(
                &serde_json::json!({"publication_id": publication_id, "retried": false, "reason": "no_failed_channels"})
            )?
        );
        return Ok(());
    }
    let csv = failed.join(",");
    publication_publish(publication_id, Some(csv.as_str()), dry_run, json).await
}

pub use super::db_research::*;

#[cfg(test)]
mod tests {
    use super::publication_item_from_manifest;
    use chrono::Utc;
    use vox_publisher::types::{SyndicationConfig, TwitterConfig, UnifiedNewsItem};

    fn sample_item() -> UnifiedNewsItem {
        UnifiedNewsItem {
            id: "x".to_string(),
            title: "t".to_string(),
            author: "a".to_string(),
            published_at: Utc::now(),
            tags: vec![],
            content_markdown: "body".to_string(),
            syndication: SyndicationConfig {
                twitter: Some(TwitterConfig {
                    short_text: Some("s".to_string()),
                    thread: false,
                }),
                rss: true,
                ..Default::default()
            },
            topic_pack: None,
        }
    }

    #[test]
    fn parse_channels_csv_normalizes() {
        let out = Some(vox_publisher::switching::parse_channels_csv(
            " twitter, reddit ,YOUTUBE ",
        ));
        assert_eq!(
            out,
            Some(vec![
                "twitter".to_string(),
                "reddit".to_string(),
                "youtube".to_string()
            ])
        );
    }

    #[test]
    fn filter_channels_keeps_only_allowed() {
        let item = sample_item();
        let allowed = vec!["twitter".to_string()];
        let mut out = item;
        vox_publisher::switching::apply_channel_allowlist(&mut out, allowed.as_slice());
        assert!(!out.syndication.rss);
        assert!(out.syndication.twitter.is_some());
    }

    #[test]
    fn publication_item_from_manifest_hydrates_topic_pack() {
        let row = vox_db::PublicationManifestRow {
            publication_id: "p1".to_string(),
            content_type: "scientia".to_string(),
            source_ref: None,
            title: "Title".to_string(),
            author: "Author".to_string(),
            abstract_text: None,
            body_markdown: "Body".to_string(),
            citations_json: None,
            metadata_json: Some(
                r#"{
                    "tags":["research_breakthrough"],
                    "topic_pack":"research_breakthrough",
                    "syndication":{"twitter":{"short_text":null,"thread":false},"rss":true}
                }"#
                .to_string(),
            ),
            content_sha3_256: "digest".to_string(),
            state: "draft".to_string(),
            version: 1,
            created_at_ms: 0,
            updated_at_ms: 0,
        };
        let item = publication_item_from_manifest(&row).expect("item");
        assert_eq!(item.topic_pack.as_deref(), Some("research_breakthrough"));
        assert!(item.syndication.twitter.is_none());
    }
}
