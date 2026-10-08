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
use vox_foundation::protocol::dei_method;
use vox_orchestrator::orch_daemon::OrchDaemonClient;
use vox_orchestrator_mcp::kb_tools::findings::file_publish_draft;

use crate::commands::daemon::PersistentDaemon;
use crate::commands::gui_db_pool::GuiDbPool;

fn pool_db(pool: &GuiDbPool) -> Result<Arc<VoxDb>, String> {
    pool.handle()
}

/// Repository root for research-doc writes: Clavis `VoxRepositoryRoot` (`VOX_REPO_ROOT`, alias
/// `VOX_REPOSITORY_ROOT`), else the current directory.
pub(crate) fn research_repo_root() -> std::path::PathBuf {
    vox_secrets::resolve_secret(vox_secrets::SecretId::VoxRepositoryRoot)
        .expose()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
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
    pub finding_id: Option<String>,
    pub proposed_path: String,
    /// `"pending"` or `"duplicate"`.
    pub status: String,
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
    lane: Option<String>,
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
                "lane": lane,
            }),
        )
        .await
        .map_err(|e| e.to_string())
}

/// Flag a citation or claim as misleading research and update domain reputation penalty in VoxDb.
#[tauri::command]
pub async fn flag_research_misleading(
    pool: tauri::State<'_, GuiDbPool>,
    params: vox_db::RecordMisguidanceParams,
) -> Result<i64, String> {
    pool_db(&pool)?
        .record_research_misguidance(&params)
        .await
        .map_err(|e| e.to_string())
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

/// Files a drafted research doc as a pending finding in the review inbox. Nothing is written
/// under `docs/src`: a reviewer promotes it later (GROW-06). The episodic research log
/// (`store_research_artifact`) is kept; it is not the KB.
#[tauri::command]
pub async fn publish_research_doc(
    pool: State<'_, GuiDbPool>,
    session_id: i64,
    slug: String,
    content: String,
) -> Result<PublishDocResult, String> {
    let db = pool_db(&pool)?;
    let filed = file_publish_draft(
        db.as_ref(),
        &research_repo_root(),
        session_id,
        &slug,
        &content,
    )
    .await?;

    // Best-effort episodic log; its failure must not fail the filing.
    let _ = db.store_research_artifact(session_id, "{}", &content).await;

    Ok(PublishDocResult {
        finding_id: filed.finding_id,
        proposed_path: filed.proposed_path,
        status: filed.status,
    })
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SandboxProbeOutcome {
    pub passed: bool,
    pub stdout: String,
    pub stderr: String,
}

#[tauri::command]
pub async fn execute_sandbox_probe(
    code: String,
    language: String,
) -> Result<SandboxProbeOutcome, String> {
    let lang = match language.trim().to_lowercase().as_str() {
        "python" => vox_research_shim::research::domain::polyglot_sandbox::PolyglotLanguage::Python,
        "typescript" | "ts" => {
            vox_research_shim::research::domain::polyglot_sandbox::PolyglotLanguage::TypeScript
        }
        "sql" => vox_research_shim::research::domain::polyglot_sandbox::PolyglotLanguage::Sql,
        "vox" => vox_research_shim::research::domain::polyglot_sandbox::PolyglotLanguage::Vox,
        _ => vox_research_shim::research::domain::polyglot_sandbox::PolyglotLanguage::Rust,
    };
    let res = vox_research_shim::research::domain::polyglot_sandbox::verify_in_polyglot_sandbox(
        &code, lang,
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(SandboxProbeOutcome {
        passed: res.passed,
        stdout: res.stdout,
        stderr: res.stderr,
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
            finding_id: Some("f1".into()),
            proposed_path: "docs/src/architecture/doc-2026.md".into(),
            status: "pending".into(),
        };
        let val = serde_json::to_value(&res).unwrap();
        assert_eq!(val["status"], "pending");
        assert_eq!(val["proposed_path"], "docs/src/architecture/doc-2026.md");
    }

    #[test]
    fn test_sandbox_probe_outcome_dto_serialization() {
        let outcome = SandboxProbeOutcome {
            passed: true,
            stdout: "hello".into(),
            stderr: "".into(),
        };
        let val = serde_json::to_value(&outcome).unwrap();
        assert_eq!(val["passed"], true);
        assert_eq!(val["stdout"], "hello");
        assert_eq!(val["stderr"], "");
    }

    #[test]
    fn test_record_misguidance_params_deserialization() {
        let json_str = r#"{
            "session_id": 1,
            "defect_class": "inelegant_code",
            "culprit_url": "https://flawed-docs.com",
            "culprit_domain": "flawed-docs.com",
            "claim_id": null,
            "research_query": "What is Vox?",
            "misleading_excerpt": "code snippet",
            "generated_code_snippet": null,
            "failure_diagnostic": null,
            "correction_diff": null,
            "reporter": "user",
            "domain_penalty": 0.2
        }"#;
        let params: vox_db::RecordMisguidanceParams =
            serde_json::from_str(json_str).expect("deserialize RecordMisguidanceParams");
        assert_eq!(params.session_id, Some(1));
        assert_eq!(
            params.defect_class,
            vox_db::ResearchDefectClass::InelegantCode
        );
        assert_eq!(params.culprit_domain, "flawed-docs.com");
        assert_eq!(params.reporter, vox_db::MisguidanceReporter::User);
    }
}
