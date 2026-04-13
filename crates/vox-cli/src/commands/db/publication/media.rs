use super::*;
use anyhow::{Context, Result};
use std::path::PathBuf;

/// Upsert one publication media asset row.
pub async fn publication_media_upsert(
    publication_id: &str,
    asset_ref: &str,
    media_type: &str,
    storage_uri: Option<&str>,
    status: &str,
    metadata_json_path: Option<&PathBuf>,
) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let metadata_json = if let Some(path) = metadata_json_path {
        Some(
            read_utf8_path_capped(path)
                .with_context(|| format!("failed to read metadata JSON from {}", path.display()))?,
        )
    } else {
        None
    };
    db.upsert_publication_media_asset(vox_db::PublicationMediaAssetParams {
        publication_id,
        asset_ref,
        media_type,
        storage_uri,
        status,
        metadata_json: metadata_json.as_deref(),
    })
    .await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "publication_id": publication_id,
            "asset_ref": asset_ref,
            "media_type": media_type,
            "storage_uri": storage_uri,
            "status": status,
            "metadata_json_present": metadata_json.is_some()
        }))?
    );
    Ok(())
}
/// List publication media assets for one publication id.
pub async fn publication_media_list(publication_id: &str) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    let rows = db.list_publication_media_assets(publication_id).await?;
    println!("{}", serde_json::to_string_pretty(&rows)?);
    Ok(())
}
/// Delete one publication media asset by `publication_id + asset_ref`.
pub async fn publication_media_delete(publication_id: &str, asset_ref: &str) -> Result<()> {
    let db = vox_db::VoxDb::connect_default().await?;
    db.delete_publication_media_asset(publication_id, asset_ref)
        .await?;
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "deleted": true,
            "publication_id": publication_id,
            "asset_ref": asset_ref
        }))?
    );
    Ok(())
}
