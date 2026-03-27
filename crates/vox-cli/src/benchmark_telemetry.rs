//! Opt-in unified benchmark telemetry to Codex (`research_metrics` type `benchmark_event`).
//!
//! Set **`VOX_BENCHMARK_TELEMETRY=1`** (or `true`) to append rows. Uses [`DbConfig::resolve_standalone`]
//! and [`vox_repository::discover_repository_or_fallback`] for connection + `repository_id`.
//!
//! When **`VOX_REPOSITORY_ROOT`** is set to a non-empty path, discovery starts there instead of
//! [`std::env::current_dir`] so CLI subprocesses can match MCP [`vox_repository::RepositoryContext`].

use std::sync::OnceLock;

use vox_db::{DbConfig, VoxDb};

fn telemetry_enabled() -> bool {
    std::env::var("VOX_BENCHMARK_TELEMETRY")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn syntax_k_telemetry_enabled() -> bool {
    std::env::var("VOX_SYNTAX_K_TELEMETRY")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or_else(|_| telemetry_enabled())
}

fn telemetry_discovery_start() -> std::path::PathBuf {
    if let Ok(p) = std::env::var("VOX_REPOSITORY_ROOT") {
        let p = p.trim();
        if !p.is_empty() {
            return std::path::PathBuf::from(p);
        }
    }
    match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            tracing::debug!(
                target: "vox.benchmark_telemetry",
                error = %e,
                "current_dir failed; using empty path for repository discovery"
            );
            std::path::PathBuf::new()
        }
    }
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

/// Async: connect and write one benchmark row (no-op unless `VOX_BENCHMARK_TELEMETRY` is set).
pub async fn record_opt(name: &str, metric_value: Option<f64>, details: Option<serde_json::Value>) {
    if !telemetry_enabled() {
        return;
    }
    let Ok(cfg) = DbConfig::resolve_standalone() else {
        tracing::debug!(target: "vox.benchmark_telemetry", "skip: db config unresolved");
        return;
    };
    let rid = repository_id_for_telemetry();
    match VoxDb::connect(cfg).await {
        Ok(db) => {
            if let Err(e) = db
                .record_benchmark_event(&rid, name, metric_value, details)
                .await
            {
                tracing::debug!(
                    target: "vox.benchmark_telemetry",
                    error = %e,
                    "record_benchmark_event failed"
                );
            }
        }
        Err(e) => tracing::debug!(
            target: "vox.benchmark_telemetry",
            error = %e,
            "VoxDb::connect failed"
        ),
    }
}

/// Async: connect and write one syntax-k telemetry row (no-op unless telemetry env is enabled).
pub async fn record_syntax_k_opt(
    name: &str,
    fixture_id: &str,
    metric_value: Option<f64>,
    details: Option<serde_json::Value>,
) {
    if !syntax_k_telemetry_enabled() {
        return;
    }
    let Ok(cfg) = DbConfig::resolve_standalone() else {
        tracing::debug!(target: "vox.benchmark_telemetry", "skip syntax-k: db config unresolved");
        return;
    };
    let rid = repository_id_for_telemetry();
    match VoxDb::connect(cfg).await {
        Ok(db) => {
            if let Err(e) = db
                .record_syntax_k_event(&rid, name, fixture_id, metric_value, details)
                .await
            {
                tracing::debug!(
                    target: "vox.benchmark_telemetry",
                    error = %e,
                    "record_syntax_k_event failed"
                );
            }
        }
        Err(e) => tracing::debug!(
            target: "vox.benchmark_telemetry",
            error = %e,
            "VoxDb::connect failed for syntax-k"
        ),
    }
}

/// Blocking variant for sync CLI paths (no Tokio handle required).
pub fn record_opt_blocking(
    name: &str,
    metric_value: Option<f64>,
    details: Option<serde_json::Value>,
) {
    if !telemetry_enabled() {
        return;
    }
    let Ok(cfg) = DbConfig::resolve_standalone() else {
        tracing::debug!(target: "vox.benchmark_telemetry", "skip: db config unresolved");
        return;
    };
    let rid = repository_id_for_telemetry();
    telemetry_runtime().block_on(async {
        match VoxDb::connect(cfg).await {
            Ok(db) => {
                if let Err(e) = db
                    .record_benchmark_event(&rid, name, metric_value, details)
                    .await
                {
                    tracing::debug!(
                        target: "vox.benchmark_telemetry",
                        error = %e,
                        "record_benchmark_event failed"
                    );
                }
            }
            Err(e) => tracing::debug!(
                target: "vox.benchmark_telemetry",
                error = %e,
                "VoxDb::connect failed"
            ),
        }
    });
}

/// Blocking variant for sync syntax-k telemetry paths.
pub fn record_syntax_k_opt_blocking(
    name: &str,
    fixture_id: &str,
    metric_value: Option<f64>,
    details: Option<serde_json::Value>,
) {
    if !syntax_k_telemetry_enabled() {
        return;
    }
    let Ok(cfg) = DbConfig::resolve_standalone() else {
        tracing::debug!(target: "vox.benchmark_telemetry", "skip syntax-k: db config unresolved");
        return;
    };
    let rid = repository_id_for_telemetry();
    telemetry_runtime().block_on(async {
        match VoxDb::connect(cfg).await {
            Ok(db) => {
                if let Err(e) = db
                    .record_syntax_k_event(&rid, name, fixture_id, metric_value, details)
                    .await
                {
                    tracing::debug!(
                        target: "vox.benchmark_telemetry",
                        error = %e,
                        "record_syntax_k_event failed"
                    );
                }
            }
            Err(e) => tracing::debug!(
                target: "vox.benchmark_telemetry",
                error = %e,
                "VoxDb::connect failed for syntax-k"
            ),
        }
    });
}
