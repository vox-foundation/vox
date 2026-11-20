//! Explicit multi-repo catalog and read-only polyrepo query tools.
//!
//! Cross-repo queries may append a **`benchmark_event`** row via `record_query_metric` (latency + trace metadata) when Codex
//! is attached — **no extra env gate** beyond having `VoxDb`. Mesh snapshot Codex mirroring is separate (see `dei_tools::orchestrator_snapshot`).

use crate::params::ToolResult;
use crate::server_state::ServerState;
use vox_actor_runtime::supervisor::spawn_supervised_infallible;
use vox_db::TrustObservationInput;

const REM_REPO_CATALOG: &str = "Add `.vox/repositories.yaml` under the current workspace root and keep local repo paths explicit.";
const REM_REPO_QUERY: &str = "Ensure the repo catalog resolves local repositories successfully before running cross-repo queries.";

pub async fn repo_status(state: &ServerState) -> String {
    let status = vox_repository::repo_workspace_status_for_cwd(&state.repository.root);
    let active_context = vox_repository::get_active_repository_context(&state.repository.root).ok();
    let is_ro = active_context.as_ref().is_some_and(|c| c.read_only);
    let target_root = active_context
        .as_ref()
        .map(|c| std::path::PathBuf::from(&c.root_path))
        .unwrap_or_else(|| status.root.clone());
    let openwiki = inspect_openwiki_summary(&target_root, is_ro);
    let mut value = serde_json::to_value(&status).unwrap_or_else(|_| serde_json::json!({}));
    if let Some(obj) = value.as_object_mut() {
        if let Some(ctx) = active_context
            && let Ok(ctx_val) = serde_json::to_value(ctx)
        {
            obj.insert("active_context".to_string(), ctx_val);
        }
        obj.insert("openwiki".to_string(), openwiki);
    }
    ToolResult::ok(value).to_json()
}

/// Fire-and-forget `research_metrics` benchmark row for polyrepo queries (trace ids, backend, counts).
fn record_query_metric(
    state: &ServerState,
    query_kind: &str,
    trace: &vox_repository::CrossRepoQueryTrace,
    result_count: usize,
    skipped_count: usize,
) {
    let Some(db) = state.db.as_ref().cloned() else {
        return;
    };
    let repository_id = state.repository.repository_id.clone();
    let latency_ms = trace.latency_ms;
    let details = serde_json::json!({
        "query_kind": query_kind,
        "trace_id": trace.trace_id,
        "correlation_id": trace.correlation_id,
        "conversation_id": trace.conversation_id,
        "workspace_repository_id": trace.workspace_repository_id,
        "target_repository_ids": trace.target_repository_ids,
        "source_plane": trace.source_plane,
        "query_backend": trace.query_backend,
        "result_count": result_count,
        "skipped_count": skipped_count,
    });
    spawn_supervised_infallible("repo_catalog_metric", async move {
        let _ = db
            .record_benchmark_event(
                &repository_id,
                "cross_repo_query",
                Some(latency_ms as f64),
                Some("milliseconds"),
                Some(details),
            )
            .await;
    });
}

fn record_catalog_refresh_observation(
    state: &ServerState,
    success: bool,
    artifact_ref: Option<&str>,
    metadata_json: serde_json::Value,
) {
    let Some(db) = state.db.as_ref().cloned() else {
        return;
    };
    let repository_id = state.repository.repository_id.clone();
    let artifact_ref = artifact_ref.map(str::to_string);
    let metadata_text = metadata_json.to_string();
    spawn_supervised_infallible("repo_catalog_metric", async move {
        let _ = db
            .record_trust_observation(TrustObservationInput {
                entity_type: "repository",
                entity_id: &repository_id,
                dimension: "repo_catalog_freshness",
                domain: Some("cross_repo_query"),
                task_class: None,
                provider: None,
                model_id: None,
                repository_id: Some(&repository_id),
                source_kind: Some("vox_repo_catalog_refresh"),
                observation_value: if success { 1.0 } else { 0.0 },
                confidence_weight: 1.0,
                sample_size: 1,
                artifact_ref: artifact_ref.as_deref(),
                metadata_json: Some(&metadata_text),
                ewma_alpha: 0.10,
            })
            .await;
    });
}

async fn get_catalog(
    state: &ServerState,
) -> Result<vox_repository::ResolvedRepoCatalog, vox_repository::RepoCatalogError> {
    let manifest_path = vox_repository::repo_catalog_manifest_path(&state.repository.root);
    let mtime = std::fs::metadata(&manifest_path)
        .and_then(|m| m.modified())
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

    {
        let cache = state.catalog_cache.read().await;
        if let Some(cached) = &*cache {
            if cached.manifest_mtime == mtime {
                return Ok(cached.resolved.clone());
            }
        }
    }

    let resolved = vox_repository::resolve_repo_catalog(&state.repository.root)?;
    let mut cache = state.catalog_cache.write().await;
    *cache = Some(crate::server_state::CachedCatalog {
        resolved: resolved.clone(),
        manifest_mtime: mtime,
    });

    Ok(resolved)
}

pub async fn repo_catalog_list(state: &ServerState) -> String {
    match get_catalog(state).await {
        Ok(catalog) => ToolResult::ok(catalog).to_json(),
        Err(e) => {
            ToolResult::<String>::err_with_remediation(format!("{e}"), REM_REPO_CATALOG).to_json()
        }
    }
}

pub async fn repo_catalog_refresh(state: &ServerState, args: serde_json::Value) -> String {
    let action = args
        .get("action")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("refresh");
    let read_only = args
        .get("read_only")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    match action {
        "init" => {
            let target_root = args
                .get("path")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|raw| {
                    let p = std::path::PathBuf::from(raw);
                    if p.is_absolute() {
                        p
                    } else {
                        state.repository.root.join(p)
                    }
                })
                .unwrap_or_else(|| state.repository.root.clone());
            return match vox_repository::onboard_repository_harness(&target_root, read_only) {
                Ok(summary) => {
                    *state.catalog_cache.write().await = None;
                    ToolResult::ok(summary).to_json()
                }
                Err(e) => {
                    ToolResult::<String>::err_with_remediation(format!("{e}"), REM_REPO_CATALOG)
                        .to_json()
                }
            };
        }
        "add" => {
            let Some(raw_path) = args
                .get("path")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                return ToolResult::<String>::err_with_remediation(
                    "Missing `path` for action `add`",
                    REM_REPO_CATALOG,
                )
                .to_json();
            };
            let target_path = {
                let p = std::path::PathBuf::from(raw_path);
                if p.is_absolute() {
                    p
                } else {
                    state.repository.root.join(p)
                }
            };
            let display_name = args
                .get("display_name")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty());
            return match vox_repository::add_repository_to_catalog(
                &state.repository.root,
                &target_path,
                display_name,
                read_only,
            ) {
                Ok(catalog) => {
                    *state.catalog_cache.write().await = None;
                    ToolResult::ok(catalog).to_json()
                }
                Err(e) => {
                    ToolResult::<String>::err_with_remediation(format!("{e}"), REM_REPO_CATALOG)
                        .to_json()
                }
            };
        }
        "remove" => {
            let Some(target) = args
                .get("target")
                .or_else(|| args.get("repository_id"))
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                return ToolResult::<String>::err_with_remediation(
                    "Missing `target` for action `remove`",
                    REM_REPO_CATALOG,
                )
                .to_json();
            };
            return match vox_repository::remove_repository_from_catalog(
                &state.repository.root,
                target,
            ) {
                Ok(catalog) => {
                    *state.catalog_cache.write().await = None;
                    ToolResult::ok(catalog).to_json()
                }
                Err(e) => {
                    ToolResult::<String>::err_with_remediation(format!("{e}"), REM_REPO_CATALOG)
                        .to_json()
                }
            };
        }
        "switch" => {
            let Some(target) = args
                .get("target")
                .or_else(|| args.get("repository_id"))
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                return ToolResult::<String>::err_with_remediation(
                    "Missing `target` for action `switch`",
                    REM_REPO_CATALOG,
                )
                .to_json();
            };
            return match vox_repository::switch_active_repository(&state.repository.root, target) {
                Ok(ctx) => {
                    if let Some(db) = state.db.as_ref().cloned() {
                        let switched_root = std::path::PathBuf::from(&ctx.root_path);
                        let _ = crate::kb_tools::library::ensure_kb_indexed_for_repo(
                            &db,
                            &switched_root,
                        )
                        .await;
                    }
                    ToolResult::ok(ctx).to_json()
                }
                Err(e) => {
                    ToolResult::<String>::err_with_remediation(format!("{e}"), REM_REPO_CATALOG)
                        .to_json()
                }
            };
        }
        "wiki_init" | "wiki_build" | "wiki_status" | "wiki_diff" | "wiki_sync"
        | "wiki_heal_tier1" | "wiki_heal_list" | "wiki_heal_propose" | "wiki_heal_apply"
        | "wiki_heal_reject" | "wiki_heal_demote" => {
            let target_root = args
                .get("path")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(|raw| {
                    let p = std::path::PathBuf::from(raw);
                    if p.is_absolute() {
                        p
                    } else {
                        state.repository.root.join(p)
                    }
                })
                .unwrap_or_else(|| {
                    vox_repository::get_active_repository_context(&state.repository.root)
                        .map(|c| std::path::PathBuf::from(c.root_path))
                        .unwrap_or_else(|_| state.repository.root.clone())
                });
            let eff_read_only = read_only
                || vox_repository::get_active_repository_context(&state.repository.root)
                    .is_ok_and(|c| c.read_only);
            return match execute_openwiki_heal_mcp_action(
                state,
                action,
                &args,
                &target_root,
                eff_read_only,
            )
            .await
            {
                Ok(summary) => ToolResult::ok(summary).to_json(),
                Err(e) => ToolResult::<String>::err_with_remediation(e, REM_REPO_CATALOG).to_json(),
            };
        }
        _ => {}
    }

    match vox_repository::refresh_repo_catalog(&state.repository.root) {
        Ok(result) => {
            record_catalog_refresh_observation(
                state,
                true,
                Some(&result.snapshot_path),
                serde_json::json!({
                    "manifest_path": result.manifest_path,
                    "snapshot_path": result.snapshot_path,
                    "repository_count": result.catalog.repositories.len(),
                }),
            );
            // Invalidate the cache by removing it
            *state.catalog_cache.write().await = None;
            ToolResult::ok(result).to_json()
        }
        Err(e) => {
            record_catalog_refresh_observation(
                state,
                false,
                None,
                serde_json::json!({
                    "error": e.to_string(),
                }),
            );
            ToolResult::<String>::err_with_remediation(format!("{e}"), REM_REPO_CATALOG).to_json()
        }
    }
}

pub async fn repo_query_text(
    state: &ServerState,
    params: vox_repository::QueryTextParams,
) -> String {
    let catalog = get_catalog(state).await.ok();
    match vox_repository::repo_query_text_with_plane(
        &state.repository.root,
        &params,
        "mcp",
        catalog.as_ref(),
    ) {
        Ok(response) => {
            record_query_metric(
                state,
                "query_text",
                &response.trace,
                response.result_count,
                response.skipped.len(),
            );
            ToolResult::ok(response).to_json()
        }
        Err(e) => {
            ToolResult::<String>::err_with_remediation(format!("{e}"), REM_REPO_QUERY).to_json()
        }
    }
}

pub async fn repo_query_file(
    state: &ServerState,
    params: vox_repository::QueryFileParams,
) -> String {
    let catalog = get_catalog(state).await.ok();
    match vox_repository::repo_query_file_with_plane(
        &state.repository.root,
        &params,
        "mcp",
        catalog.as_ref(),
    ) {
        Ok(response) => {
            record_query_metric(
                state,
                "query_file",
                &response.trace,
                response.result_count,
                response.skipped.len(),
            );
            ToolResult::ok(response).to_json()
        }
        Err(e) => {
            ToolResult::<String>::err_with_remediation(format!("{e}"), REM_REPO_QUERY).to_json()
        }
    }
}

pub async fn repo_query_history(
    state: &ServerState,
    params: vox_repository::QueryHistoryParams,
) -> String {
    let catalog = get_catalog(state).await.ok();
    match vox_repository::repo_query_history_with_plane(
        &state.repository.root,
        &params,
        "mcp",
        catalog.as_ref(),
    ) {
        Ok(response) => {
            record_query_metric(
                state,
                "query_history",
                &response.trace,
                response.result_count,
                response.skipped.len(),
            );
            ToolResult::ok(response).to_json()
        }
        Err(e) => {
            ToolResult::<String>::err_with_remediation(format!("{e}"), REM_REPO_QUERY).to_json()
        }
    }
}

/// Zero-footprint cache override for `read_only` external repos (`~/.vox/repos/<id>/cache/openwiki`).
fn openwiki_override(target_root: &std::path::Path, read_only: bool) -> Option<std::path::PathBuf> {
    if !read_only {
        return None;
    }
    let home = vox_repository::vox_user_home_dir().ok()?;
    let repo = vox_repository::discover_repository_or_fallback(target_root);
    Some(
        vox_repository::external_repo_cache_root_under(&home, &repo.repository_id)
            .join("cache")
            .join("openwiki"),
    )
}

/// Status payload backed by the shared `vox-doc-pipeline` engine (same pages, same regions as
/// `vox ci wiki-sync`). Keeps the legacy keys the Axis `OpenWikiHealStudio` reads.
fn inspect_openwiki_summary(target_root: &std::path::Path, read_only: bool) -> serde_json::Value {
    let ov = openwiki_override(target_root, read_only);
    let pages_rel = vox_doc_pipeline::pipeline::openwiki::openwiki_pages_rel(target_root);
    let rep =
        match vox_doc_pipeline::pipeline::openwiki::openwiki_status(target_root, ov.as_deref()) {
            Ok(rep) => rep,
            Err(e) => {
                return serde_json::json!({
                    "initialized": false,
                    "page_count": 0,
                    "total_pages": 0,
                    "auto_region_count": 0,
                    "legacy_mermaid_blocks": 0,
                    "stale_pages": [],
                    "pages": [],
                    "error": e,
                });
            }
        };
    let wiki_dir = std::path::PathBuf::from(&rep.wiki_dir);
    let mut page_count = 0usize;
    let mut auto_region_count = 0usize;
    let mut legacy_mermaid_blocks = 0usize;
    let mut stale_pages: Vec<String> = Vec::new();
    let mut pages = Vec::new();
    for p in &rep.pages {
        let mut legacy = 0usize;
        if p.exists {
            page_count += 1;
            if let Ok(text) = std::fs::read_to_string(wiki_dir.join(&p.rel_path)) {
                legacy = vox_diagram::fence::extract(&text, "mermaid").len();
            }
        }
        auto_region_count += p.region_ids.len();
        legacy_mermaid_blocks += legacy;
        if p.stale {
            stale_pages.push(p.rel_path.clone());
        }
        pages.push(serde_json::json!({
            "rel_path": p.rel_path,
            "doc_path": format!("{pages_rel}/{}", p.rel_path),
            "exists": p.exists,
            "region_ids": p.region_ids,
            "stale_regions": p.stale_regions,
            "legacy_mermaid_blocks": legacy,
        }));
    }
    serde_json::json!({
        "wiki_dir": rep.wiki_dir,
        "manifest_path": rep.manifest_path,
        "manifest_present": rep.manifest_present,
        "detected_stacks": rep.detected_stacks,
        "initialized": rep.initialized,
        "page_count": page_count,
        "total_pages": rep.total_pages,
        "auto_region_count": auto_region_count,
        "legacy_mermaid_blocks": legacy_mermaid_blocks,
        "stale_pages": stale_pages,
        "pages": pages,
    })
}

/// Run an OpenWiki action through the shared engine (no placeholder generator).
fn execute_openwiki_mcp_action(
    action: &str,
    target_root: &std::path::Path,
    read_only: bool,
) -> Result<serde_json::Value, String> {
    use vox_doc_pipeline::pipeline::openwiki as ow;
    let ov = openwiki_override(target_root, read_only);
    let mut extra = serde_json::Map::new();
    match action {
        "wiki_init" => {
            let rep = ow::openwiki_init(target_root, ov.as_deref())?;
            extra.insert(
                "report".into(),
                serde_json::to_value(rep).unwrap_or_default(),
            );
        }
        "wiki_build" => {
            let rep = ow::openwiki_build(target_root, true, ov.as_deref())?;
            extra.insert(
                "report".into(),
                serde_json::to_value(rep).unwrap_or_default(),
            );
        }
        "wiki_sync" | "wiki_heal_tier1" => {
            let rep = ow::heal_tier1_deterministic(target_root, &[], true, ov.as_deref())?;
            extra.insert("llm_calls".into(), serde_json::json!(rep.llm_calls));
            extra.insert(
                "report".into(),
                serde_json::to_value(rep).unwrap_or_default(),
            );
        }
        "wiki_diff" => {
            let rep = ow::openwiki_diff(target_root, ov.as_deref())?;
            extra.insert("has_changes".into(), serde_json::json!(rep.has_changes));
            extra.insert(
                "report".into(),
                serde_json::to_value(rep).unwrap_or_default(),
            );
        }
        "wiki_status" => {}
        other => return Err(format!("Unsupported OpenWiki action `{other}`")),
    }
    let mut summary = inspect_openwiki_summary(target_root, read_only);
    if let Some(obj) = summary.as_object_mut() {
        obj.insert("action".to_string(), serde_json::json!(action));
        for (k, v) in extra {
            obj.insert(k, v);
        }
        if action == "wiki_status" {
            let has_changes = obj
                .get("stale_pages")
                .and_then(|v| v.as_array())
                .is_some_and(|a| !a.is_empty());
            obj.insert("has_changes".to_string(), serde_json::json!(has_changes));
        }
    }
    Ok(summary)
}

fn demote_doc_frontmatter_to_legacy(
    target_root: &std::path::Path,
    doc_rel_path: &str,
) -> Result<(), String> {
    let norm = doc_rel_path.trim().replace('\\', "/");
    if !norm.starts_with("docs/") || norm.contains("..") {
        return Err(format!("refusing out-of-scope doc path `{norm}`"));
    }
    let full = target_root.join(&norm);
    let raw = std::fs::read_to_string(&full)
        .map_err(|e| format!("failed to read {}: {e}", full.display()))?;
    let updated = if let Some(rest) = raw.strip_prefix("---\n")
        && let Some(end) = rest.find("\n---")
    {
        let fm = &rest[..end];
        let new_fm: String = if fm.lines().any(|l| l.trim_start().starts_with("status:")) {
            fm.lines()
                .map(|l| {
                    if l.trim_start().starts_with("status:") {
                        "status: \"legacy\"".to_string()
                    } else {
                        l.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        } else {
            format!("{fm}\nstatus: \"legacy\"")
        };
        format!("---\n{new_fm}{}", &rest[end..])
    } else {
        format!("---\nstatus: \"legacy\"\n---\n\n{raw}")
    };
    std::fs::write(&full, updated)
        .map_err(|e| format!("failed to write {}: {e}", full.display()))?;
    Ok(())
}

async fn execute_openwiki_heal_mcp_action(
    state: &ServerState,
    action: &str,
    args: &serde_json::Value,
    target_root: &std::path::Path,
    read_only: bool,
) -> Result<serde_json::Value, String> {
    match action {
        "wiki_init" | "wiki_build" | "wiki_status" | "wiki_diff" | "wiki_sync"
        | "wiki_heal_tier1" => execute_openwiki_mcp_action(action, target_root, read_only),
        "wiki_heal_list" => {
            let db = state
                .db
                .as_ref()
                .ok_or_else(|| "VoxDb is not attached to MCP server state".to_string())?;
            let repo_id = args
                .get("repository_id")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty());
            let status_filter = args
                .get("status")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty());
            let rows = db
                .kb_heal_proposals_list(repo_id, status_filter, 100)
                .await
                .map_err(|e| format!("kb_heal_proposals_list failed: {e}"))?;
            Ok(serde_json::json!({
                "action": action,
                "proposals": rows,
                "count": rows.len(),
            }))
        }
        "wiki_heal_propose" => {
            let db = state
                .db
                .as_ref()
                .ok_or_else(|| "VoxDb is not attached to MCP server state".to_string())?;
            let repo = vox_repository::discover_repository_or_fallback(target_root);
            let repository_id = args
                .get("repository_id")
                .and_then(|v| v.as_str())
                .unwrap_or(&repo.repository_id)
                .to_string();
            let doc_path = args
                .get("doc_path")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "Missing `doc_path` for action `wiki_heal_propose`".to_string())?
                .to_string();
            let proposal_id = args
                .get("proposal_id")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .unwrap_or_else(|| format!("heal:{}:{}", repository_id, doc_path));
            let summary_claim_rewrite = args
                .get("summary_claim_rewrite")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            let prose_note = args
                .get("prose_note")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            let evidence_citations: Vec<String> = args
                .get("evidence_citations")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|x| x.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default();

            let outcome = vox_search::promotion::stage_verified_prose_heal_proposal(
                db,
                target_root,
                vox_search::promotion::StageHealProposalInput {
                    proposal_id,
                    repository_id,
                    doc_path: doc_path.clone(),
                    summary_claim_rewrite,
                    prose_note,
                    evidence_citations,
                    mechanical_input: vox_search::kb_staleness::MechanicalEvidenceInput {
                        doc_path,
                        ..Default::default()
                    },
                    has_mechanical_anchors: true,
                    llm_config: None,
                },
            )
            .await
            .map_err(|e| format!("{e:#}"))?;
            Ok(serde_json::to_value(outcome).unwrap_or_default())
        }
        "wiki_heal_apply" => {
            let db = state
                .db
                .as_ref()
                .ok_or_else(|| "VoxDb is not attached to MCP server state".to_string())?;
            let id = args
                .get("proposal_id")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "Missing `proposal_id` for `wiki_heal_apply`".to_string())?;
            let reviewer = args
                .get("reviewed_by")
                .and_then(|v| v.as_str())
                .unwrap_or("mcp-operator");
            let note = args.get("review_note").and_then(|v| v.as_str());
            let written = vox_search::promotion::apply_staged_heal_proposal(
                db,
                target_root,
                id,
                reviewer,
                note,
            )
            .await
            .map_err(|e| format!("{e:#}"))?;
            let _ = crate::kb_tools::library::ensure_kb_indexed_for_repo(db, target_root).await;
            Ok(serde_json::json!({
                "action": action,
                "proposal_id": id,
                "status": "applied",
                "path": written.display().to_string(),
            }))
        }
        "wiki_heal_reject" => {
            let db = state
                .db
                .as_ref()
                .ok_or_else(|| "VoxDb is not attached to MCP server state".to_string())?;
            let id = args
                .get("proposal_id")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "Missing `proposal_id` for `wiki_heal_reject`".to_string())?;
            let reviewer = args
                .get("reviewed_by")
                .and_then(|v| v.as_str())
                .unwrap_or("mcp-operator");
            let note = args.get("review_note").and_then(|v| v.as_str());
            let updated = db
                .kb_heal_proposal_update_status(id, "rejected", Some(reviewer), note)
                .await
                .map_err(|e| format!("kb_heal_proposal_update_status failed: {e}"))?;
            Ok(serde_json::json!({
                "action": action,
                "proposal_id": id,
                "status": "rejected",
                "updated": updated,
            }))
        }
        "wiki_heal_demote" => {
            let db = state
                .db
                .as_ref()
                .ok_or_else(|| "VoxDb is not attached to MCP server state".to_string())?;
            let id = args
                .get("proposal_id")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| "Missing `proposal_id` for `wiki_heal_demote`".to_string())?;
            let row = db
                .kb_heal_proposal_get(id)
                .await
                .map_err(|e| format!("kb_heal_proposal_get failed: {e}"))?
                .ok_or_else(|| format!("Heal proposal `{id}` not found"))?;
            demote_doc_frontmatter_to_legacy(target_root, &row.doc_path)?;
            let reviewer = args
                .get("reviewed_by")
                .and_then(|v| v.as_str())
                .unwrap_or("mcp-operator");
            let note = args.get("review_note").and_then(|v| v.as_str());
            let updated = db
                .kb_heal_proposal_update_status(id, "demoted_to_legacy", Some(reviewer), note)
                .await
                .map_err(|e| format!("kb_heal_proposal_update_status failed: {e}"))?;
            Ok(serde_json::json!({
                "action": action,
                "proposal_id": id,
                "doc_path": row.doc_path,
                "status": "demoted_to_legacy",
                "updated": updated,
            }))
        }
        other => Err(format!("Unsupported OpenWiki action `{other}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn repo_catalog_refresh_wiki_actions_and_status_inspect_openwiki() {
        let dir = std::env::temp_dir().join(format!("vox-mcp-openwiki-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"mcp-wiki\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let state = ServerState::new_test().await;
        let res_str = repo_catalog_refresh(
            &state,
            serde_json::json!({
                "action": "wiki_init",
                "path": dir.display().to_string(),
            }),
        )
        .await;
        let res: serde_json::Value = serde_json::from_str(&res_str).unwrap();
        assert_eq!(res["success"], true, "{res}");
        assert_eq!(res["data"]["initialized"], true);
        assert_eq!(res["data"]["page_count"], 6);
        assert_eq!(res["data"]["legacy_mermaid_blocks"], 0);

        let st_str = repo_catalog_refresh(
            &state,
            serde_json::json!({
                "action": "wiki_status",
                "path": dir.display().to_string(),
            }),
        )
        .await;
        let st: serde_json::Value = serde_json::from_str(&st_str).unwrap();
        assert_eq!(st["success"], true, "{st}");
        assert_eq!(st["data"]["stale_pages"], serde_json::json!([]));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn mcp_wiki_heal_propose_list_apply_demote_and_reject_cycle() {
        let (gs, _db) = crate::gui_ops::test_support::gateway_with_memory_db().await;
        let state = gs.server_state();
        let dir = std::env::temp_dir().join(format!("vox-mcp-heal-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(dir.join("src/auth.rs"), "pub fn check() -> bool { true }\n").unwrap();

        let init_str = repo_catalog_refresh(
            state,
            serde_json::json!({
                "action": "wiki_init",
                "path": dir.display().to_string(),
            }),
        )
        .await;
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&init_str).unwrap()["success"],
            true
        );

        let propose_str = repo_catalog_refresh(
            state,
            serde_json::json!({
                "action": "wiki_heal_propose",
                "path": dir.display().to_string(),
                "proposal_id": "heal-p1",
                "doc_path": "docs/openwiki/interfaces-and-apis.md",
                "prose_note": "Verified `check` entry point.",
                "evidence_citations": ["[src/auth.rs:L1-L1]"],
            }),
        )
        .await;
        let prop: serde_json::Value = serde_json::from_str(&propose_str).unwrap();
        assert_eq!(prop["success"], true, "{prop}");

        let list_str = repo_catalog_refresh(
            state,
            serde_json::json!({
                "action": "wiki_heal_list",
                "status": "pending",
            }),
        )
        .await;
        let listed: serde_json::Value = serde_json::from_str(&list_str).unwrap();
        assert_eq!(listed["data"]["count"], 1, "{listed}");

        let apply_str = repo_catalog_refresh(
            state,
            serde_json::json!({
                "action": "wiki_heal_apply",
                "path": dir.display().to_string(),
                "proposal_id": "heal-p1",
                "reviewed_by": "qa",
            }),
        )
        .await;
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&apply_str).unwrap()["data"]["status"],
            "applied"
        );
        assert!(
            std::fs::read_to_string(dir.join("docs/openwiki/interfaces-and-apis.md"))
                .unwrap()
                .contains("Verified `check` entry point.")
        );

        let demote_str = repo_catalog_refresh(
            state,
            serde_json::json!({
                "action": "wiki_heal_demote",
                "path": dir.display().to_string(),
                "proposal_id": "heal-p1",
            }),
        )
        .await;
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&demote_str).unwrap()["data"]["status"],
            "demoted_to_legacy"
        );
        assert!(
            std::fs::read_to_string(dir.join("docs/openwiki/interfaces-and-apis.md"))
                .unwrap()
                .contains("status: \"legacy\"")
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn mcp_wiki_build_on_docs_src_repo_targets_docs_src_and_creates_no_docs_wiki() {
        let dir = std::env::temp_dir().join(format!("vox-mcp-docsrc-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("docs/src")).unwrap();
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname = \"mcp-docsrc\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("src/lib.rs"),
            "pub fn alpha_entry() -> bool { true }\n",
        )
        .unwrap();

        let state = ServerState::new_test().await;
        let res_str = repo_catalog_refresh(
            &state,
            serde_json::json!({"action": "wiki_build", "path": dir.display().to_string()}),
        )
        .await;
        let res: serde_json::Value = serde_json::from_str(&res_str).unwrap();
        assert_eq!(res["success"], true, "{res}");
        assert_eq!(res["data"]["initialized"], true, "{res}");
        assert!(dir.join("docs/src/openwiki/README.md").is_file());
        assert!(!dir.join("docs/wiki").exists(), "must not create docs/wiki");
        let l2 =
            std::fs::read_to_string(dir.join("docs/src/openwiki/interfaces-and-apis.md")).unwrap();
        assert!(
            l2.contains("alpha_entry"),
            "real extraction, not a 1-node stub"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
