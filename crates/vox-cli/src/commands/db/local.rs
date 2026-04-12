//! Local VoxDB inspection, maintenance, and preferences.

use crate::commands::ci::bounded_read::read_utf8_path_capped_async;
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
    db.run_sqlite_vacuum()
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
        .delete_memories_created_before(user_id, threshold_str.as_str())
        .await
        .map_err(|e| anyhow::anyhow!("Prune failed: {e}"))?;
    println!("Pruned {deleted} rows from `memories` older than {days} days for '{user_id}'.");
    Ok(())
}

/// Emit JSON plan for rows that would be deleted per `contracts/db/retention-policy.yaml`.
pub async fn prune_plan(policy: Option<&Path>) -> Result<()> {
    let path = policy
        .map(PathBuf::from)
        .unwrap_or_else(db_retention::default_policy_path);
    let pol = db_retention::load_policy(&path)?;
    let db = vox_db::VoxDb::connect_default().await?;
    let mut rows_out = Vec::new();
    for (table, rule) in pol.tables.iter() {
        if rule.kind == "days" {
            let (Some(days), Some(col)) = (rule.days, rule.time_column.as_deref()) else {
                anyhow::bail!(
                    "retention policy: table `{table}` kind=days requires `days` and `time_column`"
                );
            };
            let n = db
                .retention_count_older_than_days(table, col, days)
                .await
                .with_context(|| format!("plan {table}"))?;
            rows_out.push(serde_json::json!({
                "table": table,
                "mode": "days",
                "days": days,
                "time_column": col,
                "would_delete": n,
            }));
            continue;
        }
        if rule.kind == "ms_days" {
            let (Some(days), Some(col)) = (rule.days, rule.time_column.as_deref()) else {
                anyhow::bail!(
                    "retention policy: table `{table}` kind=ms_days requires `days` and `time_column`"
                );
            };
            let cutoff = vox_db::VoxDb::retention_cutoff_ms_exclusive_for_days(days);
            let n = db
                .retention_count_older_than_ms_cutoff(table, col, cutoff)
                .await
                .with_context(|| format!("plan {table}"))?;
            rows_out.push(serde_json::json!({
                "table": table,
                "mode": "ms_days",
                "days": days,
                "time_column": col,
                "would_delete": n,
            }));
            continue;
        }
        if rule.kind == "expires_lt_now" {
            let Some(col) = rule.time_column.as_deref() else {
                anyhow::bail!(
                    "retention policy: table `{table}` kind=expires_lt_now requires `time_column`"
                );
            };
            let n = db
                .retention_count_expires_lt_now(table, col)
                .await
                .with_context(|| format!("plan {table}"))?;
            rows_out.push(serde_json::json!({
                "table": table,
                "mode": "expires_lt_now",
                "time_column": col,
                "would_delete": n,
            }));
            continue;
        }
        rows_out.push(serde_json::json!({
            "table": table,
            "mode": rule.kind,
            "would_delete": serde_json::Value::Null,
        }));
    }
    let out = serde_json::json!({
        "policy": path.display().to_string(),
        "tables": rows_out,
    });
    println!("{}", serde_json::to_string_pretty(&out)?);
    Ok(())
}

/// Execute `days` and `ms_days` rules from the retention policy (DELETE).
pub async fn prune_apply(policy: Option<&Path>, i_understand: bool) -> Result<()> {
    if !i_understand {
        anyhow::bail!("refusing prune-apply without `--i-understand` (destructive deletes)");
    }
    let path = policy
        .map(PathBuf::from)
        .unwrap_or_else(db_retention::default_policy_path);
    let pol = db_retention::load_policy(&path)?;
    let db = vox_db::VoxDb::connect_default().await?;
    let mut total: u64 = 0;
    for (table, rule) in pol.tables.iter() {
        let n = if rule.kind == "days" {
            let (Some(days), Some(col)) = (rule.days, rule.time_column.as_deref()) else {
                anyhow::bail!(
                    "retention policy: table `{table}` kind=days requires `days` and `time_column`"
                );
            };
            db.retention_delete_older_than_days(table, col, days)
                .await
                .with_context(|| format!("delete {table}"))?
        } else if rule.kind == "ms_days" {
            let (Some(days), Some(col)) = (rule.days, rule.time_column.as_deref()) else {
                anyhow::bail!(
                    "retention policy: table `{table}` kind=ms_days requires `days` and `time_column`"
                );
            };
            db.retention_delete_all_ms_older_than_days(table, col, days)
                .await
                .with_context(|| format!("delete {table}"))?
        } else if rule.kind == "expires_lt_now" {
            let Some(col) = rule.time_column.as_deref() else {
                anyhow::bail!(
                    "retention policy: table `{table}` kind=expires_lt_now requires `time_column`"
                );
            };
            db.retention_delete_expires_lt_now(table, col)
                .await
                .with_context(|| format!("delete {table}"))?
        } else {
            continue;
        };
        total += n;
    }
    // Also prune Tavily search documents with fixed 7-day TTL (Wave 1 operational hardening).
    let tavily_pruned = db.retention_prune_tavily_search_documents().await.unwrap_or(0);

    println!(
        "prune-apply: deleted {total} rows total (policy {}), plus {tavily_pruned} stale Tavily search documents.",
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

/// Query and display execution history.
pub async fn exec_history(
    tool_key: Option<&str>,
    repo: Option<&str>,
    limit: i64,
    json: bool,
) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let history = db.query_historical_exec_time(tool_key, repo, limit).await?;

    if json {
        println!("{}", serde_json::to_string_pretty(&history)?);
    } else {
        if history.is_empty() {
            println!("No execution history found.");
            return Ok(());
        }
        for item in history {
            println!(
                "[{}] tool='{}' repo='{}' result={} dur={}ms expected={:?}",
                item["recorded_at"],
                item["tool_key"].as_str().unwrap_or(""),
                item["repository_id"].as_str().unwrap_or(""),
                item["outcome"].as_str().unwrap_or(""),
                item["duration_ms"],
                item["timeout_budget_ms"],
            );
        }
    }
    Ok(())
}
