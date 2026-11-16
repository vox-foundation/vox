//! Runtime projection IR — orchestration-facing payloads derived from typed core IR.
//!
//! This is **not** WebIR: it captures DB planning policy and (later) task capability hints in a
//! serde-stable shape aligned with `vox_repository::TaskCapabilityHints` wire contracts.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// vox:defactored-from vox-repository 2026-11-12 (avoids vox-compiler -> vox-repository edge)
/// Hardware and routing capability hints aligned with the repository wire contract.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TaskCapabilityHints {
    #[serde(default)]
    pub gpu_cuda: bool,
    #[serde(default)]
    pub gpu_metal: bool,
    #[serde(default)]
    pub gpu_vulkan: bool,
    #[serde(default)]
    pub gpu_webgpu: bool,
    #[serde(default)]
    pub npu: bool,
    #[serde(default)]
    pub visus_eligible: bool,
    #[serde(default)]
    pub multi_modal: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_class: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_vram_mb: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_cores: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hostname: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_cpu_cores: Option<u32>,
    #[serde(default)]
    pub prefer_gpu_compute: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub routing_tier: Option<String>,
    #[serde(default)]
    pub is_low_confidence_bypass_blocked: bool,
    #[serde(default)]
    pub force_socrates_research: bool,
}

fn probe_host_capabilities_inline() -> TaskCapabilityHints {
    let truthy = |k: &str| {
        std::env::var(k)
            .map(|v| v.trim() == "1" || v.trim().eq_ignore_ascii_case("true"))
            .unwrap_or(false)
    };
    TaskCapabilityHints {
        gpu_cuda: truthy("VOX_MESH_ADVERTISE_GPU"),
        gpu_vulkan: truthy("VOX_MESH_ADVERTISE_VULKAN"),
        gpu_webgpu: truthy("VOX_MESH_ADVERTISE_WEBGPU"),
        npu: truthy("VOX_MESH_ADVERTISE_NPU"),
        device_class: std::env::var("VOX_MESH_DEVICE_CLASS")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        cpu_cores: std::thread::available_parallelism()
            .ok()
            .map(|n| n.get() as u32),
        arch: Some(std::env::consts::ARCH.to_string()),
        hostname: std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("COMPUTERNAME"))
            .ok()
            .filter(|s| !s.is_empty()),
        ..Default::default()
    }
}

use crate::hir::db_op_walk::{self, DbTableOpSite};
use crate::hir::{HirDbPlanCapabilities, HirDbRetrievalMode, HirModule, TypedCoreIR_v2};

/// Version of the runtime projection JSON envelope; bump when fields are added or reordered semantically.
pub const RUNTIME_PROJECTION_SCHEMA_VERSION: u32 = 1;

/// One row per distinct DB plan capability snapshot observed on a lowered `db.*` operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DbPlanningPolicySnapshot {
    pub table: String,
    pub op: String,
    pub requires_sync: bool,
    pub emits_change_log: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub live_topic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retrieval_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orchestration_scope: Option<String>,
}

/// Module-level runtime projection consumed by tooling, MCP, and orchestrator integration tests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeProjectionModule {
    pub schema_version: u32,
    /// Soft routing hints inferred from lowered `db.*` plan metadata (`.using("vector"|…)`, `.scope("…")`).
    /// Explicit `@task` / module-level capability syntax may extend this later.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_task_capability_hints: Option<TaskCapabilityHints>,
    /// Best-effort host probe for `TaskCapabilityHints` wire shape; only when
    /// `VOX_RUNTIME_PROJECTION_INCLUDE_HOST_PROBE=1` (keeps default JSON stable across machines).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host_capability_probe: Option<TaskCapabilityHints>,
    /// Distinct DB planning capability rows from all `db.Table.*` sites with a [`crate::hir::HirDbQueryPlan`].
    pub db_planning_policies: Vec<DbPlanningPolicySnapshot>,
}

fn hir_db_table_op_label(op: crate::hir::HirDbTableOp) -> &'static str {
    use crate::hir::HirDbTableOp;
    match op {
        HirDbTableOp::Insert => "insert",
        HirDbTableOp::Get => "get",
        HirDbTableOp::Update => "update",
        HirDbTableOp::Delete => "delete",
        HirDbTableOp::All => "all",
        HirDbTableOp::FilterRecord => "filter_record",
        HirDbTableOp::Count => "count",
        HirDbTableOp::UnsafeQueryRawClause => "unsafe_query_raw_clause",
    }
}

fn retrieval_mode_label(m: HirDbRetrievalMode) -> &'static str {
    match m {
        HirDbRetrievalMode::Fts => "fts",
        HirDbRetrievalMode::Vector => "vector",
        HirDbRetrievalMode::Hybrid => "hybrid",
    }
}

fn snapshot_from_plan(
    table: &str,
    op: crate::hir::HirDbTableOp,
    cap: &HirDbPlanCapabilities,
) -> DbPlanningPolicySnapshot {
    DbPlanningPolicySnapshot {
        table: table.to_string(),
        op: hir_db_table_op_label(op).to_string(),
        requires_sync: cap.requires_sync,
        emits_change_log: cap.emits_change_log,
        live_topic: cap.live_topic.clone(),
        retrieval_mode: cap
            .retrieval_mode
            .map(|m| retrieval_mode_label(m).to_string()),
        orchestration_scope: cap.orchestration_scope.clone(),
    }
}

fn is_non_default_capabilities(cap: &HirDbPlanCapabilities) -> bool {
    cap.requires_sync
        || cap.emits_change_log
        || cap.live_topic.is_some()
        || cap.retrieval_mode.is_some()
        || cap.orchestration_scope.is_some()
}

fn host_capability_probe_from_env() -> Option<TaskCapabilityHints> {
    std::env::var("VOX_RUNTIME_PROJECTION_INCLUDE_HOST_PROBE")
        .ok()
        .filter(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .map(|_| probe_host_capabilities_inline())
}

/// Derive orchestration-facing [`TaskCapabilityHints`] from DB query plan capability metadata.
///
/// Vector / hybrid retrieval implies embedding-heavy work → [`TaskCapabilityHints::prefer_gpu_compute`].
/// Labels record retrieval modes and non-empty `.scope(...)` values for schedulers.
#[must_use]
pub fn infer_module_task_capability_hints(module: &HirModule) -> Option<TaskCapabilityHints> {
    use std::collections::BTreeSet;

    let mut labels: BTreeSet<String> = BTreeSet::new();
    let mut prefer_gpu_compute = false;

    let mut visit = |site: DbTableOpSite<'_>| {
        let Some(plan) = site.plan else {
            return;
        };
        if let Some(mode) = plan.capabilities.retrieval_mode {
            match mode {
                HirDbRetrievalMode::Vector => {
                    prefer_gpu_compute = true;
                    labels.insert("vox:db_retrieval:vector".to_string());
                }
                HirDbRetrievalMode::Hybrid => {
                    prefer_gpu_compute = true;
                    labels.insert("vox:db_retrieval:hybrid".to_string());
                }
                HirDbRetrievalMode::Fts => {
                    labels.insert("vox:db_retrieval:fts".to_string());
                }
            }
        }
        if let Some(scope) = plan.capabilities.orchestration_scope.as_deref() {
            let s = scope.trim();
            if !s.is_empty() {
                labels.insert(format!("vox:orchestration_scope:{s}"));
            }
        }
    };

    db_op_walk::for_each_db_table_op_in_module(module, &mut visit);

    if !prefer_gpu_compute && labels.is_empty() {
        return None;
    }

    Some(TaskCapabilityHints {
        prefer_gpu_compute,
        labels: labels.into_iter().collect(),
        ..Default::default()
    })
}

/// Build runtime projection from lowered core IR (typed HIR).
#[must_use]
pub fn project_runtime_from_core(module: &TypedCoreIR_v2) -> RuntimeProjectionModule {
    project_runtime_from_hir(module)
}

#[must_use]
pub fn project_runtime_from_hir(module: &HirModule) -> RuntimeProjectionModule {
    let mut seen: HashSet<DbPlanningPolicySnapshot> = HashSet::new();
    let mut f = |site: DbTableOpSite<'_>| {
        let Some(plan) = site.plan else {
            return;
        };
        if !is_non_default_capabilities(&plan.capabilities) {
            return;
        }
        let snap = snapshot_from_plan(site.table, site.op, &plan.capabilities);
        seen.insert(snap);
    };
    db_op_walk::for_each_db_table_op_in_module(module, &mut f);
    let mut db_planning_policies: Vec<DbPlanningPolicySnapshot> = seen.into_iter().collect();
    db_planning_policies.sort_by(|a, b| {
        a.table
            .cmp(&b.table)
            .then_with(|| a.op.cmp(&b.op))
            .then_with(|| a.requires_sync.cmp(&b.requires_sync))
            .then_with(|| a.emits_change_log.cmp(&b.emits_change_log))
            .then_with(|| a.live_topic.cmp(&b.live_topic))
            .then_with(|| a.retrieval_mode.cmp(&b.retrieval_mode))
            .then_with(|| a.orchestration_scope.cmp(&b.orchestration_scope))
    });
    RuntimeProjectionModule {
        schema_version: RUNTIME_PROJECTION_SCHEMA_VERSION,
        module_task_capability_hints: infer_module_task_capability_hints(module),
        host_capability_probe: host_capability_probe_from_env(),
        db_planning_policies,
    }
}

/// Canonical JSON bytes for stable hashing / telemetry (sorted object keys at every depth).
pub fn canonical_runtime_projection_bytes(
    module: &RuntimeProjectionModule,
) -> Result<Vec<u8>, serde_json::Error> {
    let mut v = serde_json::to_value(module)?;
    crate::canonical_json::sort_json_value_keys(&mut v);
    serde_json::to_vec(&v)
}
