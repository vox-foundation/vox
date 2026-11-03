use super::super::common::Check;
use std::time::{SystemTime, UNIX_EPOCH};
use vox_db::DbConfig;

pub async fn run(checks: &mut Vec<Check>) {
    let db_opt = if let Ok(cfg) = DbConfig::resolve_canonical() {
        vox_db::VoxDb::connect(cfg).await.ok()
    } else {
        None
    };

    if let Some(db) = db_opt {
        // Check for catalog freshness
        match db
            .get_user_preference(
                "global",
                vox_orchestrator::models::health::MODEL_CATALOG_LAST_REFRESH_KEY,
            )
            .await
        {
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
                        let cache_file = vox_config::paths::dot_vox_user_dir()
                            .join("cache")
                            .join("model-catalog.v1.json");
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
        checks.push(routing_health_check(
            db.get_user_preference(
                "global",
                vox_orchestrator::models::health::ROUTING_HEALTH_KEY,
            )
            .await
            .ok()
            .flatten()
            .as_deref(),
        ));
    } else {
        checks.push(Check::fail(
            "Model Catalog",
            "Database not available; skipping freshness check.".to_string(),
        ));
    }
}

/// The "Model routing" doctor row from the persisted [`RoutingHealth`] JSON.
pub(crate) fn routing_health_check(json: Option<&str>) -> Check {
    use vox_orchestrator::models::health::RoutingHealth;
    use vox_orchestrator::models::reference::ReferenceSource;
    const NAME: &str = "Model routing";
    let Some(json) = json else {
        return Check::fail(
            NAME,
            "No routing health recorded yet. Run `vox model discover`.",
        );
    };
    let Ok(h) = serde_json::from_str::<RoutingHealth>(json) else {
        return Check::fail(
            NAME,
            "Routing health record is unreadable. Run `vox model discover`.",
        );
    };
    let source = |s: ReferenceSource| {
        if s == ReferenceSource::Derived {
            "live catalog"
        } else {
            "built-in fallback"
        }
    };
    let summary = format!(
        "{} of {} cloud models benchmarked ({} inherited); quality scale: {}; price bands: {}; Efficient picks {}",
        h.benchmarked,
        h.cloud_models,
        h.inherited,
        source(h.quality_scale),
        source(h.price_bands),
        h.efficient_pick.as_deref().unwrap_or("nothing")
    );
    if h.violations.is_empty() {
        Check::pass(NAME, summary)
    } else {
        let problems: Vec<String> = h
            .violations
            .iter()
            .map(|v| format!("{}: {}", v.invariant, v.detail))
            .collect();
        Check::fail(
            NAME,
            format!("{summary}. Problems: {}", problems.join("; ")),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routing_health_passes_without_violations() {
        let json = r#"{"schema_version":1,"checked_at_unix":0,"models":9,"cloud_models":7,"benchmarked":3,
          "inherited":1,"unknown_tier_cloud":0,"quality_scale":"derived","price_bands":"fallback","violations":[]}"#;
        let c = routing_health_check(Some(json));
        assert!(c.pass, "{}", c.detail);
        assert!(
            c.detail.contains("3 of 7 cloud models benchmarked"),
            "{}",
            c.detail
        );
        assert!(
            c.detail.contains("price bands: built-in fallback"),
            "{}",
            c.detail
        );
    }

    #[test]
    fn routing_health_fails_and_names_each_violation() {
        let json = r#"{"schema_version":1,"checked_at_unix":0,"models":9,"cloud_models":7,"benchmarked":3,
          "inherited":1,"unknown_tier_cloud":5,"quality_scale":"derived","price_bands":"derived",
          "violations":[{"invariant":"tiers_known","detail":"5 of 7 cloud models have no tier"}]}"#;
        let c = routing_health_check(Some(json));
        assert!(!c.pass);
        assert!(
            c.detail
                .contains("tiers_known: 5 of 7 cloud models have no tier"),
            "{}",
            c.detail
        );
    }

    #[test]
    fn routing_health_missing_or_unreadable_fails_with_the_fix() {
        assert!(!routing_health_check(None).pass);
        assert!(
            routing_health_check(None)
                .detail
                .contains("vox model discover")
        );
        assert!(!routing_health_check(Some("not json")).pass);
    }
}
