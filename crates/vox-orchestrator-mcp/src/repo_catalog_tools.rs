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
                Ok(ctx) => ToolResult::ok(ctx).to_json(),
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

const CANONICAL_OPENWIKI_PAGES: &[(&str, &str, &str, &str)] = &[
    ("README.md", "OpenWiki Hub", "doc-pipeline", "documentation"),
    (
        "architecture-topology.md",
        "Layer 1: Architecture & Module Topology",
        "search-retrieval",
        "knowledge",
    ),
    (
        "interfaces-and-apis.md",
        "Layer 2: CLI, HTTP, MCP & API Interfaces",
        "cli-surface",
        "platform",
    ),
    (
        "data-models-and-schemas.md",
        "Layer 3: Data Models & SQL/VoxDb Schemas",
        "database",
        "data",
    ),
    (
        "gui-surfaces-and-flows.md",
        "Layer 4: GUI & Frontend Surfaces",
        "desktop-app",
        "gui",
    ),
    (
        "workflows-and-runbooks.md",
        "Layer 5: Workflows, Actors & Runbooks",
        "durable-workflows",
        "runtime",
    ),
];

fn resolve_openwiki_paths(
    target_root: &std::path::Path,
    read_only: bool,
) -> (std::path::PathBuf, std::path::PathBuf) {
    if read_only && let Ok(home) = vox_repository::vox_user_home_dir() {
        let repo = vox_repository::discover_repository_or_fallback(target_root);
        let base = vox_repository::external_repo_cache_root_under(&home, &repo.repository_id)
            .join("cache")
            .join("openwiki");
        return (base.join("wiki"), base.join("manifest.v1.json"));
    }
    (
        target_root.join("docs").join("wiki"),
        target_root
            .join(".vox")
            .join("cache")
            .join("openwiki")
            .join("manifest.v1.json"),
    )
}

fn parse_openwiki_region_ids(markdown: &str) -> Vec<String> {
    let mut ids = Vec::new();
    for line in markdown.lines() {
        let t = line.trim();
        if let Some(rest) = t
            .strip_prefix("<!--")
            .and_then(|s| s.strip_suffix("-->"))
            .map(str::trim)
            .and_then(|s| s.strip_prefix("openwiki:auto:start"))
            .map(str::trim)
            .and_then(|s| s.strip_prefix("id="))
            .map(str::trim)
            && let Some(q) = rest.chars().next().filter(|c| *c == '"' || *c == '\'')
            && let Some(end) = rest[1..].find(q)
        {
            let id = rest[1..1 + end].trim();
            if !id.is_empty() {
                ids.push(id.to_string());
            }
        }
    }
    ids
}

fn inspect_openwiki_summary(target_root: &std::path::Path, read_only: bool) -> serde_json::Value {
    let (wiki_dir, manifest_path) = resolve_openwiki_paths(target_root, read_only);
    let manifest_raw = std::fs::read_to_string(&manifest_path)
        .ok()
        .and_then(|txt| serde_json::from_str::<serde_json::Value>(&txt).ok());
    let mut existing_pages = 0usize;
    let mut auto_region_count = 0usize;
    let mut legacy_mermaid_blocks = 0usize;
    let mut stale_pages = Vec::new();
    let mut page_reports = Vec::new();

    for (rel, _, _, _) in CANONICAL_OPENWIKI_PAGES {
        let p = wiki_dir.join(rel);
        let exists = p.is_file();
        let (regions, mermaid_count, content_hash) =
            if exists && let Ok(text) = std::fs::read_to_string(&p) {
                existing_pages += 1;
                let regs = parse_openwiki_region_ids(&text);
                auto_region_count += regs.len();
                let legacy = vox_diagram::fence::extract(&text, "mermaid").len();
                legacy_mermaid_blocks += legacy;
                (
                    regs,
                    legacy,
                    Some(vox_crypto::hash_fast_hex(text.as_bytes())),
                )
            } else {
                stale_pages.push((*rel).to_string());
                (Vec::new(), 0, None)
            };
        if exists
            && let Some(ref m) = manifest_raw
            && let Some(pages) = m.get("pages").and_then(|v| v.as_array())
            && let Some(recorded) = pages
                .iter()
                .find(|entry| entry.get("rel_path").and_then(|v| v.as_str()) == Some(*rel))
            && let (Some(rec_h), Some(cur_h)) = (
                recorded.get("content_hash").and_then(|v| v.as_str()),
                content_hash.as_deref(),
            )
            && rec_h != cur_h
            && !stale_pages.iter().any(|s| s == rel)
        {
            stale_pages.push((*rel).to_string());
        }
        page_reports.push(serde_json::json!({
            "rel_path": rel,
            "exists": exists,
            "region_ids": regions,
            "legacy_mermaid_blocks": mermaid_count,
        }));
    }

    serde_json::json!({
        "wiki_dir": wiki_dir.display().to_string(),
        "manifest_path": manifest_path.display().to_string(),
        "manifest_present": manifest_path.is_file(),
        "initialized": existing_pages == CANONICAL_OPENWIKI_PAGES.len(),
        "page_count": existing_pages,
        "total_pages": CANONICAL_OPENWIKI_PAGES.len(),
        "auto_region_count": auto_region_count,
        "legacy_mermaid_blocks": legacy_mermaid_blocks,
        "stale_pages": stale_pages,
        "pages": page_reports,
    })
}

fn execute_openwiki_mcp_action(
    action: &str,
    target_root: &std::path::Path,
    read_only: bool,
) -> Result<serde_json::Value, String> {
    let (wiki_dir, manifest_path) = resolve_openwiki_paths(target_root, read_only);
    if matches!(action, "wiki_status" | "wiki_diff") {
        let mut summary = inspect_openwiki_summary(target_root, read_only);
        if let Some(obj) = summary.as_object_mut() {
            obj.insert("action".to_string(), serde_json::json!(action));
            let has_changes = obj
                .get("stale_pages")
                .and_then(|v| v.as_array())
                .is_some_and(|a| !a.is_empty());
            obj.insert("has_changes".to_string(), serde_json::json!(has_changes));
        }
        return Ok(summary);
    }

    if !read_only
        && !target_root
            .join("docs")
            .join("wiki")
            .join("README.md")
            .is_file()
    {
        let _ = vox_repository::onboard_repository_harness(target_root, false);
    }
    std::fs::create_dir_all(&wiki_dir)
        .map_err(|e| format!("failed to create {}: {e}", wiki_dir.display()))?;
    if let Some(parent) = manifest_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("failed to create {}: {e}", parent.display()))?;
    }

    let repo_ctx = vox_repository::discover_repository_or_fallback(target_root);
    let repo_name = target_root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "repository".to_string());

    let mut manifest_pages = Vec::new();
    for (rel, layer_title, topic, domain) in CANONICAL_OPENWIKI_PAGES {
        let page_path = wiki_dir.join(rel);
        let region_id = rel
            .trim_end_matches(".md")
            .replace('.', "-")
            .to_ascii_lowercase();
        let region_key = if *rel == "README.md" {
            "repo-overview".to_string()
        } else {
            region_id
        };
        if !page_path.is_file() {
            let diagram = format!(
                "graph {{\n    direction: down\n    node hub: \"{repo_name} — {layer_title}\"\n}}\n"
            );
            let md = format!(
                "---\n\
                 title: \"{repo_name} — {layer_title}\"\n\
                 description: \"OpenWiki {layer_title} documentation surface for {repo_name}.\"\n\
                 category: \"Architecture SSOTs\"\n\
                 status: \"current\"\n\
                 topic: \"{topic}\"\n\
                 domain: \"{domain}\"\n\
                 authorship: \"llm-generated\"\n\
                 authorship_model: \"vox-openwiki-v7.9\"\n\
                 training_eligible: false\n\
                 ---\n\n\
                 # {repo_name} — {layer_title}\n\n\
                 <!-- kb:summary -->\n\
                 This OpenWiki layer page catalogs {layer_title} across {repo_name} including deterministic structural facts, source coordinates, and validated diagram representations inside auto-fences so human architecture rationale remains completely untouched while automated rebuilds refresh structural metadata and contract inventory cleanly across updates.\n\
                 <!-- /kb:summary -->\n\n\
                 <!-- openwiki:auto:start id=\"{region_key}\" -->\n\
                 - **Repository ID:** `{}`\n\n\
                 ```vox-diagram\n\
                 {diagram}\
                 ```\n\
                 <!-- openwiki:auto:end id=\"{region_key}\" -->\n\n\
                 ## Human Architecture Notes\n\n\
                 Document human rationale and caveats outside `openwiki:auto` fences.\n",
                repo_ctx.repository_id
            );
            std::fs::write(&page_path, &md)
                .map_err(|e| format!("failed to write {}: {e}", page_path.display()))?;
        }
        let content = std::fs::read_to_string(&page_path)
            .map_err(|e| format!("failed to read {}: {e}", page_path.display()))?;
        manifest_pages.push(serde_json::json!({
            "rel_path": rel,
            "content_hash": vox_crypto::hash_fast_hex(content.as_bytes()),
            "region_ids": parse_openwiki_region_ids(&content),
        }));
    }

    let manifest = serde_json::json!({
        "schema_version": 1,
        "repo_name": repo_name,
        "manifest_hash": vox_crypto::hash_fast_hex(
            serde_json::to_string(&manifest_pages).unwrap_or_default().as_bytes()
        ),
        "pages": manifest_pages,
    });
    std::fs::write(
        &manifest_path,
        serde_json::to_string_pretty(&manifest).unwrap_or_default(),
    )
    .map_err(|e| format!("failed to write {}: {e}", manifest_path.display()))?;

    let mut summary = inspect_openwiki_summary(target_root, read_only);
    if let Some(obj) = summary.as_object_mut() {
        obj.insert("action".to_string(), serde_json::json!(action));
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
        "wiki_init" | "wiki_build" | "wiki_status" | "wiki_diff" => {
            execute_openwiki_mcp_action(action, target_root, read_only)
        }
        "wiki_sync" | "wiki_heal_tier1" => {
            let mut res = execute_openwiki_mcp_action("wiki_build", target_root, read_only)?;
            if let Some(obj) = res.as_object_mut() {
                obj.insert("action".to_string(), serde_json::json!(action));
                obj.insert("llm_calls".to_string(), serde_json::json!(0));
            }
            Ok(res)
        }
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
                "doc_path": "docs/wiki/interfaces-and-apis.md",
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
            std::fs::read_to_string(dir.join("docs/wiki/interfaces-and-apis.md"))
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
            std::fs::read_to_string(dir.join("docs/wiki/interfaces-and-apis.md"))
                .unwrap()
                .contains("status: \"legacy\"")
        );

        let _ = std::fs::remove_dir_all(&dir);
    }
}
