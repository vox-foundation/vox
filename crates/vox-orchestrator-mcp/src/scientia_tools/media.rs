use crate::params::ToolResult;
use crate::server_state::ServerState;
use schemars::JsonSchema;
use serde::Deserialize;

use super::common::{REM_SCIENTIA_DB, no_voxdb_tool_string};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationMediaUpsertParams {
    pub publication_id: String,
    pub asset_ref: String,
    pub media_type: String,
    #[serde(default)]
    pub storage_uri: Option<String>,
    pub status: String,
    #[serde(default)]
    pub metadata_json: Option<serde_json::Value>,
}

pub async fn vox_scientia_publication_media_upsert(
    state: &ServerState,
    params: VoxScientiaPublicationMediaUpsertParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let metadata_json = params
        .metadata_json
        .as_ref()
        .map(serde_json::Value::to_string);
    if let Err(e) = db
        .upsert_publication_media_asset(vox_db::PublicationMediaAssetParams {
            publication_id: params.publication_id.as_str(),
            asset_ref: params.asset_ref.as_str(),
            media_type: params.media_type.as_str(),
            storage_uri: params.storage_uri.as_deref(),
            status: params.status.as_str(),
            metadata_json: metadata_json.as_deref(),
        })
        .await
    {
        return ToolResult::<String>::err_with_remediation(
            format!("DB error: {e}"),
            REM_SCIENTIA_DB,
        )
        .to_json();
    }
    ToolResult::ok(serde_json::json!({
        "publication_id": params.publication_id,
        "asset_ref": params.asset_ref,
        "media_type": params.media_type,
        "storage_uri": params.storage_uri,
        "status": params.status,
        "metadata_json_present": metadata_json.is_some()
    }))
    .to_json()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationMediaListParams {
    pub publication_id: String,
}

pub async fn vox_scientia_publication_media_list(
    state: &ServerState,
    params: VoxScientiaPublicationMediaListParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    let rows = match db
        .list_publication_media_assets(&params.publication_id)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            return ToolResult::<String>::err_with_remediation(
                format!("DB error: {e}"),
                REM_SCIENTIA_DB,
            )
            .to_json();
        }
    };
    ToolResult::ok(rows).to_json()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct VoxScientiaPublicationMediaDeleteParams {
    pub publication_id: String,
    pub asset_ref: String,
}

pub async fn vox_scientia_publication_media_delete(
    state: &ServerState,
    params: VoxScientiaPublicationMediaDeleteParams,
) -> String {
    let Some(db) = &state.db else {
        return no_voxdb_tool_string();
    };
    if let Err(e) = db
        .delete_publication_media_asset(&params.publication_id, &params.asset_ref)
        .await
    {
        return ToolResult::<String>::err_with_remediation(
            format!("DB error: {e}"),
            REM_SCIENTIA_DB,
        )
        .to_json();
    }
    ToolResult::ok(serde_json::json!({
        "deleted": true,
        "publication_id": params.publication_id,
        "asset_ref": params.asset_ref
    }))
    .to_json()
}
