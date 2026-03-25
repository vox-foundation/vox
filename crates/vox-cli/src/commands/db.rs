//! `vox db` subcommand — inspect and manage the local VoxDB database.

use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

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

/// Reset the database by dropping all tables and re-applying migrations.
pub async fn reset(file: Option<&PathBuf>) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    println!("Resetting database...");

    // Get list of tables to drop (excluding internal ones)
    let mut rows = db.connection().query("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE 'vox_%'", ()).await?;
    let mut tables_to_drop = Vec::new();
    while let Some(row) = rows.next().await? {
        tables_to_drop.push(row.get::<String>(0)?);
    }

    for table in tables_to_drop {
        println!("  Dropping table: {}", table);
        db.connection()
            .execute(&format!("DROP TABLE IF EXISTS {}", table), ())
            .await?;
    }

    println!("Database cleared. Re-migrating...");
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

    let migrator = vox_db::AutoMigrator::new(&db.connection());
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
            std::fs::write(path, &json_str)?;
            println!("Exported to {}", path.display());
        }
        None => println!("{json_str}"),
    }
    Ok(())
}

/// Import preferences and memory from a JSON file previously exported with `vox db export`.
pub async fn import(path: &PathBuf) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let json_str = std::fs::read_to_string(path)?;
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
            "DELETE FROM agent_memory WHERE agent_id = ?1 AND created_at < ?2",
            turso::params![user_id, threshold_str],
        )
        .await
        .map_err(|e| anyhow::anyhow!("Prune failed: {e}"))?;
    println!("Pruned {deleted} memory entries older than {days} days for '{user_id}'.");
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
    path: &PathBuf,
    abstract_text: Option<&str>,
    citations_json_path: Option<&PathBuf>,
) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let body_markdown = fs::read_to_string(path)
        .with_context(|| format!("failed to read markdown body from {}", path.display()))?;
    let citations_json = if let Some(p) = citations_json_path {
        Some(
            fs::read_to_string(p)
                .with_context(|| format!("failed to read citations JSON from {}", p.display()))?,
        )
    } else {
        None
    };
    let manifest = vox_publisher::publication::PublicationManifest {
        publication_id: publication_id.to_string(),
        content_type: content_type.to_string(),
        source_ref: Some(path.display().to_string()),
        title: title.to_string(),
        author: author.to_string(),
        abstract_text: abstract_text.map(std::string::ToString::to_string),
        body_markdown,
        citations_json: citations_json.clone(),
        metadata_json: Some(
            serde_json::json!({
                "prepared_by": "vox db publication-prepare",
            })
            .to_string(),
        ),
    };
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
        db.set_publication_state(publication_id, "approved", None).await?;
    }
    println!(
        "Recorded approval for '{}' digest={} distinct_approvers={}",
        publication_id, manifest.content_sha3_256, count
    );
    Ok(())
}

/// Submit to the first scholarly adapter integration (`local_ledger`).
pub async fn publication_submit_local(publication_id: &str) -> Result<()> {
    use vox_publisher::scholarly::ScholarlyAdapter;

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
    let adapter = vox_publisher::scholarly::LocalLedgerAdapter;
    let receipt = adapter.submit(&manifest)?;
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
        }))?
    );
    Ok(())
}

include!("db_research_impl.rs");
