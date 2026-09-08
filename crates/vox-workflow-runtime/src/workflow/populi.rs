//! Populi / mens activity steps (feature `mens`).
//!
//! [`PopuliHttpOp`] is a Vox `activity` **language surface**, so it is ported
//! onto the iroh mesh rather than retired (plan Task 0.3 Step 5 / Task 3.4).
//! Every op runs on the mesh. No HTTP, no control plane:
//!
//! - **`Noop`, `Snapshot`, `Join`, `Heartbeat`.** `Snapshot` is
//!   `vox_mesh_transport::directory()` — peers that answered a `Probe` just
//!   now, not a list somebody asserted.
//! - **`Dispatch`** first-fits a `VoxScript` peer and sends `JobRequest::Run`.
//!   [`PopuliActivity`] has no source field, so this errors until inline
//!   source exists; it does **not** synthesize
//!   `workflow_durable_shim::execute_activity`.
//! - **`Wait`** completes inline (`completed_inline`) — there is no HTTP poll.
//!
//! `Join` and `Heartbeat` have no mesh equivalent to *perform*: on iroh there
//! is no control plane to register with — membership **is** pairing
//! (`vox mesh join <ticket>`) and reachability is **demonstrated** by the
//! probe, never asserted by a POST. So they report that state plainly instead
//! of emitting a `join_ok` / `heartbeat_ok` for something that did not happen.
//!
//! The `event` / `activity` / `activity_id` / `mesh_op` keys are an observable
//! contract of the language surface and are unchanged.

#[cfg(feature = "mens")]
use std::sync::atomic::{AtomicU64, Ordering};

#[cfg(feature = "mens")]
use anyhow::anyhow;
#[cfg(feature = "mens")]
use iroh::EndpointAddr;
#[cfg(feature = "mens")]
use serde_json::{Value, json};
#[cfg(feature = "mens")]
use vox_mesh_transport::PeerEntry;
#[cfg(feature = "mens")]
use vox_mesh_transport::protocol::{self, Hello, JobId, JobRequest, JobResponse};

#[cfg(feature = "mens")]
use super::types::{PopuliActivity, PopuliHttpOp};

/// Execute one mens activity step on the iroh mesh.
#[cfg(feature = "mens")]
pub async fn execute_populi_step(activity: &PopuliActivity) -> anyhow::Result<Value> {
    let _ = vox_populi::publish_local_registry_best_effort();
    match activity.populi_op {
        PopuliHttpOp::Noop
        | PopuliHttpOp::Join
        | PopuliHttpOp::Snapshot
        | PopuliHttpOp::Heartbeat => Ok(execute_mesh_step(activity).await),
        PopuliHttpOp::Dispatch => execute_mesh_dispatch(activity).await,
        PopuliHttpOp::Wait => Ok(wait_envelope(activity)),
    }
}

/// The mesh plane. Never fails: an absent or unreadable mesh means "no peers",
/// which is a fact about the network, not an error in the workflow.
#[cfg(feature = "mens")]
async fn execute_mesh_step(activity: &PopuliActivity) -> Value {
    let op = activity.populi_op;
    let control = mesh_control_label(op);
    match op {
        PopuliHttpOp::Noop => mesh_envelope(activity, control, json!({})),
        PopuliHttpOp::Join => mesh_envelope(
            activity,
            control,
            json!({
                // Read from the key, not from a bound endpoint: answering "who
                // am I on the mesh" must not open sockets.
                "endpoint_id": vox_mesh_transport::load_or_create(&vox_dir().join("mesh.key"))
                    .ok()
                    .map(|sk| sk.public().to_string()),
                "trusted_peers": mesh_trust().rows().len(),
                "detail": "iroh has no control plane to register with: membership is pairing. \
                           Run `vox mesh join <ticket>` to pair with a peer.",
            }),
        ),
        // Both are "who is actually out there right now". Snapshot asks it as a
        // directory listing; Heartbeat asks it as a liveness check, which on a
        // probe-based mesh is the same round-trip.
        PopuliHttpOp::Snapshot | PopuliHttpOp::Heartbeat => {
            let peers = probed_peers().await;
            mesh_envelope(
                activity,
                control,
                json!({
                    "node_count": peers.len(),
                    "peers": peers,
                    "detail": "peers that answered a Probe just now; a trusted peer that is \
                               switched off is absent rather than listed",
                }),
            )
        }
        PopuliHttpOp::Dispatch | PopuliHttpOp::Wait => {
            unreachable!("Dispatch and Wait are routed by execute_populi_step")
        }
    }
}

/// Sender-assigned job id. Local to this process; the peer keys running work
/// by `(EndpointId, JobId)`, so two clients' counters cannot collide.
#[cfg(feature = "mens")]
fn next_local_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(1);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// [`PopuliActivity`] carries no source. Inventing one was the HTTP shim.
#[cfg(feature = "mens")]
fn dispatchable_source(activity: &PopuliActivity) -> anyhow::Result<Vec<u8>> {
    Err(anyhow!(
        "activity `{}` has no dispatchable source; inline source is required for mesh dispatch",
        activity.name
    ))
}

/// First-fit a `VoxScript` peer and `Run` the activity source on it.
#[cfg(feature = "mens")]
async fn execute_mesh_dispatch(activity: &PopuliActivity) -> anyhow::Result<Value> {
    let source = dispatchable_source(activity)?;
    let Some(ep) = mesh_endpoint().await else {
        return Err(anyhow!(
            "mesh endpoint unavailable for activity `{}`",
            activity.name
        ));
    };
    let trust = std::sync::Arc::new(mesh_trust());
    let peers = vox_mesh_transport::directory(ep, &trust).await;
    // first-fit, no queue-depth weighting. Phase 4 Task 4.1
    // replaces this with a PlacementRecord.
    let candidates: Vec<PeerEntry> = peers
        .into_iter()
        .filter(|p| p.task_kinds.iter().any(|k| k.as_str() == "vox_script"))
        .collect();
    let n = candidates.len();
    let Some(peer) = candidates.into_iter().next() else {
        return Err(anyhow!(
            "no mesh peer offers VoxScript for activity `{}`",
            activity.name
        ));
    };
    match run_on_peer(ep, &peer, &source).await? {
        JobResponse::Output(bytes) => Ok(mesh_envelope(
            activity,
            "dispatch_ok",
            json!({
                "peer": peer.endpoint_id.to_string(),
                "candidates": n,
                "success": true,
                "result_output": String::from_utf8_lossy(&bytes),
                "exit_code": 0,
            }),
        )),
        JobResponse::Failed(msg) => Err(anyhow!(
            "mesh dispatch failed for activity `{}`: {msg}",
            activity.name
        )),
        other => Err(anyhow!(
            "mesh dispatch for activity `{}` got unexpected response: {other:?}",
            activity.name
        )),
    }
}

/// Send `JobRequest::Run` to `peer` and read the response (16 MiB frame max).
#[cfg(feature = "mens")]
async fn run_on_peer(
    ep: &iroh::Endpoint,
    peer: &PeerEntry,
    payload: &[u8],
) -> anyhow::Result<JobResponse> {
    let mut addr = EndpointAddr::new(peer.endpoint_id);
    for a in &peer.addrs {
        addr = addr.with_ip_addr(*a);
    }
    let kind = peer
        .task_kinds
        .iter()
        .find(|k| k.as_str() == "vox_script")
        .cloned()
        .expect("first-fit already required VoxScript");
    let conn = ep.connect(addr, protocol::ALPN).await?;
    let (mut send, mut recv) = conn.open_bi().await?;
    protocol::write_frame(&mut send, &Hello::current()).await?;
    protocol::write_frame(
        &mut send,
        &JobRequest::Run {
            job_id: JobId(next_local_id()),
            kind,
            payload_bytes: payload.len() as u64,
        },
    )
    .await?;
    protocol::write_frame(&mut send, &payload.to_vec()).await?;
    send.finish()?;
    let resp = protocol::read_frame(&mut recv, 16 * 1024 * 1024).await?;
    conn.close(0u32.into(), b"asked");
    Ok(resp)
}

/// `Wait` does not poll HTTP. The prior step's result keys stay on the envelope.
#[cfg(feature = "mens")]
fn wait_envelope(activity: &PopuliActivity) -> Value {
    mesh_envelope(
        activity,
        "completed_inline",
        json!({
            "success": true,
            "result_output": "",
            "exit_code": 0,
        }),
    )
}

/// Merge `extra` into the frozen `MeshActivity` envelope.
///
/// One place so the four contract keys cannot drift per op.
#[cfg(feature = "mens")]
fn mesh_envelope(activity: &PopuliActivity, control: &str, extra: Value) -> Value {
    let mut v = json!({
        "event": "MeshActivity",
        "activity": activity.name,
        "activity_id": activity.activity_id,
        "mesh_op": populi_op_json(activity.populi_op),
        "control": control,
    });
    if let (Some(obj), Value::Object(extra)) = (v.as_object_mut(), extra) {
        obj.extend(extra);
    }
    v
}

/// What the mesh plane honestly did, per op.
#[cfg(feature = "mens")]
fn mesh_control_label(op: PopuliHttpOp) -> &'static str {
    match op {
        PopuliHttpOp::Noop => "noop",
        // Not `join_ok`: nothing was joined. Pairing is out of band.
        PopuliHttpOp::Join => "pairing_is_out_of_band",
        PopuliHttpOp::Snapshot => "snapshot_ok",
        // Not `heartbeat_ok`: nobody was told we are alive. We asked instead.
        PopuliHttpOp::Heartbeat => "reachability_probed",
        PopuliHttpOp::Dispatch => "dispatch_ok",
        PopuliHttpOp::Wait => "completed_inline",
    }
}

/// `~/.vox` — must match where `vox mesh join` writes, or pairing and workflow
/// activities disagree about which peers exist.
#[cfg(feature = "mens")]
fn vox_dir() -> std::path::PathBuf {
    vox_config::paths::dot_vox_user_dir()
}

#[cfg(feature = "mens")]
fn mesh_trust() -> vox_mesh_transport::MeshTrust {
    vox_mesh_transport::MeshTrust::at(&vox_dir().join("mesh_trust.json"))
}

/// Bound once per process.
///
/// Binding an iroh endpoint opens sockets and starts background tasks, which is
/// far too heavy to repeat per activity step.
///
// vox:defactored-from vox-orchestrator 2026-09-05 — the same bind-once wrapper
// as `vox-orchestrator/src/models/mesh_directory.rs`, duplicated (25 lines)
// rather than taking a vox-workflow-runtime -> vox-orchestrator crate edge.
#[cfg(feature = "mens")]
static MESH_ENDPOINT: tokio::sync::OnceCell<Option<iroh::Endpoint>> =
    tokio::sync::OnceCell::const_new();

#[cfg(feature = "mens")]
async fn mesh_endpoint() -> Option<&'static iroh::Endpoint> {
    MESH_ENDPOINT
        .get_or_init(|| async {
            let sk = match vox_mesh_transport::load_or_create(&vox_dir().join("mesh.key")) {
                Ok(sk) => sk,
                Err(e) => {
                    tracing::warn!(target: "vox.workflow.mesh", error = %e, "mesh identity unavailable");
                    return None;
                }
            };
            match vox_mesh_transport::bind(sk).await {
                Ok(ep) => Some(ep),
                Err(e) => {
                    tracing::warn!(target: "vox.workflow.mesh", error = %e, "mesh endpoint bind failed");
                    None
                }
            }
        })
        .await
        .as_ref()
}

/// Trusted peers that answered a `Probe`. Empty on any failure — an activity
/// must never fail because a peer is switched off.
#[cfg(feature = "mens")]
async fn probed_peers() -> Vec<Value> {
    let Some(ep) = mesh_endpoint().await else {
        return Vec::new();
    };
    let trust = std::sync::Arc::new(mesh_trust());
    vox_mesh_transport::directory(ep, &trust)
        .await
        .into_iter()
        .map(|p| {
            json!({
                "endpoint_id": p.endpoint_id.to_string(),
                "label": p.label,
                "host_triple": p.host_triple,
                "vox": p.vox,
            })
        })
        .collect()
}

#[cfg(feature = "mens")]
fn populi_op_json(op: PopuliHttpOp) -> &'static str {
    match op {
        PopuliHttpOp::Heartbeat => "heartbeat",
        PopuliHttpOp::Noop => "noop",
        PopuliHttpOp::Join => "join",
        PopuliHttpOp::Snapshot => "snapshot",
        PopuliHttpOp::Dispatch => "dispatch",
        PopuliHttpOp::Wait => "wait",
    }
}

#[cfg(all(test, feature = "mens"))]
mod tests {
    use super::*;

    fn activity(op: PopuliHttpOp) -> PopuliActivity {
        PopuliActivity {
            name: "mesh_thing".into(),
            populi_op: op,
            timeout_ms: None,
            activity_id: "act-1".into(),
            required_labels: None,
            is_detached: false,
        }
    }

    fn sample_activity() -> PopuliActivity {
        activity(PopuliHttpOp::Wait)
    }

    // PopuliActivity has no source field — mesh Dispatch cannot invent one
    // (the old HTTP plane synthesized `workflow_durable_shim::execute_activity`).
    #[tokio::test]
    #[ignore = "owner:mesh sunset:2026-12-31 slow: spawns the vox binary via InterpExecutor"]
    async fn dispatch_runs_real_source_on_a_loopback_peer() {
        let activity = PopuliActivity {
            name: "mesh_dispatch".into(),
            populi_op: PopuliHttpOp::Dispatch,
            timeout_ms: None,
            activity_id: "act-1".into(),
            required_labels: None,
            is_detached: false,
        };
        let err = execute_populi_step(&activity)
            .await
            .expect_err("Dispatch with no source must fail")
            .to_string();
        assert!(
            err.contains("activity `mesh_dispatch` has no dispatchable source"),
            "{err}"
        );
        assert!(
            err.contains("inline source is required for mesh dispatch"),
            "{err}"
        );
    }

    #[test]
    fn wait_is_inline_and_keeps_the_result_keys() {
        let env = wait_envelope(&sample_activity());
        assert_eq!(env["control"], "completed_inline");
        assert_eq!(env["success"], true);
        assert!(env.get("result_output").is_some() && env.get("exit_code").is_some());
    }

    // `event` / `activity` / `activity_id` / `mesh_op` are an observable
    // contract of the `activity` language surface. A port that renames them
    // breaks user workflows, so pin them for every mesh-plane op.
    #[test]
    fn every_mesh_envelope_keeps_the_contract_keys() {
        for op in [
            PopuliHttpOp::Noop,
            PopuliHttpOp::Join,
            PopuliHttpOp::Snapshot,
            PopuliHttpOp::Heartbeat,
        ] {
            let v = mesh_envelope(&activity(op), "whatever", json!({}));
            assert_eq!(v["event"], "MeshActivity");
            assert_eq!(v["activity"], "mesh_thing");
            assert_eq!(v["activity_id"], "act-1");
            assert_eq!(v["mesh_op"], populi_op_json(op));
        }
    }

    #[test]
    fn extra_fields_merge_into_the_envelope() {
        let v = mesh_envelope(
            &activity(PopuliHttpOp::Snapshot),
            "snapshot_ok",
            json!({"node_count": 3}),
        );
        assert_eq!(v["control"], "snapshot_ok");
        assert_eq!(v["node_count"], 3);
    }

    // On iroh there is no control plane to register with: membership *is*
    // pairing, and reachability is demonstrated by a probe. Emitting `join_ok`
    // or `heartbeat_ok` would claim an acknowledgement nobody sent.
    #[test]
    fn join_and_heartbeat_never_claim_a_control_plane_ack() {
        for op in [PopuliHttpOp::Join, PopuliHttpOp::Heartbeat] {
            let v = mesh_envelope(&activity(op), mesh_control_label(op), json!({}));
            let control = v["control"].as_str().unwrap();
            assert_ne!(control, "join_ok");
            assert_ne!(control, "heartbeat_ok");
        }
    }
}
