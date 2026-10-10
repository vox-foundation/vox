//! Harness issue, fix-proposal and eval routes and ops for Axis (plan 07 Tasks 7, 10): the
//! golden-corpus staleness scanner, issue listing / decisions, and LLM fix proposals that are
//! written to disk only after approval.
use std::net::SocketAddr;
use std::path::{Path as FsPath, PathBuf};

use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use chrono::Datelike as _;
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

use crate::gui_ops::dto::harness::{
    CategorySummaryDto, HarnessEvalRunDto, HarnessFixProposalDto, HarnessIssueDto,
    RegressionFlagDto,
};
use crate::gui_ops::{api_db, enforce_route, require_db};
use crate::http_gateway::GatewayState;
use crate::params::ToolResult;
use crate::server_state::ServerState;
use crate::services::routes::{err, ok};

const HARNESS_SURFACES: &[&str] = &["scientia"];
const LIST_LIMIT: i64 = 200;
const STALENESS_THRESHOLD_DAYS: i64 = 90;

// ─── Corpus staleness ────────────────────────────────────────────────────────

/// The same `// vox:skip` opt-out `examples_golden_doctor_green.rs` honors.
fn is_skipped(src: &str) -> bool {
    src.lines()
        .next()
        .is_some_and(|line| line.trim_start().starts_with("// vox:skip"))
}

fn extract_frontmatter_field(content: &str, key: &str) -> Option<String> {
    let prefix = format!("// {key}: ");
    content
        .lines()
        .find(|line| line.trim_start().starts_with(&prefix))
        .map(|line| {
            line.trim_start()[prefix.len()..]
                .trim()
                .trim_matches('"')
                .to_string()
        })
}

/// Days elapsed from a `YYYY-MM-DD` date to `today` `(year, month, day)`; `None` if unparseable.
fn days_since(date_str: &str, today: (i32, u32, u32)) -> Option<i64> {
    let parts: Vec<&str> = date_str.splitn(3, '-').collect();
    if parts.len() != 3 {
        return None;
    }
    let then = chrono::NaiveDate::from_ymd_opt(
        parts[0].parse().ok()?,
        parts[1].parse().ok()?,
        parts[2].parse().ok()?,
    )?;
    let now = chrono::NaiveDate::from_ymd_opt(today.0, today.1, today.2)?;
    Some((now - then).num_days())
}

/// A staleness finding, before it's persisted as a `scientia_harness_issues` row.
pub struct StalenessFinding {
    pub target_path: String,
    pub summary: String,
}

/// Check one golden file's `last_validated` frontmatter for staleness. `today_ymd` is injected
/// so this stays pure.
pub fn check_staleness(
    path: &str,
    content: &str,
    today_ymd: (i32, u32, u32),
) -> Option<StalenessFinding> {
    if is_skipped(content) {
        return None;
    }
    let last_validated = extract_frontmatter_field(content, "last_validated")?;
    let age_days = days_since(&last_validated, today_ymd)?;
    (age_days > STALENESS_THRESHOLD_DAYS).then(|| StalenessFinding {
        target_path: path.to_string(),
        summary: format!(
            "last_validated {last_validated} is {age_days} days old (threshold {STALENESS_THRESHOLD_DAYS})"
        ),
    })
}

/// Scan `examples/golden/*.vox` under `repo_root`, insert one pending `corpus_scan` issue per new
/// finding (skipping files that already have one), and return how many rows were inserted.
pub async fn scan_corpus(db: &vox_db::VoxDb, repo_root: &FsPath) -> Result<usize, String> {
    let golden_dir = repo_root.join("examples").join("golden");
    let mut entries = tokio::fs::read_dir(&golden_dir)
        .await
        .map_err(|e| format!("read_dir {}: {e}", golden_dir.display()))?;
    let today = chrono::Utc::now().date_naive();
    let today_ymd = (today.year(), today.month(), today.day());
    let mut inserted = 0usize;
    while let Some(entry) = entries.next_entry().await.map_err(|e| e.to_string())? {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("vox") {
            continue;
        }
        let Ok(content) = tokio::fs::read_to_string(&path).await else {
            continue;
        };
        let rel_path = vox_repository::path_relative_to_repo_root(repo_root, &path)
            .unwrap_or_else(|_| path.to_string_lossy().replace('\\', "/"));
        let Some(finding) = check_staleness(&rel_path, &content, today_ymd) else {
            continue;
        };
        if db
            .has_pending_harness_issue("corpus_scan", &finding.target_path, "stale_frontmatter")
            .await
            .map_err(|e| e.to_string())?
        {
            continue;
        }
        let evidence_json = serde_json::json!({ "path": finding.target_path }).to_string();
        db.insert_harness_issue(vox_db::NewHarnessIssue {
            source: "corpus_scan",
            session_key: None,
            target_path: Some(&finding.target_path),
            detected_at_ms: chrono::Utc::now().timestamp_millis(),
            category: "stale_frontmatter",
            severity: "low",
            summary: &finding.summary,
            evidence_json: &evidence_json,
        })
        .await
        .map_err(|e| e.to_string())?;
        inserted += 1;
    }
    Ok(inserted)
}

// ─── Fix proposals ───────────────────────────────────────────────────────────

/// Resolve `target_path` strictly under `repo_root`, requiring the file to exist. Used by both
/// propose and apply so traversal or an absolute path elsewhere on disk is refused.
fn resolve_target_path(repo_root: &FsPath, target_path: &str) -> Result<PathBuf, String> {
    // `resolve_local_path_under_repo_root` canonicalizes, which needs the file to exist; report a
    // deleted/renamed file as missing rather than as a traversal refusal.
    if !repo_root.join(target_path).exists() {
        return Err(format!("target file does not exist: {target_path}"));
    }
    vox_repository::resolve_local_path_under_repo_root(repo_root, target_path)
        .map_err(|e| format!("refusal: target_path resolves outside the repository root ({e})"))
}

/// Unified diff of a proposal, for display only — never parsed back into content.
pub fn build_unified_diff(target_path: &str, old: &str, new: &str) -> String {
    similar::TextDiff::from_lines(old, new)
        .unified_diff()
        .context_radius(3)
        .header(target_path, target_path)
        .to_string()
}

/// Ask the model-agnostic LLM facade for a corrected version of `target_path` (which must match
/// the issue's own stored target) and persist it as a pending fix proposal.
pub async fn propose_fix(
    db: &vox_db::VoxDb,
    repo_root: &FsPath,
    issue_id: i64,
    target_path: &str,
) -> Result<i64, String> {
    let issue = db
        .get_harness_issue(issue_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("no harness issue with id {issue_id}"))?;
    // A caller-supplied path must match the issue's persisted target, or any real issue id could
    // be used to propose (and later write) an unrelated file.
    let issue_target_path = issue
        .target_path
        .as_deref()
        .ok_or_else(|| format!("harness issue {issue_id} has no target_path"))?;
    if issue_target_path != target_path {
        return Err(format!(
            "target_path {target_path:?} does not match harness issue {issue_id}'s stored target_path {issue_target_path:?}"
        ));
    }
    let full_path = resolve_target_path(repo_root, target_path)?;
    let old_content = tokio::fs::read_to_string(&full_path)
        .await
        .map_err(|e| format!("read {}: {e}", full_path.display()))?;

    let prompt = format!(
        "The following Vox source file has an issue: {}\n\nCurrent content:\n{}\n\n\
         Propose a corrected version of the ENTIRE file. Respond with ONLY the corrected \
         file content, no explanation, no markdown fences.",
        issue.summary, old_content
    );
    let messages = vec![vox_actor_runtime::llm::LlmChatMessage {
        role: "user".into(),
        content: prompt,
        ..Default::default()
    }];
    let model_id = vox_orchestrator::models::select_with_default_registry(
        &vox_orchestrator::models::SelectionIntent::repair_loop(),
    )
    .map(|o| o.model_id)
    .unwrap_or_else(|| "google/gemini-3.1-pro".to_string());
    let mut llm_config = vox_actor_runtime::llm::LlmConfig::openrouter(&model_id);
    llm_config.temperature = Some(0.0);
    llm_config.max_tokens = Some(2048);
    llm_config.timeout_ms = Some(30_000);
    llm_config.telemetry_task_category = Some("HarnessIssueFixDispatch".into());
    llm_config.telemetry_attempt_number = Some(1);
    let activity_options = vox_actor_runtime::ActivityOptions::default()
        // drift-allow(timeout-literal,duplicate-numeric-literal): local wait/poll bound, not an HTTP request timeout; a shared constant needs a vox-config edge
        .with_timeout(std::time::Duration::from_secs(30));
    let new_content = match vox_actor_runtime::llm::infer_with_retry(
        &activity_options,
        messages,
        vec![llm_config],
    )
    .await
    {
        vox_actor_runtime::ActivityResult::Ok(Ok((resp, _cfg))) => resp.content,
        other => return Err(format!("fix-dispatch LLM call failed: {other:?}")),
    };
    // Coarse truncation guard: the proposal becomes the sole content written on approval, so an
    // empty response is rejected outright.
    if new_content.trim().is_empty() {
        return Err("fix-dispatch LLM call returned empty content".to_string());
    }
    let diff = build_unified_diff(target_path, &old_content, &new_content);
    db.insert_harness_fix_proposal(vox_db::NewFixProposal {
        issue_id,
        target_path,
        proposed_content: &new_content,
        proposed_diff: &diff,
        proposed_at_ms: chrono::Utc::now().timestamp_millis(),
    })
    .await
    .map_err(|e| e.to_string())
}

/// Approve (write `proposed_content` verbatim to its target) or reject a fix proposal. The
/// proposal is claimed atomically (only from `pending_approval`) BEFORE any write, so a stale
/// approve can never apply an already-rejected proposal.
pub async fn resolve_fix(
    db: &vox_db::VoxDb,
    repo_root: &FsPath,
    proposal_id: i64,
    approve: bool,
) -> Result<(), String> {
    let resolved_at_ms = chrono::Utc::now().timestamp_millis();
    if !approve {
        return db
            .resolve_harness_fix_proposal(proposal_id, "rejected", resolved_at_ms)
            .await
            .map_err(|e| e.to_string());
    }
    let proposal = db
        .get_harness_fix_proposal(proposal_id)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("no fix proposal with id {proposal_id}"))?;
    db.resolve_harness_fix_proposal(proposal_id, "applied", resolved_at_ms)
        .await
        .map_err(|e| e.to_string())?;
    let full_path = resolve_target_path(repo_root, &proposal.target_path)?;
    // Never anything derived from `proposed_diff` (lossy; see vox-db
    // `scientia_harness_fix_proposals`).
    tokio::fs::write(&full_path, &proposal.proposed_content)
        .await
        .map_err(|e| format!("write {}: {e}", full_path.display()))
}

// ─── Ops ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct CorpusScanParams {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FixProposeParams {
    pub issue_id: i64,
    /// Must equal the issue's stored `target_path`.
    pub target_path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FixResolveParams {
    pub proposal_id: i64,
    pub approve: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct IssueDecideParams {
    pub issue_id: i64,
    /// `confirmed` | `dismissed`.
    pub decision: String,
    #[serde(default)]
    pub reason: Option<String>,
}

pub async fn corpus_scan(state: &ServerState, _p: CorpusScanParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<usize>::err(e).to_json(),
    };
    match scan_corpus(db, &state.repository.root).await {
        Ok(n) => ToolResult::ok(n).to_json(),
        Err(e) => ToolResult::<usize>::err(e).to_json(),
    }
}

pub async fn fix_propose(state: &ServerState, p: FixProposeParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<i64>::err(e).to_json(),
    };
    match propose_fix(db, &state.repository.root, p.issue_id, &p.target_path).await {
        Ok(id) => ToolResult::ok(id).to_json(),
        Err(e) => ToolResult::<i64>::err(e).to_json(),
    }
}

pub async fn fix_resolve(state: &ServerState, p: FixResolveParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<()>::err(e).to_json(),
    };
    match resolve_fix(db, &state.repository.root, p.proposal_id, p.approve).await {
        Ok(()) => ToolResult::ok(()).to_json(),
        Err(e) => ToolResult::<()>::err(e).to_json(),
    }
}

pub async fn issue_decide(state: &ServerState, p: IssueDecideParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<()>::err(e).to_json(),
    };
    let row = vox_db::HarnessIssueDecisionRow {
        issue_id: p.issue_id,
        decision: p.decision,
        actor: vox_config::local_user_id(),
        reason: p.reason,
        decided_at_ms: chrono::Utc::now().timestamp_millis(),
    };
    match db.record_harness_issue_decision(&row).await {
        Ok(()) => ToolResult::ok(()).to_json(),
        Err(e) => ToolResult::<()>::err(e.to_string()).to_json(),
    }
}

// ─── Routes ──────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct IssuesQuery {
    pub status: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProposalsQuery {
    pub status: Option<String>,
}

/// GET /api/v2/harness/fix-proposals?status=
pub async fn get_fix_proposals(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<ProposalsQuery>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, HARNESS_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    match db
        .list_harness_fix_proposals(q.status.as_deref(), LIST_LIMIT)
        .await
    {
        Ok(rows) => ok(rows
            .into_iter()
            .map(HarnessFixProposalDto::from)
            .collect::<Vec<_>>()),
        Err(e) => err("db_error", &e.to_string()),
    }
}

/// GET /api/v2/harness/issues?status=&source=
pub async fn get_issues(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<IssuesQuery>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, HARNESS_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    match db
        .list_harness_issues(q.status.as_deref(), q.source.as_deref(), LIST_LIMIT)
        .await
    {
        Ok(rows) => ok(rows
            .into_iter()
            .map(HarnessIssueDto::from)
            .collect::<Vec<_>>()),
        Err(e) => err("db_error", &e.to_string()),
    }
}

/// GET /api/v2/harness/issues/by-session/{key}
pub async fn get_issues_for_session(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(session_key): Path<String>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, HARNESS_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    match db.list_harness_issues_for_session(&session_key).await {
        Ok(rows) => ok(rows
            .into_iter()
            .map(HarnessIssueDto::from)
            .collect::<Vec<_>>()),
        Err(e) => err("db_error", &e.to_string()),
    }
}

// ─── Eval history & regressions (Harness Health) ─────────────────────────────

const HARNESS_EVAL_SURFACES: &[&str] = &["harness-health"];
const EVAL_HISTORY_DEFAULT_LIMIT: usize = 50;

#[derive(Debug, Deserialize)]
pub struct EvalHistoryQuery {
    pub limit: Option<usize>,
}

async fn eval_history(
    db: &vox_db::VoxDb,
    limit: usize,
) -> Result<Vec<HarnessEvalRunDto>, vox_db::StoreError> {
    let runs = db.list_harness_eval_runs(limit).await?;
    let run_ids: Vec<String> = runs.iter().map(|r| r.run_id.clone()).collect();
    let mut task_results_by_run = db.get_harness_eval_task_results_for_runs(&run_ids).await?;
    Ok(runs
        .into_iter()
        .map(|r| {
            let task_results = task_results_by_run.remove(&r.run_id).unwrap_or_default();
            let mut by_category: std::collections::BTreeMap<String, (i64, i64)> =
                std::collections::BTreeMap::new();
            for t in &task_results {
                let entry = by_category.entry(t.category.clone()).or_default();
                if t.status == "pass" {
                    entry.0 += 1;
                } else if t.status == "fail" {
                    entry.1 += 1;
                }
            }
            HarnessEvalRunDto {
                run_id: r.run_id,
                git_sha: r.git_sha,
                triggered_by: r.triggered_by,
                pass_count: r.pass_count,
                fail_count: r.fail_count,
                skip_count: r.skip_count,
                total_cost_usd: r.total_cost_usd,
                started_at_ms: r.started_at_ms,
                category_breakdown: by_category
                    .into_iter()
                    .map(|(category, (pass_count, fail_count))| CategorySummaryDto {
                        category,
                        pass_count,
                        fail_count,
                    })
                    .collect(),
            }
        })
        .collect())
}

/// The stored SHAs become a `git diff` argument, so anything that isn't a lowercase hex SHA
/// (e.g. a `--output=` flag) is refused before the subprocess is built.
fn is_valid_git_sha(s: &str) -> bool {
    (7..=40).contains(&s.len())
        && s.chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

/// Compare the two most recent runs; empty when fewer than two exist.
async fn eval_regressions(
    db: &vox_db::VoxDb,
    repo_root: &FsPath,
) -> Result<Vec<RegressionFlagDto>, String> {
    let runs = db
        .list_harness_eval_runs(2)
        .await
        .map_err(|e| e.to_string())?;
    if runs.len() < 2 {
        return Ok(vec![]);
    }
    let (current, previous) = (&runs[0], &runs[1]);
    if !is_valid_git_sha(&previous.git_sha) || !is_valid_git_sha(&current.git_sha) {
        return Err(format!(
            "refusing to run git diff with a malformed git_sha (previous={:?}, current={:?})",
            previous.git_sha, current.git_sha
        ));
    }
    let map = |e: vox_db::StoreError| e.to_string();
    let previous_task_results = db
        .get_harness_eval_task_results(&previous.run_id)
        .await
        .map_err(map)?;
    let current_task_results = db
        .get_harness_eval_task_results(&current.run_id)
        .await
        .map_err(map)?;
    let previous_events = db
        .get_model_selection_events(&previous.run_id)
        .await
        .map_err(map)?;
    let current_events = db
        .get_model_selection_events(&current.run_id)
        .await
        .map_err(map)?;
    let range = format!("{}..{}", previous.git_sha, current.git_sha);
    let changed_files: Vec<String> = crate::git_exec::GitExec::new(repo_root)
        .run(&["diff", "--name-only", &range, "--"])
        .await
        .map(|out| out.stdout.lines().map(str::to_string).collect())
        .unwrap_or_default();
    Ok(vox_db::detect_regressions(
        previous,
        current,
        &previous_task_results,
        &current_task_results,
        &previous_events,
        &current_events,
        &changed_files,
    )
    .into_iter()
    .map(|f| RegressionFlagDto {
        kind: format!("{:?}", f.kind),
        previous_run_id: f.previous_run_id,
        current_run_id: f.current_run_id,
        previous_git_sha: f.previous_git_sha,
        current_git_sha: f.current_git_sha,
        changed_files: f.changed_files,
        flipped_task_ids: f.flipped_task_ids,
        detail: f.detail,
    })
    .collect())
}

/// GET /api/v2/harness/eval/history?limit=
pub async fn get_eval_history(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<EvalHistoryQuery>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, HARNESS_EVAL_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    match eval_history(&db, q.limit.unwrap_or(EVAL_HISTORY_DEFAULT_LIMIT)).await {
        Ok(rows) => ok(rows),
        Err(e) => err("db_error", &e.to_string()),
    }
}

/// GET /api/v2/harness/eval/regressions
pub async fn get_eval_regressions(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, HARNESS_EVAL_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    match eval_regressions(&db, &gs.server_state().repository.root).await {
        Ok(rows) => ok(rows),
        Err(e) => err("harness_eval_error", &e),
    }
}

pub fn api_v2_router() -> Router<GatewayState> {
    Router::new()
        .route("/harness/fix-proposals", get(get_fix_proposals))
        .route("/harness/issues", get(get_issues))
        .route("/harness/eval/history", get(get_eval_history))
        .route("/harness/eval/regressions", get(get_eval_regressions))
        .route(
            "/harness/issues/by-session/{key}",
            get(get_issues_for_session),
        )
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::gui_ops::test_support::{body, gateway_with_memory_db, peer, tool_json};

    #[test]
    fn router_builds() {
        let _ = api_v2_router();
    }

    #[test]
    fn staleness_threshold_skip_and_missing_field() {
        let stale = "// last_validated: 2026-01-01\nfn main() {}\n";
        let finding = check_staleness("examples/golden/x.vox", stale, (2026, 8, 2));
        assert!(finding.unwrap().summary.contains("2026-01-01"));
        let recent = "// last_validated: 2026-07-20\nfn main() {}\n";
        assert!(check_staleness("x.vox", recent, (2026, 8, 2)).is_none());
        assert!(check_staleness("x.vox", "fn main() {}\n", (2026, 8, 2)).is_none());
        let skipped = "// vox:skip out of grammar\n// last_validated: 2020-01-01\n";
        assert!(check_staleness("x.vox", skipped, (2026, 8, 2)).is_none());
    }

    #[test]
    fn unified_diff_contains_both_paths_and_changed_lines() {
        let diff = build_unified_diff("examples/golden/x.vox", "old line\n", "new line\n");
        assert!(diff.contains("examples/golden/x.vox"));
        assert!(diff.contains("-old line"));
        assert!(diff.contains("+new line"));
    }

    #[test]
    fn path_traversal_target_path_is_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let repo_root = dir.path();
        std::fs::create_dir_all(repo_root.join("examples/golden")).expect("mkdir");
        std::fs::write(repo_root.join("examples/golden/ok.vox"), "fine").expect("seed");
        assert!(resolve_target_path(repo_root, "../../etc/passwd").is_err());
        let outside = tempfile::tempdir().expect("other tempdir");
        let outside_file = outside.path().join("elsewhere.vox");
        std::fs::write(&outside_file, "x").expect("seed outside file");
        assert!(resolve_target_path(repo_root, outside_file.to_str().unwrap()).is_err());
        assert!(resolve_target_path(repo_root, "examples/golden/ok.vox").is_ok());
    }

    #[tokio::test]
    async fn scan_inserts_once_then_routes_list_and_decision_confirms() {
        let dir = tempfile::tempdir().expect("tempdir");
        let golden = dir.path().join("examples/golden");
        std::fs::create_dir_all(&golden).unwrap();
        std::fs::write(golden.join("x.vox"), "// last_validated: 2020-01-01\n").unwrap();
        let (gs, db) = gateway_with_memory_db().await;
        assert_eq!(scan_corpus(&db, dir.path()).await.unwrap(), 1);
        assert_eq!(
            scan_corpus(&db, dir.path()).await.unwrap(),
            0,
            "a pending finding is not duplicated"
        );

        let issues = body(
            get_issues(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Query(IssuesQuery {
                    status: Some("pending".into()),
                    source: Some("corpus_scan".into()),
                }),
            )
            .await,
        )
        .await;
        let rows = issues["data"].as_array().expect("rows");
        assert_eq!(rows.len(), 1, "{issues}");
        assert_eq!(rows[0]["target_path"], "examples/golden/x.vox");
        let issue_id = rows[0]["id"].as_i64().unwrap();

        let decided = tool_json(
            &issue_decide(
                gs.server_state(),
                IssueDecideParams {
                    issue_id,
                    decision: "confirmed".into(),
                    reason: None,
                },
            )
            .await,
        );
        assert_eq!(decided["success"], true, "{decided}");
        assert_eq!(
            db.get_harness_issue(issue_id)
                .await
                .unwrap()
                .unwrap()
                .status,
            "confirmed"
        );
    }

    #[tokio::test]
    async fn approve_writes_proposed_content_verbatim_and_a_resolved_proposal_cannot_apply() {
        let dir = tempfile::tempdir().expect("tempdir");
        let golden = dir.path().join("examples/golden");
        std::fs::create_dir_all(&golden).unwrap();
        let target = golden.join("x.vox");
        std::fs::write(&target, "line one\nline two\n").unwrap();
        let db = vox_db::VoxDb::connect(vox_db::DbConfig::Memory)
            .await
            .unwrap();
        let issue_id = db
            .insert_harness_issue(vox_db::NewHarnessIssue {
                source: "corpus_scan",
                session_key: None,
                target_path: Some("examples/golden/x.vox"),
                detected_at_ms: 1,
                category: "stale_frontmatter",
                severity: "low",
                summary: "s",
                evidence_json: "{}",
            })
            .await
            .unwrap();
        let new_content = "line one\nCHANGED\nline three\n";
        let proposal = |content: &'static str| vox_db::NewFixProposal {
            issue_id,
            target_path: "examples/golden/x.vox",
            proposed_content: content,
            proposed_diff: "(display only)",
            proposed_at_ms: 2,
        };
        let approved = db
            .insert_harness_fix_proposal(proposal(new_content))
            .await
            .unwrap();
        resolve_fix(&db, dir.path(), approved, true)
            .await
            .expect("approve");
        assert_eq!(std::fs::read_to_string(&target).unwrap(), new_content);

        let rejected = db
            .insert_harness_fix_proposal(proposal("STALE\n"))
            .await
            .unwrap();
        resolve_fix(&db, dir.path(), rejected, false)
            .await
            .expect("reject");
        assert!(resolve_fix(&db, dir.path(), rejected, true).await.is_err());
        assert_eq!(std::fs::read_to_string(&target).unwrap(), new_content);
    }

    #[tokio::test]
    async fn propose_refuses_a_path_that_is_not_the_issue_target() {
        let (gs, db) = gateway_with_memory_db().await;
        let issue_id = db
            .insert_harness_issue(vox_db::NewHarnessIssue {
                source: "corpus_scan",
                session_key: None,
                target_path: Some("examples/golden/x.vox"),
                detected_at_ms: 1,
                category: "stale_frontmatter",
                severity: "low",
                summary: "s",
                evidence_json: "{}",
            })
            .await
            .unwrap();
        let out = tool_json(
            &fix_propose(
                gs.server_state(),
                FixProposeParams {
                    issue_id,
                    target_path: "Cargo.toml".into(),
                },
            )
            .await,
        );
        assert!(
            out["error"].as_str().unwrap().contains("does not match"),
            "{out}"
        );
    }

    #[tokio::test]
    async fn harness_routes_are_profile_gated() {
        let named = ServerState::new_test().await.with_profile(
            crate::profiles::ActiveProfile::resolve(&["mesh"]).expect("mesh profile"),
        );
        let gs = GatewayState::for_test_with_state(named, HashSet::new()).await;
        let denied = body(
            get_fix_proposals(
                State(gs),
                peer(),
                HeaderMap::new(),
                Query(ProposalsQuery { status: None }),
            )
            .await,
        )
        .await;
        assert_eq!(denied["error"]["code"], "tool_not_in_profile", "{denied}");
    }

    #[test]
    fn is_valid_git_sha_rejects_a_dash_prefixed_value() {
        assert!(!is_valid_git_sha("--output=/tmp/evil"));
        assert!(!is_valid_git_sha("ABC1234"));
        assert!(is_valid_git_sha("abc1234"));
    }

    fn eval_run(run_id: &str, git_sha: &str, started_at_ms: i64) -> vox_db::HarnessEvalRunRecord {
        vox_db::HarnessEvalRunRecord {
            run_id: run_id.to_string(),
            triggered_by: "local".to_string(),
            git_sha: git_sha.to_string(),
            git_branch: "main".to_string(),
            changed_files: vec![],
            config_version: None,
            samples_per_task: 3,
            task_count: 10,
            pass_count: 9,
            fail_count: 1,
            skip_count: 0,
            total_cost_usd: 0.05,
            started_at_ms,
            finished_at_ms: started_at_ms + 1000,
        }
    }

    #[tokio::test]
    async fn eval_history_and_regressions_routes() {
        let (gs, db) = gateway_with_memory_db().await;
        let empty =
            body(get_eval_regressions(State(gs.clone()), peer(), HeaderMap::new()).await).await;
        assert_eq!(empty["data"], serde_json::json!([]), "{empty}");

        db.record_harness_eval_run(&eval_run("r1", "abc1234", 1000))
            .await
            .unwrap();
        let history = body(
            get_eval_history(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Query(EvalHistoryQuery { limit: None }),
            )
            .await,
        )
        .await;
        let rows = history["data"]
            .as_array()
            .unwrap_or_else(|| panic!("{history}"));
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["run_id"], "r1");
        assert_eq!(rows[0]["pass_count"], 9);

        db.record_harness_eval_run(&eval_run("r2", "--output=/tmp/evil", 2000))
            .await
            .unwrap();
        let refused = body(get_eval_regressions(State(gs), peer(), HeaderMap::new()).await).await;
        assert_eq!(refused["error"]["code"], "harness_eval_error", "{refused}");
    }
}
