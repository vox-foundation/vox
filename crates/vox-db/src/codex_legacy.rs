// Legacy database import/export planning for **Codex** greenfield releases.
//
// Baseline **V1** stores a single `schema_version` row; pre-baseline databases that ran the
// historical multi-step chain must be exported with `export_legacy_jsonl` (using a connection that
// skips baseline migration) and imported into a fresh file. See `docs/src/architecture/codex-vnext-schema.md` and ADR 004.

use std::io::{BufRead, Write};
use turso::Value as SqlValue;
use turso::params;

use crate::schema::CODEX_REACTIVITY_TABLES;

use crate::StoreError;

/// Result of [`verify_legacy_store`].
#[derive(Debug, Clone)]
pub struct LegacyVerification {
    /// Highest `schema_version.version` from the store (baseline Codex uses **1** only).
    pub schema_version: i64,
    /// Whether all [`CODEX_REACTIVITY_TABLES`] exist (manifest-derived; not a numeric threshold).
    pub has_codex_reactivity: bool,
    /// `true` when this database still has the pre-baseline multi-version chain (`MAX(version) > 1`).
    pub is_legacy_schema_chain: bool,
}

async fn table_exists(store: &crate::VoxDb, name: &str) -> Result<bool, StoreError> {
    let mut rows: turso::Rows = store
        .connection()
        .query(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1 LIMIT 1",
            params![name],
        )
        .await?;
    Ok(rows.next().await?.is_some())
}

/// Inspect a connected store for legacy import / migration workflows.
pub async fn verify_legacy_store(store: &crate::VoxDb) -> Result<LegacyVerification, StoreError> {
    let schema_version = store.schema_version().await?;
    let mut has_codex_reactivity = true;
    for t in CODEX_REACTIVITY_TABLES {
        if !table_exists(store, t).await? {
            has_codex_reactivity = false;
            break;
        }
    }
    Ok(LegacyVerification {
        schema_version,
        has_codex_reactivity,
        is_legacy_schema_chain: schema_version > crate::schema::BASELINE_VERSION,
    })
}

/// Planned CLI surface (see `vox codex` in `vox-cli`).
pub const PLANNED_CODEX_CLI: &[&str] = &[
    "vox codex export-legacy",
    "vox codex import-legacy",
    "vox codex verify",
];

/// Documented import sources for future implementers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyImportSource {
    /// Current `vox-pm` / `VoxDb` dump.
    VoxPmSqliteTurso,
    /// Orchestrator file-first memory (`memory/*.md`, `MEMORY.md`).
    OrchestratorFileMemory,
    /// Published skill bundles on disk or HTTP.
    SkillBundles,
}

impl LegacyImportSource {
    /// Human-readable label for reports.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::VoxPmSqliteTurso => "vox_pm_turso",
            Self::OrchestratorFileMemory => "orchestrator_file_memory",
            Self::SkillBundles => "skill_bundles",
        }
    }
}

/// Tables exported by [`export_legacy_jsonl`] / restored by [`import_legacy_jsonl`].
pub const LEGACY_EXPORT_TABLES: &[&str] = &[
    "objects",
    "names",
    "metadata",
    "packages",
    "package_deps",
    "execution_log",
    "scheduled",
    "components",
    "users",
    "user_preferences",
    "conversations",
    "conversation_messages",
    "conversation_tool_calls",
    "topics",
    "conversation_topics",
    "conversation_message_topics",
    "usage_limit_definitions",
    "usage_counter_snapshots",
    "search_documents",
    "search_document_chunks",
    "search_indexing_jobs",
    "processing_runs",
    "processing_run_steps",
    "audit_log",
    "research_sessions",
    "conversation_versions",
    "conversation_edges",
    "topic_evolution_events",
    "memories",
    "knowledge_nodes",
    "knowledge_edges",
    "embeddings",
    "skill_manifests",
    "snippets",
    "artifacts",
    "agents",
    "codex_change_log",
    "codex_subscriptions",
    "codex_schema_lineage",
    "codex_query_snapshots",
    "codex_projection_versions",
];

fn sql_value_to_json(v: SqlValue) -> serde_json::Value {
    match v {
        SqlValue::Null => serde_json::Value::Null,
        SqlValue::Integer(i) => serde_json::json!(i),
        SqlValue::Real(f) => serde_json::json!(f),
        SqlValue::Text(s) => serde_json::Value::String(s),
        SqlValue::Blob(b) => {
            serde_json::Value::String(format!("base64:{}", data_encoding::BASE64.encode(&b)))
        }
    }
}

/// Stream one JSON object per line: `{"table":"…","columns":[…],"row":{…}}`.
pub async fn export_legacy_jsonl<W: Write>(
    store: &crate::VoxDb,
    writer: &mut W,
) -> Result<u64, StoreError> {
    let mut count = 0u64;
    for table in LEGACY_EXPORT_TABLES {
        let pragma = format!("PRAGMA table_info({table})");
        let mut cols_rows: turso::Rows = store.connection().query(&pragma, ()).await?;
        let mut columns = Vec::new();
        while let Some(r) = cols_rows.next().await? {
            let name: String = r.get(1)?;
            columns.push(name);
        }
        if columns.is_empty() {
            continue;
        }
        let select = format!("SELECT {} FROM {}", columns.join(", "), table);
        let mut rows: turso::Rows = match store.connection().query(&select, ()).await {
            Ok(r) => r,
            Err(_) => continue,
        };
        while let Some(row) = rows.next().await? {
            let mut obj = serde_json::Map::new();
            for i in 0..columns.len() {
                let v = row.get_value(i)?;
                obj.insert(columns[i].clone(), sql_value_to_json(v));
            }
            let line = serde_json::json!({
                "table": table,
                "columns": columns,
                "row": serde_json::Value::Object(obj),
            });
            writeln!(
                writer,
                "{}",
                serde_json::to_string(&line)
                    .map_err(|e| StoreError::Serialization(e.to_string()))?
            )
            .map_err(|e| StoreError::Db(format!("legacy export write: {e}")))?;
            count += 1;
        }
    }
    Ok(count)
}

/// Apply JSONL from [`export_legacy_jsonl`]. Uses `INSERT OR REPLACE` with bound parameters.
pub async fn import_legacy_jsonl<R: BufRead>(
    store: &crate::VoxDb,
    reader: R,
) -> Result<u64, StoreError> {
    let mut applied = 0u64;
    for line in reader.lines() {
        let line = line.map_err(|e| StoreError::Db(format!("legacy import read: {e}")))?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let v: serde_json::Value =
            serde_json::from_str(trimmed).map_err(|e| StoreError::Serialization(e.to_string()))?;
        let table = v
            .get("table")
            .and_then(|x| x.as_str())
            .ok_or_else(|| StoreError::Db("legacy import line missing table".into()))?;
        let row = v
            .get("row")
            .and_then(|x| x.as_object())
            .ok_or_else(|| StoreError::Db("legacy import line missing row object".into()))?;
        if !LEGACY_EXPORT_TABLES.contains(&table) {
            continue;
        }
        let columns: Vec<String> = row.keys().cloned().collect();
        if columns.is_empty() {
            continue;
        }
        let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("?{i}")).collect();
        let sql = format!(
            "INSERT OR REPLACE INTO {} ({}) VALUES ({})",
            table,
            columns.join(", "),
            placeholders.join(", ")
        );
        let mut values: Vec<SqlValue> = Vec::with_capacity(columns.len());
        for c in &columns {
            let cell = row.get(c).unwrap_or(&serde_json::Value::Null);
            values.push(json_to_sql_value(cell)?);
        }
        store.connection().execute(&sql, values).await?;
        applied += 1;
    }
    Ok(applied)
}

fn json_to_sql_value(v: &serde_json::Value) -> Result<SqlValue, StoreError> {
    Ok(match v {
        serde_json::Value::Null => SqlValue::Null,
        serde_json::Value::Bool(b) => {
            if *b {
                SqlValue::Integer(1)
            } else {
                SqlValue::Integer(0)
            }
        }
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                SqlValue::Integer(i)
            } else if let Some(f) = n.as_f64() {
                SqlValue::Real(f)
            } else {
                SqlValue::Text(n.to_string())
            }
        }
        serde_json::Value::String(s) => {
            if let Some(rest) = s.strip_prefix("base64:") {
                let bytes = data_encoding::BASE64
                    .decode(rest.as_bytes())
                    .map_err(|e| StoreError::Serialization(e.to_string()))?;
                SqlValue::Blob(bytes)
            } else {
                SqlValue::Text(s.clone())
            }
        }
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => SqlValue::Text(
            serde_json::to_string(v).map_err(|e| StoreError::Serialization(e.to_string()))?,
        ),
    })
}
