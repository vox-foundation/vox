//! Script dispatch handlers: dispatch_script, dispatch_results_poll, queue_stats, execute_on_worker.

use axum::Json;
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use base64::Engine as _;

use crate::{NodeRecord, node_maintenance_blocks_new_work};

use super::super::auth::{PopuliAuthContext, auth_allows_deliver, auth_allows_worker_plane};
use super::super::{DispatchRequest, DispatchResponse, MeshQueueStats, PopuliTransportState};

use super::super::dispatch_results_sweep;
use super::nodes::ResponseErr;

pub(crate) async fn dispatch_script(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
    Json(req): Json<DispatchRequest>,
) -> Result<Json<DispatchResponse>, ResponseErr> {
    if !auth_allows_deliver(&ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: submitter/mesh/admin token required for dispatch".into(),
        ));
    }

    let nodes = st.inner.read().await;
    let target = if let Some(id) = &req.node_id {
        nodes.nodes.iter().find(|n| n.id == *id).cloned()
    } else {
        select_best_node(&nodes.nodes, &req).cloned()
    };
    drop(nodes);

    let Some(target) = target else {
        return Err(ResponseErr(
            StatusCode::NOT_FOUND,
            "populi: no suitable worker node found for dispatch".into(),
        ));
    };

    let Some(addr) = &target.listen_addr else {
        return Err(ResponseErr(
            StatusCode::BAD_REQUEST,
            format!("populi: target node {} has no listen_addr", target.id),
        ));
    };

    // Forward to worker
    let client = crate::http_client::PopuliHttpClient::new(addr).with_env_token();

    if req.is_detached {
        use vox_foundation::primitives::id::simple_hex_id;
        let dispatch_id = simple_hex_id();
        let st_cl = st.clone();
        let dispatch_id_cl = dispatch_id.clone();
        let target_node_id = target.id.clone();

        tokio::spawn(async move {
            let res = client.worker_execute(&req).await;
            match res {
                Ok(mut resp) => {
                    resp.expires_unix_ms = Some(crate::now_ms() + 3_600_000); // 1 hour TTL
                    st_cl.dispatch_results.insert(dispatch_id_cl, resp);
                }
                Err(e) => {
                    st_cl.dispatch_results.insert(
                        dispatch_id_cl,
                        DispatchResponse {
                            success: false,
                            output: String::new(),
                            error: Some(format!(
                                "populi: detached execution failed to forward: {}",
                                e
                            )),
                            node_id: target_node_id,
                            duration_ms: 0,
                            exit_code: None,
                            is_truncated: false,
                            expires_unix_ms: Some(crate::now_ms() + 3_600_000),
                        },
                    );
                }
            }
            if let Some(path) = &st_cl.dispatch_results_store_path {
                let _ = super::super::store::persist_dispatch_results_store(
                    path,
                    &st_cl.dispatch_results,
                );
            }
        });

        Ok(Json(DispatchResponse {
            success: true,
            output: format!(
                "populi: detached dispatch accepted. poll for results with id: {}",
                dispatch_id
            ),
            error: None,
            node_id: target.id,
            duration_ms: 0,
            exit_code: None,
            is_truncated: false,
            expires_unix_ms: None,
        }))
    } else {
        let mut resp = client.worker_execute(&req).await.map_err(|e| {
            ResponseErr(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("populi: failed to forward dispatch to worker: {}", e),
            )
        })?;
        resp.expires_unix_ms = None;
        Ok(Json(resp))
    }
}

pub(crate) async fn dispatch_results_poll(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
    axum::extract::Path(dispatch_id): axum::extract::Path<String>,
) -> Result<Json<DispatchResponse>, ResponseErr> {
    if !auth_allows_deliver(&ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: submitter/mesh/admin token required for dispatch polls".into(),
        ));
    }

    dispatch_results_sweep(&st.dispatch_results, crate::now_ms());

    if let Some(res) = st.dispatch_results.get(&dispatch_id) {
        Ok(Json(res.clone()))
    } else {
        Err(ResponseErr(
            StatusCode::NOT_FOUND,
            format!(
                "populi: dispatch result for id {} not found or still in-flight",
                dispatch_id
            ),
        ))
    }
}

fn node_passes_dispatch_policy(n: &NodeRecord, req: &DispatchRequest, now_unix_ms: u64) -> bool {
    if n.quarantined == Some(true) || node_maintenance_blocks_new_work(now_unix_ms, n) {
        return false;
    }
    if let Some(required) = &req.required_labels
        && !required.is_empty()
        && !required
            .iter()
            .all(|req_lab| n.capabilities.labels.contains(req_lab))
    {
        return false;
    }
    if let (Some(task_kind_str), Some(policy)) = (&req.task_kind, &n.donation_policy) {
        let allowed = policy.slots.iter().any(|slot| {
            format!("{:?}", slot.task_kind).to_lowercase() == task_kind_str.to_lowercase()
        });
        if !allowed {
            return false;
        }
    }
    true
}

fn select_best_node<'a>(nodes: &'a [NodeRecord], req: &DispatchRequest) -> Option<&'a NodeRecord> {
    let mut candidates: Vec<_> = nodes
        .iter()
        .filter(|n| {
            if !node_passes_dispatch_policy(n, req, crate::now_ms()) {
                return false;
            }
            // VRAM matching stays on script dispatch. The mesh size pick does not use it.
            if let Some(min_vram) = req.min_vram_mb {
                let node_vram = n.capabilities.min_vram_mb.unwrap_or(0);
                if node_vram < min_vram {
                    return false;
                }
            }
            true
        })
        .collect();

    // Load balancing: Sort by CPU usage ascending
    candidates.sort_by(|a, b| {
        let a_usage = a.cpu_usage_pct.unwrap_or(100.0);
        let b_usage = b.cpu_usage_pct.unwrap_or(100.0);
        a_usage
            .partial_cmp(&b_usage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    candidates.first().copied()
}

pub(crate) async fn queue_stats(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
) -> Result<Json<MeshQueueStats>, ResponseErr> {
    if !auth_allows_worker_plane(&ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: worker/mesh/admin token required for queue stats".into(),
        ));
    }
    let msgs = st.a2a_messages.read().await;
    let mut stats = MeshQueueStats {
        pending_count: 0,
        pending_by_kind: std::collections::HashMap::new(),
        pending_by_priority: std::collections::HashMap::new(),
    };

    for m in msgs.iter() {
        if m.acknowledged || m.lease_holder_node_id.is_some() {
            continue;
        }
        stats.pending_count += 1;
        if let Some(kind) = &m.task_kind {
            *stats.pending_by_kind.entry(kind.clone()).or_insert(0) += 1;
        }
        *stats.pending_by_priority.entry(m.priority).or_insert(0) += 1;
    }

    Ok(Json(stats))
}

pub(crate) async fn execute_on_worker(
    State(_st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
    Json(req): Json<DispatchRequest>,
) -> Result<Json<DispatchResponse>, ResponseErr> {
    if !auth_allows_worker_plane(&ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: worker/mesh/admin token required for worker execution".into(),
        ));
    }

    // vox-deprecated-since="0.6.0" retire-by="0.7.0" reason="mesh-phase6" canonical="vox_mesh_transport::InterpExecutor"
    let policy = "source-only";
    if req.is_bundle {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            format!(
                "populi policy: {policy}: this node only allows source-based dispatch (binary execution disabled)"
            ),
        ));
    }

    if let Some(req_labels) = &req.required_labels {
        let local_record = crate::node_record_for_current_process("".into(), None);
        for req_label in req_labels {
            if !local_record.capabilities.labels.contains(req_label) {
                return Err(ResponseErr(
                    StatusCode::FORBIDDEN,
                    format!(
                        "populi capacity constraints: this node lacks the required capability label '{}'",
                        req_label
                    ),
                ));
            }
        }
    }

    let source_bytes = base64::engine::general_purpose::STANDARD
        .decode(&req.source)
        .map_err(|e| {
            ResponseErr(
                StatusCode::BAD_REQUEST,
                format!("populi: invalid base64 source: {}", e),
            )
        })?;

    // Phase 2: Integrity Verification
    if let Some(expected_hex) = &req.source_blake3_hex {
        let actual_hash = blake3::hash(&source_bytes);
        let actual_hex = actual_hash.to_hex().to_string();
        if &actual_hex != expected_hex {
            return Err(ResponseErr(
                StatusCode::BAD_REQUEST,
                format!(
                    "populi integrity error: bundle hash mismatch (expected {}, got {})",
                    expected_hex, actual_hex
                ),
            ));
        }
    }

    let tmp_dir = std::env::temp_dir();
    let bin_path = if req.is_bundle {
        // Source is actually a pre-compiled binary.
        // Identify .wasm vs native.
        let is_wasm = source_bytes.starts_with(b"\0asm");
        let ext = if is_wasm { ".wasm" } else { "" };
        let file_name = format!(
            "vox-bundle-{}{}",
            vox_foundation::primitives::id::simple_hex_id(),
            ext
        );
        let tmp_file = tmp_dir.join(file_name);
        std::fs::write(&tmp_file, &source_bytes).map_err(|e| {
            ResponseErr(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("populi: failed to write bundle: {}", e),
            )
        })?;

        // Ensure executable on Unix
        #[cfg(unix)]
        if !is_wasm {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&tmp_file)
                .map(|m| m.permissions())
                .unwrap_or_else(|_| std::fs::Permissions::from_mode(0o755));
            perms.set_mode(0o755);
            let _ = std::fs::set_permissions(&tmp_file, perms);
        }

        tmp_file
    } else {
        let file_name = format!(
            "vox-dispatch-{}.vox",
            vox_foundation::primitives::id::simple_hex_id()
        );
        let tmp_file = tmp_dir.join(file_name);
        std::fs::write(&tmp_file, &source_bytes).map_err(|e| {
            ResponseErr(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("populi: failed to write tmp file: {}", e),
            )
        })?;
        tmp_file
    };

    let start_time = std::time::Instant::now();

    let output = if req.is_bundle {
        if bin_path.extension().is_some_and(|ext| ext == "wasm") {
            return Err(ResponseErr(
                StatusCode::BAD_REQUEST,
                "populi: WASI bundle execution was retired; dispatch source (`.vox`) instead"
                    .into(),
            ));
        } else {
            std::process::Command::new(&bin_path).output()
        }
    } else {
        std::process::Command::new("vox")
            .arg("run")
            .arg("--mode")
            .arg("script")
            .arg(&bin_path)
            .output()
    };

    let duration_ms = start_time.elapsed().as_millis() as u64;

    // Cleanup early
    let _ = std::fs::remove_file(&bin_path);

    match output {
        Ok(out) => {
            // Phase 3: Output Truncation (10MB Limit)
            const MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024;
            let mut combined_stdout = out.stdout;
            let mut combined_stderr = out.stderr;

            let total_len = combined_stdout.len() + combined_stderr.len();
            let is_truncated = total_len > MAX_OUTPUT_BYTES;

            if is_truncated {
                // Keep the first 10MB of stderr then stdout or split evenly
                if combined_stderr.len() > MAX_OUTPUT_BYTES / 2 {
                    combined_stderr.truncate(MAX_OUTPUT_BYTES / 2);
                }
                let remaining = MAX_OUTPUT_BYTES.saturating_sub(combined_stderr.len());
                if combined_stdout.len() > remaining {
                    combined_stdout.truncate(remaining);
                }
            }

            let output_str = String::from_utf8_lossy(&combined_stdout).to_string()
                + &String::from_utf8_lossy(&combined_stderr);

            Ok(Json(DispatchResponse {
                success: out.status.success(),
                output: output_str,
                is_truncated,
                duration_ms,
                exit_code: out.status.code(),
                error: if out.status.success() {
                    None
                } else {
                    Some(format!("Exit code: {:?}", out.status.code()))
                },
                node_id: vox_secrets::resolve_secret(vox_secrets::SecretId::VoxMeshNodeId)
                    .expose()
                    .unwrap_or("unknown")
                    .to_string(),
                expires_unix_ms: None,
            }))
        }
        Err(e) => Ok(Json(DispatchResponse {
            success: false,
            output: String::new(),
            is_truncated: false,
            duration_ms,
            exit_code: None,
            error: Some(format!("Failed to execute vox: {}", e)),
            node_id: vox_secrets::resolve_secret(vox_secrets::SecretId::VoxMeshNodeId)
                .expose()
                .unwrap_or("unknown")
                .to_string(),
            expires_unix_ms: None,
        })),
    }
}

/// One mesh placement of a measured `qwen3_code` QLoRA size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Qwen3MeshSelection {
    pub node_id: String,
    pub size_token: String,
    pub preset: String,
    pub floor_mb: u32,
    pub hf_id: String,
    pub accelerator_free_mb: u64,
}

/// Why [`select_qwen3_mesh`] refused to name a node.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(crate) enum Qwen3MeshError {
    /// `accelerator_free_mb` does not fit the picker's `u32` argument.
    #[error("accelerator_free_mb {0} does not fit in u32")]
    ReadingExceedsU32(u64),
    /// `pick_base` (via the qwen3 code QLoRA selector) returned an error.
    #[error("{0}")]
    PickFailed(String),
    /// No node has a usable free-memory reading.
    #[error("no fresh accelerator free-memory reading")]
    NoFreshReading,
    /// An explicit size spelling does not fit any eligible node.
    #[error("explicit size does not fit any eligible node")]
    ExplicitDoesNotFit,
    /// Fresh readings exist and none hold a measured floor.
    #[error("no measured qwen3_code QLoRA floor fits the fresh readings")]
    NothingFits,
}

/// Largest measured `qwen3_code` QLoRA size that fits some eligible node's
/// `accelerator_free_mb`, placed on a node that holds that floor.
///
/// One call to `select_qwen3_code_qlora_for_free_mb` (which calls `pick_base`).
/// Total VRAM, system RAM, and `min_vram_mb` are not inputs.
pub(crate) fn select_qwen3_mesh(
    nodes: &[NodeRecord],
    req: &DispatchRequest,
    now_unix_ms: u64,
    stale_window_ms: Option<u64>,
    explicit_size: Option<&str>,
) -> Result<Qwen3MeshSelection, Qwen3MeshError> {
    let _ = (stale_window_ms, explicit_size);
    let candidates: Vec<&NodeRecord> = nodes
        .iter()
        .filter(|node| {
            node.accelerator_free_mb.is_some()
                && node_passes_dispatch_policy(node, req, now_unix_ms)
        })
        .collect();
    let Some(max_free) = candidates
        .iter()
        .filter_map(|node| node.accelerator_free_mb)
        .max()
    else {
        return Err(Qwen3MeshError::NoFreshReading);
    };
    let free_u32 =
        u32::try_from(max_free).map_err(|_| Qwen3MeshError::ReadingExceedsU32(max_free))?;
    let pick = vox_populi::mens::tensor::spoke_base_resolver::select_qwen3_code_qlora_for_free_mb(
        free_u32,
    )
    .map_err(|err| Qwen3MeshError::PickFailed(err.to_string()))?;
    route_pick_to_holder(candidates, pick)
}

fn route_pick_to_holder(
    candidates: Vec<&NodeRecord>,
    pick: vox_populi::mens::tensor::spoke_base_resolver::Qwen3LivePick,
) -> Result<Qwen3MeshSelection, Qwen3MeshError> {
    let floor = u64::from(pick.floor_mb);
    let mut holders: Vec<&NodeRecord> = candidates
        .into_iter()
        .filter(|node| node.accelerator_free_mb.unwrap_or(0) >= floor)
        .collect();
    if holders.is_empty() {
        return Err(Qwen3MeshError::NothingFits);
    }
    holders.sort_by(|left, right| {
        let left_cpu = left.cpu_usage_pct.unwrap_or(100.0);
        let right_cpu = right.cpu_usage_pct.unwrap_or(100.0);
        left_cpu
            .partial_cmp(&right_cpu)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let chosen = holders[0];
    Ok(Qwen3MeshSelection {
        node_id: chosen.id.clone(),
        size_token: pick.size_token,
        preset: pick.preset,
        floor_mb: pick.floor_mb,
        hf_id: pick.hf_id,
        accelerator_free_mb: chosen
            .accelerator_free_mb
            .expect("holder was filtered on accelerator_free_mb"),
    })
}

#[cfg(test)]
mod qwen3_mesh_tests {
    use super::*;

    fn node(id: &str, free_mb: Option<u64>, cpu: Option<f32>) -> NodeRecord {
        let mut record: NodeRecord = serde_json::from_value(serde_json::json!({
            "id": id,
            "capabilities": {},
            "version": "test",
            "last_seen_unix_ms": 10_000_000,
            "memory_free_bytes": null,
        }))
        .expect("minimal NodeRecord");
        record.accelerator_free_mb = free_mb;
        record.cpu_usage_pct = cpu;
        record.last_seen_unix_ms = 10_000_000;
        record
    }

    fn plain_req() -> DispatchRequest {
        serde_json::from_value(serde_json::json!({ "source": "test" }))
            .expect("minimal DispatchRequest")
    }

    #[test]
    fn mesh_pick_routes_largest_floor_to_the_node_that_holds_it() {
        let nodes = vec![
            node("wide", Some(70_000), Some(50.0)),
            node("narrow", Some(25_000), Some(10.0)),
        ];
        let got = select_qwen3_mesh(&nodes, &plain_req(), 10_000_000, Some(3_600_000), None)
            .expect("mesh pick");
        let local =
            vox_populi::mens::tensor::spoke_base_resolver::select_qwen3_code_qlora_for_free_mb(
                70_000,
            )
            .expect("local pick");
        assert_eq!(got.node_id, "wide");
        assert_eq!(got.size_token, "32B");
        assert_eq!(got.preset, "qwen3_96g");
        assert_eq!(got.floor_mb, 60_000);
        assert_eq!(
            got.hf_id,
            "Qwen/Qwen3-32B@9216db5781bf21249d130ec9da846c4624c16137"
        );
        assert_eq!(got.size_token, local.size_token);
        assert_eq!(got.preset, local.preset);
        assert_eq!(got.floor_mb, local.floor_mb);
        assert_eq!(got.hf_id, local.hf_id);
        assert_ne!(got.node_id, "narrow");
        assert_eq!(got.accelerator_free_mb, 70_000);

        let missing: NodeRecord = serde_json::from_value(serde_json::json!({
            "id": "missing-free",
            "capabilities": {},
            "version": "test",
            "last_seen_unix_ms": 1,
            "memory_free_bytes": 123,
        }))
        .expect("record without accelerator_free_mb");
        assert_eq!(missing.accelerator_free_mb, None);
        assert_eq!(missing.memory_free_bytes, Some(123));
    }

    #[test]
    fn mesh_pick_tie_breaks_on_lowest_cpu() {
        let nodes = vec![
            node("busy", Some(70_000), Some(80.0)),
            node("idle", Some(70_000), Some(10.0)),
        ];
        let got = select_qwen3_mesh(&nodes, &plain_req(), 10_000_000, Some(3_600_000), None)
            .expect("tie pick");
        assert_eq!(got.node_id, "idle");
        assert_eq!(got.size_token, "32B");
        assert_eq!(got.preset, "qwen3_96g");
        assert_eq!(got.floor_mb, 60_000);
    }

    fn assert_narrow_14b(nodes: &[NodeRecord], req: &DispatchRequest) {
        let got =
            select_qwen3_mesh(nodes, req, 10_000_000, Some(3_600_000), None).expect("narrow pick");
        assert_eq!(got.node_id, "narrow");
        assert_eq!(got.size_token, "14B");
        assert_eq!(got.preset, "qwen3_24g");
        assert_eq!(got.floor_mb, 20_000);
    }

    fn empty_donation() -> vox_mesh_types::WorkerDonationPolicy {
        serde_json::from_value(serde_json::json!({
            "slots": [],
            "nsfw_allowed": false,
            "max_job_duration_secs": 0,
            "public_mesh_opt_in": false,
            "min_priority": 0,
            "allowed_scopes": null,
            "allowed_users": null,
            "denied_users": null,
            "allowed_mesh_networks": null
        }))
        .expect("empty donation policy")
    }

    #[test]
    fn mesh_pick_skips_quarantine_maintenance_labels_and_donation() {
        let mut quarantined = node("wide", Some(70_000), Some(10.0));
        quarantined.quarantined = Some(true);
        let narrow = node("narrow", Some(25_000), Some(90.0));
        assert_narrow_14b(&[quarantined, narrow], &plain_req());

        let mut maintained = node("wide", Some(70_000), Some(10.0));
        maintained.maintenance = Some(true);
        maintained.maintenance_until_unix_ms = Some(10_000_001);
        let narrow = node("narrow", Some(25_000), Some(90.0));
        assert_narrow_14b(&[maintained, narrow], &plain_req());

        let unlabeled = node("wide", Some(70_000), Some(10.0));
        let mut labeled = node("narrow", Some(25_000), Some(90.0));
        labeled
            .capabilities
            .labels
            .push("region=us-east".to_string());
        let mut req = plain_req();
        req.required_labels = Some(vec!["region=us-east".to_string()]);
        assert_narrow_14b(&[unlabeled, labeled], &req);

        let mut closed = node("wide", Some(70_000), Some(10.0));
        closed.donation_policy = Some(empty_donation());
        let open = node("narrow", Some(25_000), Some(90.0));
        let mut req = plain_req();
        req.task_kind = Some("text_infer".to_string());
        assert_narrow_14b(&[closed, open], &req);
    }

    #[test]
    fn mesh_pick_ignores_total_vram_system_ram_and_min_vram() {
        let mut rich = node("rich", None, Some(1.0));
        rich.gpu_vram_total_mb = Some(80_000);
        rich.memory_free_bytes = Some(80u64 * 1024 * 1024 * 1024);
        rich.capabilities.min_vram_mb = Some(80_000);
        let lean = node("lean", Some(25_000), Some(90.0));
        let mut req = plain_req();
        req.min_vram_mb = Some(70_000);
        let got = select_qwen3_mesh(&[rich, lean], &req, 10_000_000, Some(3_600_000), None)
            .expect("lean pick");
        assert_eq!(got.node_id, "lean");
        assert_eq!(got.size_token, "14B");
        assert_eq!(got.preset, "qwen3_24g");
        assert_eq!(got.floor_mb, 20_000);
    }
}
