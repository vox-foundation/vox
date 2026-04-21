use super::super::common::Check;
use vox_db::DbConfig;
use std::time::{SystemTime, UNIX_EPOCH};

pub async fn run(checks: &mut Vec<Check>) {
    let db_opt = if let Ok(cfg) = DbConfig::resolve_canonical() {
        vox_db::VoxDb::connect(cfg).await.ok()
    } else {
        None
    };

    if let Some(db) = db_opt {
        // Check for catalog freshness
        match db.get_user_preference("global", "catalog_refresh").await {
            Ok(Some(last_str)) => {
                if let Ok(last_secs) = last_str.parse::<u64>() {
                    let now_secs = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    
                    let age_secs = now_secs.saturating_sub(last_secs);
                    let age_hours = age_secs / 3600;

                    if age_hours > 24 {
                        checks.push(Check::fail(
                            "Model Catalog",
                            format!("Catalog is stale (last refreshed {}h ago). Run `vox model discover`.", age_hours),
                        ));
                    } else {
                        // Also check cache file for model counts
                        let cache_file = vox_config::paths::dot_vox_user_dir().join("cache").join("model-catalog.v1.json");
                        let count_str = if let Ok(contents) = std::fs::read_to_string(&cache_file) {
                            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&contents) {
                                if let Some(arr) = json.as_array() {
                                    format!(" ({} models)", arr.len())
                                } else {
                                    "".to_string()
                                }
                            } else {
                                "".to_string()
                            }
                        } else {
                            "".to_string()
                        };

                        checks.push(Check::pass(
                            "Model Catalog",
                            format!("Fresh (refreshed {}h ago){}", age_hours, count_str),
                        ));
                    }
                } else {
                    checks.push(Check::fail(
                        "Model Catalog",
                        "Invalid refresh timestamp in database.".to_string(),
                    ));
                }
            }
            Ok(None) => {
                checks.push(Check::fail(
                    "Model Catalog",
                    "No refresh history found. Run `vox model discover`.".to_string(),
                ));
            }
            Err(e) => {
                checks.push(Check::fail(
                    "Model Catalog",
                    format!("Failed to query catalog refresh status: {}", e),
                ));
            }
        }
    } else {
        checks.push(Check::fail(
            "Model Catalog",
            "Database not available; skipping freshness check.".to_string(),
        ));
    }
}
