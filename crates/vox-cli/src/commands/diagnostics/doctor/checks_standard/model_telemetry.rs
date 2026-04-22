use super::super::common::Check;
use vox_db::DbConfig;

pub async fn run(checks: &mut Vec<Check>) {
    let db_opt = if let Ok(cfg) = DbConfig::resolve_canonical() {
        vox_db::VoxDb::connect(cfg).await.ok()
    } else {
        None
    };

    if let Some(db) = db_opt {
        // Check for scoreboard freshness
        match db.get_model_scoreboard(7).await {
            Ok(rows) => {
                let max_updated: i64 = rows.iter().map(|r| r.updated_at_ms).max().unwrap_or(0);
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as i64)
                    .unwrap_or(0);

                let age_ms = now_ms - max_updated;
                let age_hours = age_ms / 3600000;

                if max_updated == 0 {
                    checks.push(Check::fail(
                        "Model Scoreboard",
                        "Scoreboard is empty. Run `vox model rollup`.".to_string(),
                    ));
                } else if age_hours > 24 {
                    checks.push(Check::fail(
                        "Model Scoreboard",
                        format!(
                            "Scoreboard is stale (last updated {}h ago). Run `vox model rollup`.",
                            age_hours
                        ),
                    ));
                } else {
                    checks.push(Check::pass(
                        "Model Scoreboard",
                        format!("Fresh (updated {}h ago)", age_hours),
                    ));
                }
            }
            Err(e) => {
                checks.push(Check::fail(
                    "Model Scoreboard",
                    format!("Failed to query scoreboard: {}", e),
                ));
            }
        }
    } else {
        checks.push(Check::fail(
            "Model Scoreboard",
            "Database not available; skipping freshness check.".to_string(),
        ));
    }
}
