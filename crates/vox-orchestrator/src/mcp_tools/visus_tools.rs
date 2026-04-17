use crate::mcp_tools::server_state::ServerState;
use crate::mcp_tools::params::ToolResult;
use vox_browser::global_engine;
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct AuditParams {
    url: String,
}

#[derive(Deserialize)]
struct BaselineParams {
    url: String,
}

pub async fn vox_visus_audit(_state: &ServerState, args: serde_json::Value) -> String {
    let params: AuditParams = match serde_json::from_value(args.clone()) {
        Ok(p) => p,
        Err(e) => {
            return ToolResult::<serde_json::Value>::err(format!("Invalid arguments: {e}")).to_json_compact();
        }
    };

    let engine = global_engine();
    
    let page_id = match engine.open(&params.url, true).await {
        Ok(id) => id,
        Err(e) => return ToolResult::<serde_json::Value>::err(format!("Failed to open browser: {e}")).to_json_compact(),
    };

    // Wait for JS hydration
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let ax_tree = match engine.ax_tree(&page_id).await {
        Ok(tree) => tree,
        Err(e) => json!({ "error": e }),
    };

    let overlaps = match engine.check_overlaps(&page_id).await {
        Ok(o) => json!(o),
        Err(e) => json!({ "error": e }),
    };

    let screenshot_b64 = match engine.screenshot_bytes(&page_id).await {
        Ok(bytes) => {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD.encode(&bytes)
        },
        Err(e) => format!("Error capturing screenshot: {}", e),
    };

    let _ = engine.close(&page_id).await;

    let out = json!({
        "status": "success",
        "url": params.url,
        "ax_tree": ax_tree,
        "overlaps": overlaps,
        "screenshot_base64_length": screenshot_b64.len(),
        "screenshot_base64": screenshot_b64,
    });
    
    ToolResult::<serde_json::Value>::ok(out).to_json_compact()
}

pub async fn vox_visus_baseline(state: &ServerState, args: serde_json::Value) -> String {
    let params: BaselineParams = match serde_json::from_value(args.clone()) {
        Ok(p) => p,
        Err(e) => {
            return ToolResult::<serde_json::Value>::err(format!("Invalid arguments: {e}")).to_json_compact();
        }
    };
    
    let db = match state.db.as_ref() {
        Some(db) => db,
        None => return ToolResult::<serde_json::Value>::err("No VoxDb connected".to_string()).to_json_compact(),
    };

    tracing::info!("Visus baseline triggered for URL: {}", params.url);

    let engine = global_engine();
    let page_id = match engine.open(&params.url, true).await {
        Ok(id) => id,
        Err(e) => return ToolResult::<serde_json::Value>::err(format!("Failed to open browser: {e}")).to_json_compact(),
    };

    // Wait for JS hydration
    tokio::time::sleep(std::time::Duration::from_secs(3)).await;

    let ax_tree = engine.ax_tree(&page_id).await.unwrap_or_default();
    let screenshot_bytes = engine.screenshot_bytes(&page_id).await.unwrap_or_default();
    let _ = engine.close(&page_id).await;

    let screenshot_cas = blake3::hash(&screenshot_bytes).to_string();
    let ax_tree_str = serde_json::to_string(&ax_tree).unwrap_or_default();
    let ax_tree_cas = blake3::hash(ax_tree_str.as_bytes()).to_string();

    let id = format!("visus_base_{}", uuid::Uuid::new_v4());
    
    let row = vox_db::store::types::VisusBaselineRow {
        id: id.clone(),
        target_url: params.url.clone(),
        viewport: "desktop".to_string(),
        theme: "auto".to_string(),
        screenshot_cas,
        ax_tree_cas,
        metadata_json: Some("{}".to_string()),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    if let Err(e) = db.upsert_visus_baseline(row).await {
        return ToolResult::<serde_json::Value>::err(format!("DB error: {e}")).to_json_compact();
    }

    let out = json!({
        "status": "baseline_recorded",
        "url": params.url,
        "baseline_id": id,
        "message": "Golden baseline recorded in DB.",
    });
    ToolResult::<serde_json::Value>::ok(out).to_json_compact()
}
