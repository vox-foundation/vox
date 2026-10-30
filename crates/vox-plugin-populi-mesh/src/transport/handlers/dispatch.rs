//! Script dispatch handlers: dispatch_script, dispatch_results_poll, queue_stats, execute_on_worker.

use axum::Json;
use axum::extract::{Extension, State};
use axum::http::StatusCode;
use base64::Engine as _;

use std::collections::HashMap;

use crate::{NodeRecord, node_maintenance_blocks_new_work};
use vox_populi::mens::tensor::size_select::{self, FreeMemory, Selection, SizeRung};
use vox_populi::mens::tensor::spoke_base_resolver::TrainBase;

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

/// One mesh placement of a `qwen3_code` QLoRA size (MESH-01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Qwen3MeshSelection {
    pub node_id: String,
    pub rung: SizeRung,
    pub accelerator_free_mb: u64,
}

/// Why [`select_qwen3_mesh`] refused to name a node.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub(crate) enum Qwen3MeshError {
    /// Every fresh `accelerator_free_mb` reading is above `u32::MAX` MB.
    #[error("accelerator_free_mb {0} does not fit in u32")]
    ReadingExceedsU32(u64),
    /// The phase 318 selector refused (no ladder, nothing fits, typo).
    #[error("{0}")]
    PickFailed(String),
    /// No node has a usable free-memory reading.
    #[error("no fresh accelerator free-memory reading")]
    NoFreshReading,
    /// An explicit size does not fit any eligible node.
    #[error("explicit size does not fit any eligible node")]
    ExplicitDoesNotFit,
    /// An explicit base off the ladder has no floor to route on.
    #[error("explicit base {0} is not a qwen3_code ladder size")]
    NotALadderSize(String),
    /// Fresh readings exist and none holds a ladder floor.
    #[error("no qwen3_code QLoRA floor fits the fresh readings")]
    NothingFits,
}

/// Dispatch-results TTL; the size pick's staleness bound when the window is unset.
const DEFAULT_STALE_WINDOW_MS: u64 = 3_600_000;

fn effective_stale_window_ms(stale_window_ms: Option<u64>) -> u64 {
    match stale_window_ms {
        Some(window) if window > 0 => window,
        _ => DEFAULT_STALE_WINDOW_MS,
    }
}

/// One phase 318 pick (`size_select::select_size`) on `free`. An explicit size
/// is refused, never shrunk, when it does not fit; only the automatic pick
/// steps down.
fn pick_rung(
    overlay: &HashMap<String, Vec<TrainBase>>,
    free: &FreeMemory,
    explicit_size: Option<&str>,
) -> Result<SizeRung, Qwen3MeshError> {
    match size_select::select_size(overlay, Some(free), explicit_size, false) {
        Ok(Selection::Size(pick)) => Ok(pick.rung),
        Ok(Selection::PassThrough(id)) => Err(Qwen3MeshError::NotALadderSize(id)),
        Err(_) if explicit_size.is_some() => Err(Qwen3MeshError::ExplicitDoesNotFit),
        Err(_) => Err(Qwen3MeshError::NothingFits),
    }
}

fn mesh_free(mb: u64) -> FreeMemory {
    FreeMemory {
        mb,
        source: "mesh accelerator_free_mb",
    }
}

/// Largest `qwen3_code` QLoRA size whose floor fits some eligible node's fresh
/// `accelerator_free_mb`, placed on a node that holds that floor (lowest CPU
/// wins a tie). Total VRAM, system RAM and `min_vram_mb` are not inputs.
pub(crate) fn select_qwen3_mesh(
    overlay: &HashMap<String, Vec<TrainBase>>,
    nodes: &[NodeRecord],
    req: &DispatchRequest,
    now_unix_ms: u64,
    stale_window_ms: Option<u64>,
    explicit_size: Option<&str>,
) -> Result<Qwen3MeshSelection, Qwen3MeshError> {
    let window = effective_stale_window_ms(stale_window_ms);
    let candidates: Vec<&NodeRecord> = nodes
        .iter()
        .filter(|node| {
            node.accelerator_free_mb.is_some()
                && now_unix_ms.saturating_sub(node.last_seen_unix_ms) <= window
                && node_passes_dispatch_policy(node, req, now_unix_ms)
        })
        .collect();
    // A reading above u32::MAX MB is garbage (floors are u32): drop it before
    // the max so it cannot win. ReadingExceedsU32 only when every one overflows.
    let usable: Vec<&NodeRecord> = candidates
        .iter()
        .copied()
        .filter(|node| {
            node.accelerator_free_mb
                .is_some_and(|free| u32::try_from(free).is_ok())
        })
        .collect();
    let Some(max_free) = usable.iter().filter_map(|n| n.accelerator_free_mb).max() else {
        return Err(
            match candidates
                .iter()
                .filter_map(|n| n.accelerator_free_mb)
                .max()
            {
                Some(free) => Qwen3MeshError::ReadingExceedsU32(free),
                None => Qwen3MeshError::NoFreshReading,
            },
        );
    };
    let rung = pick_rung(overlay, &mesh_free(max_free), explicit_size)?;
    let floor = u64::from(rung.floor_mb);
    let chosen = usable
        .into_iter()
        .filter(|node| node.accelerator_free_mb.unwrap_or(0) >= floor)
        .min_by(|left, right| {
            let left_cpu = left.cpu_usage_pct.unwrap_or(100.0);
            let right_cpu = right.cpu_usage_pct.unwrap_or(100.0);
            left_cpu
                .partial_cmp(&right_cpu)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .ok_or(Qwen3MeshError::NothingFits)?;
    Ok(Qwen3MeshSelection {
        node_id: chosen.id.clone(),
        rung,
        accelerator_free_mb: chosen.accelerator_free_mb.unwrap_or(0),
    })
}

/// Mesh placement when some node has a fresh reading; otherwise the phase 318
/// pick on the caller-supplied local reading (an explicit size is refused when
/// it does not fit there either). This function does not probe the GPU.
#[derive(Debug)]
pub(crate) enum Qwen3Placement {
    Mesh(Qwen3MeshSelection),
    Local(SizeRung),
}

pub(crate) fn place_qwen3_code_qlora(
    overlay: &HashMap<String, Vec<TrainBase>>,
    nodes: &[NodeRecord],
    req: &DispatchRequest,
    now_unix_ms: u64,
    stale_window_ms: Option<u64>,
    explicit_size: Option<&str>,
    local_free: Option<FreeMemory>,
) -> Result<Qwen3Placement, Qwen3MeshError> {
    match select_qwen3_mesh(
        overlay,
        nodes,
        req,
        now_unix_ms,
        stale_window_ms,
        explicit_size,
    ) {
        Ok(selection) => return Ok(Qwen3Placement::Mesh(selection)),
        Err(Qwen3MeshError::NoFreshReading) => {}
        Err(other) => return Err(other),
    }
    let Some(free) = local_free else {
        return Err(Qwen3MeshError::NoFreshReading);
    };
    pick_rung(overlay, &free, explicit_size).map(Qwen3Placement::Local)
}

#[cfg(test)]
mod qwen3_mesh_tests {
    use super::*;

    // Ladder floors (gpu-specs.yaml, measured on silicon-gpu-metal): 0.6B 15319,
    // 1.7B 27007, 4B 33037. 8B and up are estimates and are not used here.
    const FREE_4B: u64 = 34_000;
    const FREE_1_7B: u64 = 28_000;

    fn overlay() -> HashMap<String, Vec<TrainBase>> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        size_select::load_ladder_overlay(Some(&root)).expect("repo gpu-specs.yaml")
    }

    fn select(
        nodes: &[NodeRecord],
        req: &DispatchRequest,
        window: Option<u64>,
        explicit: Option<&str>,
    ) -> Result<Qwen3MeshSelection, Qwen3MeshError> {
        select_qwen3_mesh(&overlay(), nodes, req, 10_000_000, window, explicit)
    }

    fn place(
        nodes: &[NodeRecord],
        explicit: Option<&str>,
        local_mb: Option<u64>,
    ) -> Result<Qwen3Placement, Qwen3MeshError> {
        place_qwen3_code_qlora(
            &overlay(),
            nodes,
            &plain_req(),
            10_000_000,
            Some(3_600_000),
            explicit,
            local_mb.map(|mb| FreeMemory {
                mb,
                source: "injected local reading",
            }),
        )
    }

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
        record
    }

    fn plain_req() -> DispatchRequest {
        serde_json::from_value(serde_json::json!({ "source": "test" }))
            .expect("minimal DispatchRequest")
    }

    fn assert_rung(rung: &SizeRung, size: &str, preset: &str, floor: u32) {
        assert_eq!(
            (rung.size.as_str(), rung.preset, rung.floor_mb),
            (size, preset, floor)
        );
    }

    fn assert_4b(rung: &SizeRung) {
        assert_rung(rung, "4B", "qwen3_12g", 33_037);
        assert_eq!(
            rung.hf_id,
            "Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c"
        );
    }

    fn assert_1_7b(rung: &SizeRung) {
        assert_rung(rung, "1.7B", "qwen3_6g", 27_007);
    }

    #[test]
    fn mesh_pick_routes_largest_floor_to_the_node_that_holds_it() {
        let nodes = vec![
            node("wide", Some(FREE_4B), Some(50.0)),
            node("narrow", Some(FREE_1_7B), Some(10.0)),
        ];
        let got = select(&nodes, &plain_req(), Some(3_600_000), None).expect("mesh pick");
        assert_eq!(got.node_id, "wide");
        assert_4b(&got.rung);
        assert_eq!(got.accelerator_free_mb, FREE_4B);
        // Same rung the phase 318 selector picks locally on that reading.
        let Selection::Size(local) =
            size_select::select_size(&overlay(), Some(&mesh_free(FREE_4B)), None, false)
                .expect("local pick")
        else {
            panic!("pass-through");
        };
        assert_eq!(got.rung, local.rung);

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
            node("busy", Some(FREE_4B), Some(80.0)),
            node("idle", Some(FREE_4B), Some(10.0)),
        ];
        let got = select(&nodes, &plain_req(), Some(3_600_000), None).expect("tie pick");
        assert_eq!(got.node_id, "idle");
        assert_4b(&got.rung);
    }

    fn assert_narrow_1_7b(nodes: &[NodeRecord], req: &DispatchRequest) {
        let got = select(nodes, req, Some(3_600_000), None).expect("narrow pick");
        assert_eq!(got.node_id, "narrow");
        assert_1_7b(&got.rung);
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
        let mut quarantined = node("wide", Some(FREE_4B), Some(10.0));
        quarantined.quarantined = Some(true);
        let narrow = node("narrow", Some(FREE_1_7B), Some(90.0));
        assert_narrow_1_7b(&[quarantined, narrow], &plain_req());

        let mut maintained = node("wide", Some(FREE_4B), Some(10.0));
        maintained.maintenance = Some(true);
        maintained.maintenance_until_unix_ms = Some(10_000_001);
        let narrow = node("narrow", Some(FREE_1_7B), Some(90.0));
        assert_narrow_1_7b(&[maintained, narrow], &plain_req());

        let unlabeled = node("wide", Some(FREE_4B), Some(10.0));
        let mut labeled = node("narrow", Some(FREE_1_7B), Some(90.0));
        labeled
            .capabilities
            .labels
            .push("region=us-east".to_string());
        let mut req = plain_req();
        req.required_labels = Some(vec!["region=us-east".to_string()]);
        assert_narrow_1_7b(&[unlabeled, labeled], &req);

        let mut closed = node("wide", Some(FREE_4B), Some(10.0));
        closed.donation_policy = Some(empty_donation());
        let open = node("narrow", Some(FREE_1_7B), Some(90.0));
        let mut req = plain_req();
        req.task_kind = Some("text_infer".to_string());
        assert_narrow_1_7b(&[closed, open], &req);
    }

    #[test]
    fn mesh_pick_ignores_total_vram_system_ram_and_min_vram() {
        let mut rich = node("rich", None, Some(1.0));
        rich.gpu_vram_total_mb = Some(80_000);
        rich.memory_free_bytes = Some(80u64 * 1024 * 1024 * 1024);
        rich.capabilities.min_vram_mb = Some(80_000);
        let lean = node("lean", Some(FREE_1_7B), Some(90.0));
        let mut req = plain_req();
        req.min_vram_mb = Some(70_000);
        let got = select(&[rich, lean], &req, Some(3_600_000), None).expect("lean pick");
        assert_eq!(got.node_id, "lean");
        assert_1_7b(&got.rung);
    }

    fn stale_trio(wide_last_seen: u64) -> Vec<NodeRecord> {
        let mut wide = node("wide", Some(FREE_4B), Some(1.0));
        wide.last_seen_unix_ms = wide_last_seen;
        let mut missing = node("missing", None, Some(1.0));
        missing.gpu_vram_total_mb = Some(80_000);
        missing.memory_free_bytes = Some(80u64 * 1024 * 1024 * 1024);
        let fresh = node("fresh", Some(FREE_1_7B), Some(90.0));
        vec![wide, missing, fresh]
    }

    #[test]
    fn mesh_pick_ignores_stale_and_missing_free_memory() {
        let nodes = stale_trio(6_399_999);
        let got = select(&nodes, &plain_req(), Some(3_600_000), None).expect("fresh 1.7B");
        assert_eq!(got.node_id, "fresh");
        assert_1_7b(&got.rung);
        let excluded = vec![nodes[0].clone(), nodes[1].clone()];
        assert_eq!(
            select(&excluded, &plain_req(), Some(3_600_000), None).expect_err("no fresh reading"),
            Qwen3MeshError::NoFreshReading
        );
    }

    #[test]
    fn mesh_pick_unset_window_uses_dispatch_hour() {
        let nodes = stale_trio(6_399_999);
        let got = select(&nodes, &plain_req(), None, None).expect("still 1.7B");
        assert_eq!(got.node_id, "fresh");
        assert_1_7b(&got.rung);
        let on_the_hour = stale_trio(6_400_000);
        let got = select(&on_the_hour, &plain_req(), None, None).expect("hour-old 4B");
        assert_eq!(got.node_id, "wide");
        assert_4b(&got.rung);
        let got =
            select(&on_the_hour, &plain_req(), Some(0), None).expect("zero window uses the hour");
        assert_eq!(got.node_id, "wide");
        assert_4b(&got.rung);
    }

    #[test]
    fn mesh_pick_explicit_size_that_no_node_holds_is_an_error() {
        let nodes = vec![node("narrow", Some(FREE_1_7B), Some(10.0))];
        let err = select(&nodes, &plain_req(), Some(3_600_000), Some("Qwen3-4B"))
            .expect_err("4B does not fit");
        assert_eq!(err, Qwen3MeshError::ExplicitDoesNotFit);
        let got = select(&nodes, &plain_req(), Some(3_600_000), None).expect("automatic step-down");
        assert_1_7b(&got.rung);
    }

    #[test]
    fn mesh_pick_drops_overflowing_free_memory_reading() {
        let nodes = vec![
            node("overflow", Some(u64::MAX), Some(1.0)),
            node("usable", Some(FREE_1_7B), Some(40.0)),
        ];
        let got = select(&nodes, &plain_req(), Some(3_600_000), None).expect("usable sibling");
        assert_eq!(got.node_id, "usable");
        assert_1_7b(&got.rung);
        assert_eq!(got.accelerator_free_mb, FREE_1_7B);

        let only_overflow = vec![node("overflow", Some(u64::MAX), Some(1.0))];
        let err = select(&only_overflow, &plain_req(), Some(3_600_000), None)
            .expect_err("every fresh reading overflows u32");
        assert_eq!(err, Qwen3MeshError::ReadingExceedsU32(u64::MAX));
    }

    #[test]
    fn mesh_pick_explicit_size_routes_to_a_node_that_holds_it() {
        let nodes = vec![
            node("wide", Some(FREE_4B), Some(40.0)),
            node("narrow", Some(FREE_1_7B), Some(5.0)),
        ];
        let got =
            select(&nodes, &plain_req(), Some(3_600_000), Some("Qwen3-4B")).expect("explicit 4B");
        assert_eq!(got.node_id, "wide");
        assert_4b(&got.rung);
    }

    #[test]
    fn mesh_pick_explicit_off_ladder_base_is_refused() {
        let nodes = vec![node("wide", Some(FREE_4B), Some(40.0))];
        let err = select(&nodes, &plain_req(), Some(3_600_000), Some("gpt2"))
            .expect_err("no floor to route on");
        assert_eq!(err, Qwen3MeshError::NotALadderSize("gpt2".to_string()));
    }

    #[test]
    fn select_best_node_ignores_accelerator_free_mb() {
        let nodes = vec![
            node("busy", Some(FREE_4B), Some(90.0)),
            node("idle", None, Some(10.0)),
        ];
        let best = select_best_node(&nodes, &plain_req()).expect("script dispatch");
        assert_eq!(best.id, "idle");
        let mesh = select(&nodes, &plain_req(), Some(3_600_000), None).expect("mesh pick");
        assert_eq!(mesh.node_id, "busy");
        assert_4b(&mesh.rung);
    }

    fn local(placement: Qwen3Placement) -> SizeRung {
        match placement {
            Qwen3Placement::Local(rung) => rung,
            Qwen3Placement::Mesh(selection) => panic!("named {}", selection.node_id),
        }
    }

    #[test]
    fn place_qwen3_empty_registry_uses_local_reading() {
        assert_4b(&local(place(&[], None, Some(FREE_4B)).expect("local 4B")));
    }

    #[test]
    fn place_qwen3_no_fresh_reading_uses_local_reading() {
        let nodes = vec![node("unread", None, Some(10.0))];
        assert_1_7b(&local(
            place(&nodes, None, Some(FREE_1_7B)).expect("local 1.7B"),
        ));
    }

    #[test]
    fn place_qwen3_without_local_reading_does_not_probe() {
        assert_eq!(
            place(&[], None, None).expect_err("no reading"),
            Qwen3MeshError::NoFreshReading
        );
    }

    #[test]
    fn place_qwen3_only_this_node_is_a_mesh_pick() {
        let nodes = vec![node("local-1", Some(FREE_4B), Some(10.0))];
        match place(&nodes, None, Some(FREE_1_7B)).expect("mesh pick of this node") {
            Qwen3Placement::Mesh(selection) => {
                assert_eq!(selection.node_id, "local-1");
                assert_4b(&selection.rung);
            }
            Qwen3Placement::Local(rung) => panic!("local {}", rung.size),
        }
    }

    #[test]
    fn place_qwen3_explicit_miss_does_not_fall_back_local() {
        let nodes = vec![node("narrow", Some(FREE_1_7B), Some(5.0))];
        assert_eq!(
            place(&nodes, Some("Qwen3-4B"), Some(FREE_4B)).expect_err("explicit miss"),
            Qwen3MeshError::ExplicitDoesNotFit
        );
    }

    #[test]
    fn place_qwen3_empty_explicit_4b_does_not_step_down() {
        assert_eq!(
            place(&[], Some("Qwen3-4B"), Some(FREE_1_7B)).expect_err("4B does not fit 28000"),
            Qwen3MeshError::ExplicitDoesNotFit
        );
    }

    #[test]
    fn place_qwen3_empty_explicit_1_7b_does_not_step_up() {
        assert_1_7b(&local(
            place(&[], Some("Qwen3-1.7B"), Some(FREE_4B)).expect("explicit 1.7B"),
        ));
    }

    fn stale_or_missing_nodes() -> Vec<NodeRecord> {
        let mut stale = node("stale", Some(FREE_4B), Some(1.0));
        stale.last_seen_unix_ms = 1;
        vec![stale, node("missing", None, Some(1.0))]
    }

    #[test]
    fn place_qwen3_stale_or_missing_explicit_size_uses_local_rule() {
        assert_eq!(
            place(&stale_or_missing_nodes(), Some("Qwen3-4B"), Some(FREE_1_7B))
                .expect_err("stale registry must not step an explicit size down"),
            Qwen3MeshError::ExplicitDoesNotFit
        );
        assert_1_7b(&local(
            place(&stale_or_missing_nodes(), Some("Qwen3-1.7B"), Some(FREE_4B))
                .expect("explicit 1.7B on the local reading"),
        ));
    }
}
