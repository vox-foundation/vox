//! Vox **populi** — node registry, optional HTTP control plane, and (feature **`mens`**) native ML.
//!
//! CPU-first: each [`NodeRecord`] carries [`vox_orchestrator::TaskCapabilityHints`]. See
//! `docs/src/architecture/populi-ssot.md` for environment variables.
//! The **`mens`** module holds Burn/Candle QLoRA training (`--features mens …`).

#![deny(missing_docs)]
#![cfg_attr(feature = "mens", allow(missing_docs))]
// Burn/wgpu + absorbed Mens stack: default recursion limit can overflow on deep generic graphs.
#![recursion_limit = "256"]

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

/// Whether populi hooks are enabled (`VOX_MESH_ENABLED=1` or `true`).
#[must_use]
pub fn populi_enabled_from_env() -> bool {
    std::env::var("VOX_MESH_ENABLED")
        .map(|v| {
            let v = v.trim();
            v == "1" || v.eq_ignore_ascii_case("true")
        })
        .unwrap_or(false)
}

/// Parsed populi-related environment (best-effort).
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct PopuliEnv {
    /// `VOX_MESH_ENABLED`
    pub enabled: bool,
    /// `VOX_MESH_NODE_ID` — stable id for this process; generated if unset when registering.
    pub node_id: Option<String>,
    /// `VOX_MESH_LABELS` — comma-separated labels merged into capability labels.
    pub labels: Vec<String>,
    /// `VOX_MESH_CONTROL_ADDR` — e.g. `http://127.0.0.1:9847` for HTTP control plane client/server.
    pub control_addr: Option<String>,
    /// `VOX_MESH_REGISTRY_PATH` — override for the local JSON registry file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_path: Option<String>,
    /// `VOX_MESH_SCOPE_ID` — populi cluster / tenancy id (join/heartbeat must match server when server enforces scope).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_id: Option<String>,
}

/// Merge `Vox.toml` `[populi]` into env-derived values when the corresponding env is unset.
/// Precedence: **environment always wins** over TOML for each field.
#[must_use]
pub fn populi_env_resolved(vox_toml_path: Option<&std::path::Path>) -> PopuliEnv {
    let mut env = populi_env();
    let Some(path) = vox_toml_path else {
        return env;
    };
    let Ok(Some(toml)) = vox_repository::read_vox_populi_toml(path) else {
        return env;
    };
    if env.control_addr.is_none()
        && let Some(url) = toml
            .control_url
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
    {
        env.control_addr = Some(url.to_string());
    }
    if env.scope_id.is_none()
        && let Some(s) = toml
            .scope_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
    {
        env.scope_id = Some(s.to_string());
    }
    if let Some(labels) = toml.labels {
        for lab in labels {
            let lab = lab.trim().to_string();
            if lab.is_empty() || env.labels.contains(&lab) {
                continue;
            }
            env.labels.push(lab);
        }
    }
    if toml.advertise_gpu == Some(true) && std::env::var("VOX_MESH_ADVERTISE_GPU").is_err() {
        // Caller applies gpu via probe merge in node_record; flag via env struct is absent — handled in node_record.
    }
    env
}

/// Whether `VOX_MESH_ADVERTISE_GPU` is set, or `[populi].advertise_gpu = true` when env is unset.
#[must_use]
pub fn populi_advertise_gpu_effective(vox_toml_path: Option<&std::path::Path>) -> bool {
    if std::env::var("VOX_MESH_ADVERTISE_GPU")
        .map(|v| {
            let v = v.trim();
            v == "1" || v.eq_ignore_ascii_case("true")
        })
        .unwrap_or(false)
    {
        return true;
    }
    let Some(path) = vox_toml_path else {
        return false;
    };
    matches!(
        vox_repository::read_vox_populi_toml(path),
        Ok(Some(m)) if m.advertise_gpu == Some(true)
    )
}

/// Normalize a control-plane URL for use as an HTTP **client** base (join / heartbeat / list).
///
/// Returns [`None`] for empty strings, invalid trim, or **bind-all** hosts (`0.0.0.0`, `::`) where
/// the value is meant for `vox populi serve --bind`, not outbound requests.
#[must_use]
pub fn normalize_http_control_base(raw: &str) -> Option<String> {
    let mut s = raw.trim().to_string();
    if s.is_empty() {
        return None;
    }
    let lower = s.to_ascii_lowercase();
    if !lower.starts_with("http://") && !lower.starts_with("https://") {
        s = format!("http://{s}");
    }
    while s.ends_with('/') {
        s.pop();
    }
    if http_control_host_is_bind_all(&s) {
        return None;
    }
    Some(s)
}

fn http_control_host_is_bind_all(url: &str) -> bool {
    let Some(idx) = url.find("://") else {
        return false;
    };
    let rest = &url[idx + 3..];
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    let hostport = authority
        .rsplit_once('@')
        .map(|(_, h)| h)
        .unwrap_or(authority);
    let host = if let Some(stripped) = hostport.strip_prefix('[') {
        stripped.split(']').next().unwrap_or(stripped)
    } else {
        hostport.split(':').next().unwrap_or(hostport)
    };
    host == "0.0.0.0" || host == "::"
}

/// Read populi env vars (does not mutate process state).
#[must_use]
pub fn populi_env() -> PopuliEnv {
    let enabled = populi_enabled_from_env();
    let node_id = std::env::var("VOX_MESH_NODE_ID")
        .ok()
        .filter(|s| !s.is_empty());
    let labels = std::env::var("VOX_MESH_LABELS")
        .ok()
        .map(|s| {
            s.split(',')
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .collect()
        })
        .unwrap_or_default();
    let control_addr = std::env::var("VOX_MESH_CONTROL_ADDR")
        .ok()
        .filter(|s| !s.is_empty());
    let registry_path = std::env::var("VOX_MESH_REGISTRY_PATH")
        .ok()
        .filter(|s| !s.is_empty());
    let scope_id = populi_scope_id_from_env();
    PopuliEnv {
        enabled,
        node_id,
        labels,
        control_addr,
        registry_path,
        scope_id,
    }
}

/// `VOX_MESH_SCOPE_ID` when set and non-empty after trim.
#[must_use]
pub fn populi_scope_id_from_env() -> Option<String> {
    std::env::var("VOX_MESH_SCOPE_ID")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Current Unix time in milliseconds (for federation timestamps and tests).
#[must_use]
pub fn wall_clock_unix_ms() -> u64 {
    now_ms()
}

pub(crate) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

mod node_registry;

pub use node_registry::{
    LocalRegistry, NodeRecord, PopuliRegistryError, PopuliRegistryFile,
    filter_registry_by_max_stale_ms,
};

/// Resolve `Vox.toml` path next to the current working directory (nearest manifest root).
#[must_use]
pub fn resolve_vox_toml_best_effort() -> Option<PathBuf> {
    std::env::current_dir()
        .ok()
        .and_then(|cwd| vox_repository::find_project_manifest_root(&cwd))
        .map(|root| root.join("Vox.toml"))
        .filter(|p| p.is_file())
}

/// Build a [`NodeRecord`] for this process using orchestrator host probe + populi env labels.
#[must_use]
pub fn node_record_for_current_process(node_id: String, listen_addr: Option<String>) -> NodeRecord {
    let vox = resolve_vox_toml_best_effort();
    let vox_ref = vox.as_deref();
    let env = populi_env_resolved(vox_ref);
    let mut caps = vox_repository::probe_host_capabilities();
    if populi_advertise_gpu_effective(vox_ref) {
        caps.gpu_cuda = true;
    }
    for lab in env.labels {
        if !caps.labels.contains(&lab) {
            caps.labels.push(lab);
        }
    }
    NodeRecord {
        id: node_id,
        capabilities: caps,
        listen_addr,
        version: env!("CARGO_PKG_VERSION").to_string(),
        last_seen_unix_ms: now_ms(),
        scope_id: env.scope_id.clone(),
        visibility: None,
        pool_id: None,
        trust_tier: None,
        workload_classes: None,
        privacy_class: None,
        maintenance: None,
        provider: None,
        quarantined: None,
    }
}

/// Register this process into the default local registry file (no-op if `VOX_MESH_ENABLED` is off).
pub fn publish_local_registry_best_effort() -> Result<(), PopuliRegistryError> {
    if !populi_enabled_from_env() {
        return Ok(());
    }
    let record = populi_registration_record_for_process();
    let reg = LocalRegistry::new(LocalRegistry::resolved_default_path());
    reg.upsert_node(record)
}

fn uuid_simple() -> String {
    use std::time::Instant;
    let n = Instant::now().elapsed().as_nanos();
    format!("{n:x}")
}

/// [`NodeRecord`] for this process — same id and `listen_addr` as [`publish_local_registry_best_effort`].
#[must_use]
pub fn populi_registration_record_for_process() -> NodeRecord {
    let vox = resolve_vox_toml_best_effort();
    let env = populi_env_resolved(vox.as_deref());
    let id = env
        .node_id
        .clone()
        .unwrap_or_else(|| format!("local-{}", uuid_simple()));
    let listen = env.control_addr.clone();
    node_record_for_current_process(id, listen)
}

/// Resolved on-disk registry path (honours `VOX_MESH_REGISTRY_PATH`).
#[must_use]
pub fn local_registry_path() -> PathBuf {
    LocalRegistry::resolved_default_path()
}

#[cfg(feature = "mens")]
pub mod mens;

#[cfg(feature = "transport")]
mod bounded_fs;
#[cfg(feature = "transport")]
pub mod http_client;
#[cfg(feature = "transport")]
pub mod http_lifecycle;
#[cfg(feature = "transport")]
pub mod transport;

#[cfg(test)]
mod normalize_http_control_base_tests {
    use super::normalize_http_control_base;

    #[test]
    fn adds_scheme_and_strips_slash() {
        assert_eq!(
            normalize_http_control_base("populi-ctrl:9847/").as_deref(),
            Some("http://populi-ctrl:9847")
        );
    }

    #[test]
    fn rejects_bind_all_ipv4() {
        assert!(normalize_http_control_base("http://0.0.0.0:9847").is_none());
    }

    #[test]
    fn rejects_bind_all_ipv6() {
        assert!(normalize_http_control_base("http://[::]:9847").is_none());
    }

    #[test]
    fn accepts_loopback() {
        assert_eq!(
            normalize_http_control_base("http://127.0.0.1:9847").as_deref(),
            Some("http://127.0.0.1:9847")
        );
    }
}
