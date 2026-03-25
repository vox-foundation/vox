use crate::{ServerState, ToolResult};
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use vox_publisher::templates;
use vox_publisher::types::UnifiedNewsItem;
use vox_publisher::{Publisher, PublisherConfig};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxNewsTestSyndicateParams {
    /// Markdown with YAML frontmatter matching [`UnifiedNewsItem`].
    pub content: String,
}

pub async fn vox_news_test_syndicate(
    _state: &ServerState,
    params: VoxNewsTestSyndicateParams,
) -> String {
    let item = match UnifiedNewsItem::parse(&params.content, "test-id") {
        Ok(mut it) => {
            it.syndication.dry_run = true;
            it
        }
        Err(e) => {
            return ToolResult::<String>::err(format!(
                "Failed to parse news item frontmatter: {}",
                e
            ))
            .to_json();
        }
    };

    let publisher = Publisher::new(PublisherConfig {
        twitter_bearer_token: None,
        github_token: None,
        open_collective_token: None,
        dry_run: true,
        ..Default::default()
    });

    let result = match publisher.publish_all(&item).await {
        Ok(r) => r,
        Err(e) => {
            return ToolResult::<String>::err(format!("Dry-run syndication failed: {}", e))
                .to_json();
        }
    };

    ToolResult::ok(format!(
        "Dry-run syndication OK (no live HTTP). Result: {:?}",
        result
    ))
    .to_json()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxNewsDraftResearchParams {
    /// Filename stem for `docs/news/drafts/{id}.md` (no slashes).
    pub id: String,
    pub title: String,
    pub author: String,
    pub abstract_text: String,
}

pub async fn vox_news_draft_research(
    _state: &ServerState,
    params: VoxNewsDraftResearchParams,
) -> String {
    if let Err(e) = vox_publisher::contract::validate_news_id(&params.id) {
        return ToolResult::<String>::err(e.to_string()).to_json();
    }
    let now = chrono::Utc::now();
    let draft_content = templates::render_research_update(
        &params.id,
        &params.title,
        &params.author,
        &now.to_rfc3339(),
        &params.abstract_text,
    );

    let draft_path = PathBuf::from("docs/news/drafts").join(format!("{}.md", params.id));
    if let Some(parent) = draft_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Err(e) = fs::write(&draft_path, draft_content) {
        return ToolResult::<String>::err(format!(
            "Failed to write draft to {:?}: {}",
            draft_path, e
        ))
        .to_json();
    }

    ToolResult::ok(format!("Research draft written to {:?}", draft_path)).to_json()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxNewsApproveParams {
    /// News item id (markdown filename stem without `.md`).
    pub news_id: String,
    /// Opaque approver identity (e.g. GitHub login). Two **distinct** values required for live publish.
    pub approver: String,
}

pub async fn vox_news_approve(state: &ServerState, params: VoxNewsApproveParams) -> String {
    if let Err(e) = vox_publisher::contract::validate_news_id(&params.news_id) {
        return ToolResult::<String>::err(e.to_string()).to_json();
    }
    let approver = params.approver.trim();
    if approver.is_empty() {
        return ToolResult::<String>::err("approver must not be empty".to_string()).to_json();
    }
    let Some(db) = &state.db else {
        return ToolResult::<String>::err(
            "VoxDb is not connected; cannot record approvals".to_string(),
        )
        .to_json();
    };
    match db.record_news_approval(&params.news_id, approver).await {
        Ok(()) => {}
        Err(e) => {
            return ToolResult::<String>::err(format!("DB error: {}", e)).to_json();
        }
    }
    let count = match db.count_news_approvers(&params.news_id).await {
        Ok(c) => c,
        Err(e) => {
            return ToolResult::<String>::err(format!("DB error: {}", e)).to_json();
        }
    };
    ToolResult::ok(format!(
        "Recorded approval from {:?} for news_id {:?}. Distinct approver count: {} (need 2 for live).",
        approver, params.news_id, count
    ))
    .to_json()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxNewsApprovalStatusParams {
    pub news_id: String,
}

#[derive(Debug, Serialize)]
struct ApprovalStatusBody {
    news_id: String,
    distinct_approver_count: i64,
    dual_approval_met: bool,
}

pub async fn vox_news_approval_status(
    state: &ServerState,
    params: VoxNewsApprovalStatusParams,
) -> String {
    if let Err(e) = vox_publisher::contract::validate_news_id(&params.news_id) {
        return ToolResult::<String>::err(e.to_string()).to_json();
    }
    let Some(db) = &state.db else {
        return ToolResult::<String>::err("VoxDb is not connected".to_string()).to_json();
    };
    let count = match db.count_news_approvers(&params.news_id).await {
        Ok(c) => c,
        Err(e) => return ToolResult::<String>::err(format!("DB error: {}", e)).to_json(),
    };
    let dual = match db.has_dual_news_approval(&params.news_id).await {
        Ok(b) => b,
        Err(e) => return ToolResult::<String>::err(format!("DB error: {}", e)).to_json(),
    };
    ToolResult::ok(ApprovalStatusBody {
        news_id: params.news_id,
        distinct_approver_count: count,
        dual_approval_met: dual,
    })
    .to_json()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxNewsSimulatePublishGateParams {
    /// Parsed with this id (filename stem semantics).
    pub news_id: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
struct GateReport {
    parse_ok: bool,
    validate_ok: bool,
    dual_approval_met: bool,
    orchestrator_publish_armed: bool,
    would_be_live_without_dry_run: bool,
    blocking_reasons: Vec<String>,
}

pub async fn vox_news_simulate_publish_gate(
    state: &ServerState,
    params: VoxNewsSimulatePublishGateParams,
) -> String {
    let mut reasons = Vec::new();
    let item = match UnifiedNewsItem::parse(&params.content, &params.news_id) {
        Ok(i) => i,
        Err(e) => {
            reasons.push(format!("parse: {}", e));
            return ToolResult::ok(GateReport {
                parse_ok: false,
                validate_ok: false,
                dual_approval_met: false,
                orchestrator_publish_armed: state.orchestrator_config.news.publish_armed,
                would_be_live_without_dry_run: false,
                blocking_reasons: reasons,
            })
            .to_json();
        }
    };

    let validate_ok = match item.validate() {
        Ok(()) => true,
        Err(e) => {
            reasons.push(format!("validate: {}", e));
            false
        }
    };

    let would_live = !state.orchestrator_config.news.dry_run && !item.syndication.dry_run;

    let dual = if let Some(db) = &state.db {
        db.has_dual_news_approval(&params.news_id)
            .await
            .unwrap_or(false)
    } else {
        reasons.push("no VoxDb: cannot verify approvals".into());
        false
    };

    let armed = state.orchestrator_config.news.publish_armed
        || std::env::var("VOX_NEWS_PUBLISH_ARMED")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

    if would_live && !dual {
        reasons.push("need two distinct approvers (vox_news_approve)".into());
    }
    if would_live && !armed {
        reasons.push("publish not armed: set [orchestrator.news].publish_armed=true or VOX_NEWS_PUBLISH_ARMED=1".into());
    }
    if would_live && state.db.is_none() {
        reasons.push("live publish requires VoxDb".into());
    }

    ToolResult::ok(GateReport {
        parse_ok: true,
        validate_ok,
        dual_approval_met: dual,
        orchestrator_publish_armed: armed,
        would_be_live_without_dry_run: would_live,
        blocking_reasons: reasons,
    })
    .to_json()
}
