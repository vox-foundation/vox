//! Optional importers for sources described in [`crate::codex_legacy::LegacyImportSource`].
//!
//! These are separate from JSONL legacy round-trip; callers run them against an already-open
//! baseline [`crate::VoxDb`].

use std::path::Path;

use turso::params;

use crate::StoreError;

/// Ingest markdown files from `dir` (non-recursive, `*.md` only) into `memories`.
///
/// Each file becomes one row with `memory_type` `orchestrator_markdown` and `metadata` JSON
/// holding the relative file name.
pub async fn import_orchestrator_memory_dir(
    store: &crate::VoxDb,
    dir: &Path,
    agent_id: &str,
    session_id: &str,
) -> Result<u64, StoreError> {
    if !dir.is_dir() {
        return Err(StoreError::Db(format!(
            "not a directory: {}",
            dir.display()
        )));
    }
    let mut inserted = 0u64;
    let mut read_dir = tokio::fs::read_dir(dir)
        .await
        .map_err(|e| StoreError::Db(format!("read_dir {}: {e}", dir.display())))?;
    while let Some(ent) = read_dir
        .next_entry()
        .await
        .map_err(|e| StoreError::Db(format!("next_entry: {e}")))?
    {
        let path = ent.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let name = path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| StoreError::Db(format!("read {}: {e}", path.display())))?;
        let meta = serde_json::json!({ "source_file": name, "source": "orchestrator_memory_dir" })
            .to_string();
        store
            .connection()
            .execute(
                "INSERT INTO memories (agent_id, session_id, memory_type, content, metadata, importance)
                 VALUES (?1, ?2, ?3, ?4, ?5, 1.0)",
                params![
                    agent_id,
                    session_id,
                    "orchestrator_markdown",
                    content,
                    meta,
                ],
            )
            .await
            .map_err(|e| StoreError::Db(format!("insert memory {name}: {e}")))?;
        inserted += 1;
    }
    Ok(inserted)
}

/// Upsert one skill manifest from a small JSON descriptor file.
///
/// Expected shape: `{ "id", "version", "manifest_json", "skill_md" }` (all strings).
pub async fn import_skill_bundle_json_file(
    store: &crate::VoxDb,
    path: &Path,
) -> Result<(), StoreError> {
    let raw = tokio::fs::read_to_string(path)
        .await
        .map_err(|e| StoreError::Db(format!("read {}: {e}", path.display())))?;
    let v: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| StoreError::Serialization(format!("skill bundle json: {e}")))?;
    let id = v
        .get("id")
        .and_then(|x| x.as_str())
        .ok_or_else(|| StoreError::Db("skill bundle: missing id".into()))?;
    let version = v
        .get("version")
        .and_then(|x| x.as_str())
        .ok_or_else(|| StoreError::Db("skill bundle: missing version".into()))?;
    let manifest_json = v
        .get("manifest_json")
        .and_then(|x| x.as_str())
        .ok_or_else(|| StoreError::Db("skill bundle: missing manifest_json".into()))?;
    let skill_md = v
        .get("skill_md")
        .and_then(|x| x.as_str())
        .ok_or_else(|| StoreError::Db("skill bundle: missing skill_md".into()))?;
    let c = store.connection();
    c.execute(
        "DELETE FROM skill_manifests WHERE id = ?1 AND version = ?2",
        params![id, version],
    )
    .await
    .map_err(|e| StoreError::Db(format!("skill_manifests delete: {e}")))?;
    c.execute(
        "INSERT INTO skill_manifests (id, version, manifest_json, skill_md) VALUES (?1, ?2, ?3, ?4)",
        params![id, version, manifest_json, skill_md],
    )
    .await
    .map_err(|e| StoreError::Db(format!("skill_manifests insert: {e}")))?;
    Ok(())
}
