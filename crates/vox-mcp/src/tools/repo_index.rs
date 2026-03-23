//! Lightweight repository file index for MCP (path counts + optional JSON cache).

use serde::Serialize;
use std::fs;
use walkdir::WalkDir;

use crate::params::ToolResult;
use crate::server::ServerState;
use vox_orchestrator::AgentId;

const MAX_FILES_WALKED: usize = 50_000;

/// Bounded repository walk statistics for MCP clients and on-disk cache files.
#[derive(Debug, Serialize)]
pub struct RepoIndexSummary {
    /// Stable blake3-based id from `vox_repository`.
    pub repository_id: String,
    /// Absolute/resolved root path string.
    pub root: String,
    /// Files visited (excluding skipped dirs).
    pub files_scanned: usize,
    /// True when the walker stopped after scanning 50,000 files (hard cap in this module).
    pub stopped_at_cap: bool,
    /// Top file extensions by count (trimmed list).
    pub by_extension_top: Vec<(String, usize)>,
    /// Number of SKILL.md files discovered.
    pub skills_discovered: usize,
    /// Number of workflows discovered (`workflows/**/*.md` or `*.yaml`).
    pub workflows_discovered: usize,
}

fn index_cache_path(state: &ServerState) -> std::path::PathBuf {
    vox_config::repo_tooling_cache_dir(&state.repository.root, &state.repository.repository_id)
        .join("repo_index.json")
}

fn build_summary(state: &ServerState) -> Result<RepoIndexSummary, String> {
    let root = &state.repository.root;
    if !root.is_dir() {
        return Err(format!(
            "repository root is not a directory: {}",
            root.display()
        ));
    }

    let mut files_scanned = 0usize;
    let mut ext_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut stopped = false;
    let mut skills_discovered = 0usize;
    let mut workflows_discovered = 0usize;

    for ent in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_str().unwrap_or("");
            name != ".git" && name != "target" && name != "node_modules"
        })
        .filter_map(Result::ok)
    {
        if files_scanned >= MAX_FILES_WALKED {
            stopped = true;
            break;
        }
        let p = ent.path();
        if p == root.as_path() {
            continue;
        }
        if ent.file_type().is_file() {
            files_scanned += 1;
            let ext = p
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_string();
                
            let name_str = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name_str == "SKILL.md" {
                skills_discovered += 1;
            } else if p.components().any(|c| {
                let s = c.as_os_str().to_str().unwrap_or("");
                s == "workflows" || s == "_workflows" || s == ".agents" || s == "_agents"
            }) && (ext == "md" || ext == "yaml" || ext == "yml") {
                workflows_discovered += 1;
            }
                
            let key = if ext.is_empty() {
                "(no ext)".to_string()
            } else {
                format!(".{ext}")
            };
            *ext_counts.entry(key).or_insert(0) += 1;
        }
    }

    let mut pairs: Vec<(String, usize)> = ext_counts.into_iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(&a.1));
    pairs.truncate(24);

    Ok(RepoIndexSummary {
        repository_id: state.repository.repository_id.clone(),
        root: root.display().to_string(),
        files_scanned,
        stopped_at_cap: stopped,
        by_extension_top: pairs,
        skills_discovered,
        workflows_discovered,
    })
}

/// Walk `state.repository.root` (bounded), return JSON summary of file counts.
pub async fn repo_index_status(state: &ServerState) -> String {
    let fresh_check = {
        let orch = state.orchestrator.lock().await;
        if orch.context_store().is_fresh("workspace_index_status", 30) {
            let entry = orch.context_store().get_entry("workspace_index_status");
            entry.map(|e| e.value)
        } else {
            None
        }
    };
    if let Some(cached) = fresh_check {
        return cached;
    }

    let summary = match build_summary(state) {
        Ok(s) => ToolResult::ok(s).to_json(),
        Err(e) => ToolResult::<RepoIndexSummary>::err(e).to_json(),
    };
    
    let orch = state.orchestrator.lock().await;
    orch.context_store().set(
        vox_orchestrator::AgentId(0),
        "workspace_index_status",
        summary.clone(),
        30
    );
    summary
}

/// Refresh on-disk cache under `.vox/cache/repos/<repository_id>/repo_index.json`.
pub async fn repo_index_refresh(state: &ServerState) -> String {
    let fresh_check = {
        let orch = state.orchestrator.lock().await;
        if orch.context_store().is_fresh("workspace_index_refresh", 30) {
            let entry = orch.context_store().get_entry("workspace_index_refresh");
            entry.map(|e| e.value)
        } else {
            None
        }
    };
    if let Some(cached) = fresh_check {
        return cached;
    }

    let summary = match build_summary(state) {
        Ok(s) => s,
        Err(e) => return ToolResult::<String>::err(e).to_json(),
    };
    let path = index_cache_path(state);
    if let Some(parent) = path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            return ToolResult::<String>::err(format!("mkdir {}: {e}", parent.display())).to_json();
        }
    }
    let text = match serde_json::to_string_pretty(&summary) {
        Ok(t) => t,
        Err(e) => return ToolResult::<String>::err(format!("serialize: {e}")).to_json(),
    };
    let res = match fs::write(&path, &text) {
        Ok(()) => ToolResult::ok(format!("wrote {}", path.display())).to_json(),
        Err(e) => ToolResult::<String>::err(format!("write {}: {e}", path.display())).to_json(),
    };
    
    let orch = state.orchestrator.lock().await;
    orch.context_store().set(
        vox_orchestrator::AgentId(0),
        "workspace_index_refresh",
        res.clone(),
        30
    );
    res
}
