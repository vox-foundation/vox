use crate::{ServerState, ToolResult};
use schemars::JsonSchema;
use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use vox_publisher::gate::{GateReason, PublishGateInputs, evaluate_publish_gate};
use vox_publisher::templates;
use vox_publisher::types::UnifiedNewsItem;
use vox_publisher::{Publisher, PublisherConfig};

fn read_news_markdown_first(paths: &[PathBuf; 2]) -> Result<String, String> {
    crate::bounded_fs::read_utf8_path_capped(&paths[0])
        .or_else(|e1| {
            crate::bounded_fs::read_utf8_path_capped(&paths[1]).map_err(|e2| {
                format!(
                    "Could not read {} ({}); alternate {} ({})",
                    paths[0].display(),
                    e1,
                    paths[1].display(),
                    e2
                )
            })
        })
        .map_err(|e| e.to_string())
}

fn news_content_paths(state: &ServerState, news_id: &str) -> [PathBuf; 2] {
    let root = PathBuf::from(&state.orchestrator_config.news.news_dir);
    [
        root.join(format!("{news_id}.md")),
        root.join("drafts").join(format!("{news_id}.md")),
    ]
}

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
    /// Filename stem for `docs/news/drafts/{news_id}.md` (no slashes).
    pub news_id: String,
    pub title: String,
    pub author: String,
    pub abstract_text: String,
}

pub async fn vox_news_draft_research(
    state: &ServerState,
    params: VoxNewsDraftResearchParams,
) -> String {
    if let Err(e) = vox_publisher::contract::validate_news_id(&params.news_id) {
        return ToolResult::<String>::err(e.to_string()).to_json();
    }
    let now = chrono::Utc::now();
    let draft_content = templates::render_research_update(
        &params.news_id,
        &params.title,
        &params.author,
        &now.to_rfc3339(),
        &params.abstract_text,
    );

    let draft_path = PathBuf::from(&state.orchestrator_config.news.news_dir)
        .join("drafts")
        .join(format!("{}.md", params.news_id));
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
    let paths = news_content_paths(state, &params.news_id);
    let content = match read_news_markdown_first(&paths) {
        Ok(c) => c,
        Err(e) => {
            return ToolResult::<String>::err(format!(
                "Could not read news markdown for {:?}: {}",
                params.news_id, e
            ))
            .to_json();
        }
    };
    let digest = match UnifiedNewsItem::parse(&content, &params.news_id) {
        Ok(item) => item.content_sha3_256(),
        Err(e) => {
            return ToolResult::<String>::err(format!(
                "Cannot approve without a valid UnifiedNewsItem parse: {}",
                e
            ))
            .to_json();
        }
    };
    let item = match UnifiedNewsItem::parse(&content, &params.news_id) {
        Ok(item) => item,
        Err(e) => {
            return ToolResult::<String>::err(format!(
                "Cannot approve without a valid UnifiedNewsItem parse: {}",
                e
            ))
            .to_json();
        }
    };
    let metadata_json = serde_json::json!({
        "tags": item.tags,
        "syndication": item.syndication,
    })
    .to_string();
    let source_ref = paths[0].to_string_lossy().to_string();
    if let Err(e) = db
        .upsert_publication_manifest(vox_db::PublicationManifestParams {
            publication_id: &params.news_id,
            content_type: "news",
            source_ref: Some(source_ref.as_str()),
            title: &item.title,
            author: &item.author,
            abstract_text: None,
            body_markdown: &item.content_markdown,
            citations_json: None,
            metadata_json: Some(metadata_json.as_str()),
            content_sha3_256: &digest,
            state: "draft",
        })
        .await
    {
        return ToolResult::<String>::err(format!("DB error: {}", e)).to_json();
    }
    match db
        .record_publication_approval_for_digest(&params.news_id, &digest, approver)
        .await
    {
        Ok(()) => {}
        Err(e) => {
            return ToolResult::<String>::err(format!("DB error: {}", e)).to_json();
        }
    }
    let count = match db
        .count_publication_approvers_for_digest(&params.news_id, &digest)
        .await
    {
        Ok(c) => c,
        Err(e) => {
            return ToolResult::<String>::err(format!("DB error: {}", e)).to_json();
        }
    };
    ToolResult::ok(format!(
        "Recorded approval from {:?} for news_id {:?} digest {}. Distinct approver count: {} (need 2 for live).",
        approver, params.news_id, digest, count
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
    content_sha3_256: String,
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
    let paths = news_content_paths(state, &params.news_id);
    let content = match read_news_markdown_first(&paths) {
        Ok(c) => c,
        Err(e) => {
            return ToolResult::<String>::err(format!(
                "Could not read news markdown for {:?}: {}",
                params.news_id, e
            ))
            .to_json();
        }
    };
    let digest = match UnifiedNewsItem::parse(&content, &params.news_id) {
        Ok(item) => item.content_sha3_256(),
        Err(e) => {
            return ToolResult::<String>::err(format!(
                "Cannot read approval status without a valid UnifiedNewsItem parse: {}",
                e
            ))
            .to_json();
        }
    };
    let count = match db
        .count_publication_approvers_for_digest(&params.news_id, &digest)
        .await
    {
        Ok(c) => c,
        Err(e) => return ToolResult::<String>::err(format!("DB error: {}", e)).to_json(),
    };
    let dual = match db
        .has_dual_publication_approval_for_digest(&params.news_id, &digest)
        .await
    {
        Ok(b) => b,
        Err(e) => return ToolResult::<String>::err(format!("DB error: {}", e)).to_json(),
    };
    ToolResult::ok(ApprovalStatusBody {
        news_id: params.news_id,
        content_sha3_256: digest,
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
    /// Effective “armed for live publish” after config + `VOX_NEWS_PUBLISH_ARMED` env.
    publish_armed_effective: bool,
    would_be_live_without_dry_run: bool,
    blocking_reasons: Vec<GateReason>,
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
                publish_armed_effective: state.orchestrator_config.news.publish_armed,
                would_be_live_without_dry_run: false,
                blocking_reasons: reasons
                    .into_iter()
                    .map(|m| GateReason {
                        code: "parse_error".to_string(),
                        message: m,
                    })
                    .collect(),
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

    let digest = item.content_sha3_256();

    let dual = if let Some(db) = &state.db {
        db.has_dual_publication_approval_for_digest(&params.news_id, &digest)
            .await
            .unwrap_or(false)
    } else {
        reasons.push("no VoxDb: cannot verify approvals".to_string());
        false
    };

    let gate = evaluate_publish_gate(PublishGateInputs {
        orchestrator_dry_run: state.orchestrator_config.news.dry_run,
        item_dry_run: item.syndication.dry_run,
        publish_armed_config: state.orchestrator_config.news.publish_armed,
        publish_armed_env: std::env::var("VOX_NEWS_PUBLISH_ARMED")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false),
        db_present: state.db.is_some(),
        dual_approval_met: dual,
    });
    let mut blocking_reasons = gate.blocking_reasons;
    if !validate_ok {
        blocking_reasons.push(GateReason {
            code: "validate_error".to_string(),
            message: reasons.join("; "),
        });
    }

    ToolResult::ok(GateReport {
        parse_ok: true,
        validate_ok,
        dual_approval_met: dual,
        publish_armed_effective: gate.armed,
        would_be_live_without_dry_run: gate.would_be_live_without_dry_run,
        blocking_reasons,
    })
    .to_json()
}
