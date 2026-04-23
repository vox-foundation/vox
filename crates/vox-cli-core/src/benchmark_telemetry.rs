//! Opt-in unified benchmark telemetry to Codex.

use std::sync::OnceLock;
use vox_db::{DbConfig, VoxDb};

fn telemetry_enabled() -> bool {
    vox_clavis::resolve_secret(vox_clavis::SecretId::VoxBenchmarkTelemetry)
        .expose()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn telemetry_discovery_start() -> std::path::PathBuf {
    if let Some(p) = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxRepositoryRoot).expose() {
        let p = p.trim();
        if !p.is_empty() {
            return std::path::PathBuf::from(p);
        }
    }
    std::env::current_dir().unwrap_or_default()
}

fn repository_id_for_telemetry() -> String {
    let start = telemetry_discovery_start();
    vox_repository::discover_repository_or_fallback(&start).repository_id
}

fn telemetry_runtime() -> &'static tokio::runtime::Runtime {
    static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("benchmark telemetry tokio runtime")
    })
}

/// Async: connect and write one benchmark row.
pub async fn record_opt(name: &str, metric_value: Option<f64>, details: Option<serde_json::Value>) {
    if !telemetry_enabled() {
        return;
    }
    let Ok(cfg) = DbConfig::resolve_canonical() else {
        return;
    };
    let rid = repository_id_for_telemetry();
    if let Ok(db) = VoxDb::connect(cfg).await {
        let _ = db
            .record_benchmark_event(&rid, name, metric_value, None, details)
            .await;
    }
}

/// Blocking variant for sync CLI paths.
pub fn record_opt_blocking(
    name: &str,
    metric_value: Option<f64>,
    details: Option<serde_json::Value>,
) {
    if !telemetry_enabled() {
        return;
    }
    let Ok(cfg) = DbConfig::resolve_canonical() else {
        return;
    };
    let rid = repository_id_for_telemetry();
    telemetry_runtime().block_on(async {
        if let Ok(db) = VoxDb::connect(cfg).await {
            let _ = db
                .record_benchmark_event(&rid, name, metric_value, None, details)
                .await;
        }
    });
}
