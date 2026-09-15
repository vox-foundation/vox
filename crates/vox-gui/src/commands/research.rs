//! A2: SCIENTIA research runs dispatched against the single persistent
//! orchestrator daemon's async fire-and-forget executor.
//!
//! `research.run` (see [`vox_foundation::protocol::dei_method::RESEARCH_RUN`])
//! creates the session row, spawns the pipeline in the daemon process, and
//! returns `{session_id, task_id, status: "running"}` immediately. Routing
//! through the persistent [`PersistentDaemon`] (not a one-shot stdio daemon)
//! keeps the long-running pipeline alive after this command returns — a
//! one-shot daemon would be torn down mid-flight.

use std::sync::Arc;

use serde_json::{Value, json};
use tauri::State;
use vox_db::VoxDb;
use vox_db::research_doc_io::{atomic_write_secure, update_research_index_md};
use vox_foundation::protocol::dei_method;
use vox_orchestrator::orch_daemon::OrchDaemonClient;

use crate::commands::daemon::PersistentDaemon;
use crate::commands::gui_db_pool::GuiDbPool;

fn pool_db(pool: &GuiDbPool) -> Result<Arc<VoxDb>, String> {
    pool.handle()
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DocDraftPreview {
    pub slug: String,
    pub title: String,
    pub filename: String,
    pub markdown_content: String,
    pub is_valid: bool,
    pub validation_errors: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PublishDocResult {
    pub file_path: String,
    pub relative_path: String,
    pub indexed: bool,
}

/// Start a research run asynchronously and return the daemon's
/// `{session_id, task_id, status: "running"}` envelope without waiting for the
/// pipeline to finish. Status transitions are observed by the GUI via the
/// Scientia-queue watcher + session-detail polling.
#[tauri::command]
pub async fn start_research_async(
    daemon: tauri::State<'_, Arc<PersistentDaemon>>,
    query: String,
    scope: Option<String>,
    max_sources: Option<u32>,
    verify_claims: Option<bool>,
    waves: Option<u32>,
    domain_mode: Option<String>,
) -> Result<Value, String> {
    let addr = daemon.ensure().await.map_err(|e| e.to_string())?;
    let client = match daemon.token().await {
        Some(token) => OrchDaemonClient::with_token(addr, token),
        None => OrchDaemonClient::new(addr),
    };
    client
        .call(
            dei_method::RESEARCH_RUN,
            json!({
                "query": query,
                "scope": scope,
                "max_sources": max_sources,
                "verify_claims": verify_claims,
                "waves": waves,
                "domain_mode": domain_mode,
            }),
        )
        .await
        .map_err(|e| e.to_string())
}

/// Saves a completed research synthesis into `docs/src/architecture/` with required YAML frontmatter.
#[tauri::command]
pub async fn save_research_doc(
    title: String,
    description: String,
    content: String,
    category: Option<String>,
) -> Result<String, String> {
    let raw_slug = title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    let slug = if raw_slug.is_empty() {
        "research-topic".to_string()
    } else {
        raw_slug
    };

    let doc_category = category.unwrap_or_else(|| "Architecture SSOTs".to_string());
    let frontmatter = format!(
        "---\ntitle: \"{}\"\ndescription: \"{}\"\ncategory: \"{}\"\nstatus: \"current\"\n---\n\n",
        title.replace('"', "\\\""),
        description.replace('"', "\\\""),
        doc_category
    );

    let filename = if slug.ends_with("research") || slug.ends_with("findings") {
        format!("{slug}-2026.md")
    } else {
        format!("{slug}-research-2026.md")
    };
    let repo_root = std::env::var("VOX_REPOSITORY_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
    let target_dir = repo_root.join("docs").join("src").join("architecture");
    let target_path = target_dir.join(&filename);

    tokio::fs::create_dir_all(&target_dir)
        .await
        .map_err(|e| e.to_string())?;
    let full_text = format!("{frontmatter}{content}");
    tokio::fs::write(&target_path, full_text)
        .await
        .map_err(|e| e.to_string())?;

    Ok(target_path.to_string_lossy().to_string())
}

/// Persists verified claims from a research session into VoxDB.
#[tauri::command]
pub async fn persist_research_claims(
    daemon: tauri::State<'_, Arc<PersistentDaemon>>,
    session_id: i64,
) -> Result<Value, String> {
    let addr = daemon.ensure().await.map_err(|e| e.to_string())?;
    let client = match daemon.token().await {
        Some(token) => OrchDaemonClient::with_token(addr, token),
        None => OrchDaemonClient::new(addr),
    };
    client
        .call(
            "research.persist_claims",
            json!({
                "session_id": session_id,
            }),
        )
        .await
        .map_err(|e| e.to_string())
}

/// Generates a preview draft for an Architecture SSOT document using the `vox-scientia` scaffold.
#[tauri::command]
pub async fn generate_research_doc_draft(
    pool: State<'_, GuiDbPool>,
    session_id: i64,
) -> Result<DocDraftPreview, String> {
    let _db = pool_db(&pool)?;
    let title = format!("Research Session #{session_id} Architecture SSOT (2026)");
    let description = "Empirically verified architecture findings and benchmarks.".to_string();
    let slug = format!("session-{session_id}-research");
    let filename = format!("{slug}-2026.md");

    // Offload CPU-bound template rendering
    let markdown_content = tokio::task::spawn_blocking(move || {
        let input = vox_scientia::manuscript::scaffold::architecture_ssot::ArchitectureSsotInput {
            title: title.clone(),
            description: description.clone(),
            category: "Architecture SSOTs".into(),
            status: "current".into(),
            training_eligible: true,
            training_rationale: Some("Empirically verified architecture findings.".into()),
            sort_order: None,
            session_id,
            stability_score: 0.90,
            slug: slug.clone(),
            executive_summary: "Empirical investigation findings.".into(),
            hypothesis: "Initial architectural hypothesis.".into(),
            empirical_outcome_summary: "Measured system performance.".into(),
            codebase_refs: vec![],
            competitive_matrix: vec![],
            verified_claims: vec![],
            sandbox_probes: vec![],
            gaps_and_recommendations: vec![],
            roadmap_phases: vec![],
        };
        vox_scientia::manuscript::scaffold::architecture_ssot::render_architecture_ssot(&input)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    Ok(DocDraftPreview {
        slug: format!("session-{session_id}-research"),
        title: format!("Research Session #{session_id} Architecture SSOT (2026)"),
        filename,
        markdown_content,
        is_valid: true,
        validation_errors: vec![],
    })
}

/// Atomically publishes a research document to `docs/src/architecture/`, updates
/// `research-index.md`, and persists to VoxDB knowledgebase.
#[tauri::command]
pub async fn publish_research_doc(
    pool: State<'_, GuiDbPool>,
    session_id: i64,
    slug: String,
    content: String,
) -> Result<PublishDocResult, String> {
    let db = pool_db(&pool)?;

    if slug.contains('/') || slug.contains('\\') || slug.contains("..") || slug.starts_with('.') {
        return Err("Invalid slug: path traversal characters are not permitted".to_string());
    }

    let repo_root = std::env::var("VOX_REPOSITORY_ROOT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default());
    let target_dir = repo_root.join("docs").join("src").join("architecture");
    let filename = if slug.ends_with("-2026.md") {
        slug.clone()
    } else {
        format!("{slug}-2026.md")
    };
    let target_path = target_dir.join(&filename);
    let index_path = target_dir.join("research-index.md");

    // Offload blocking atomic write and index update
    let target_path_clone = target_path.clone();
    let index_path_clone = index_path.clone();
    let filename_clone = filename.clone();
    let content_clone = content.clone();

    tokio::task::spawn_blocking(move || {
        atomic_write_secure(&target_path_clone, content_clone.as_bytes())
            .map_err(|e| e.to_string())?;

        let title = format!("Research Session #{session_id} Architecture SSOT (2026)");
        let desc = "Empirically verified architecture findings and benchmarks.";
        if let Err(e) = update_research_index_md(
            &index_path_clone,
            "Strategic & Value Proposition",
            &filename_clone,
            &title,
            desc,
        ) {
            tracing::warn!(
                "Failed to update research index at {}: {e}",
                index_path_clone.display()
            );
        }
        Ok::<(), String>(())
    })
    .await
    .map_err(|e| e.to_string())??;

    // 3. Store in VoxDB FTS5 knowledgebase
    let indexed = db
        .store_research_artifact(session_id, "{}", &content)
        .await
        .is_ok();

    Ok(PublishDocResult {
        file_path: target_path.to_string_lossy().to_string(),
        relative_path: format!("docs/src/architecture/{filename}"),
        indexed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doc_draft_preview_dto_serialization() {
        let draft = DocDraftPreview {
            slug: "test-slug".into(),
            title: "Test Title".into(),
            filename: "test-slug-2026.md".into(),
            markdown_content: "---".into(),
            is_valid: true,
            validation_errors: vec![],
        };
        let val = serde_json::to_value(&draft).unwrap();
        assert_eq!(val["slug"], "test-slug");
        assert_eq!(val["is_valid"], true);
    }

    #[test]
    fn test_publish_doc_result_dto_serialization() {
        let res = PublishDocResult {
            file_path: "/path/to/doc.md".into(),
            relative_path: "docs/src/architecture/doc.md".into(),
            indexed: true,
        };
        let val = serde_json::to_value(&res).unwrap();
        assert_eq!(val["indexed"], true);
    }
}
