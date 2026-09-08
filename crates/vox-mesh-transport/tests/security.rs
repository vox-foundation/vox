//! The four properties the mesh's security model reduces to. Each runs two real
//! iroh endpoints over loopback — a mock of the transport would not exercise
//! the thing under test.

mod common;

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Result;
use iroh::SecretKey;
use tokio::time::timeout;
use vox_mesh_transport::endpoint::{JobExecutor, REFUSED_PROTO, ReceivedJob};
use vox_mesh_transport::protocol::{
    self, ALPN, Hello, Isolation, JobId, JobLimits, JobRequest, JobResponse,
};
use vox_mesh_transport::trust::MeshTrust;
use vox_mesh_types::TaskKind;

use common::{
    MeshTarget, client_endpoint, client_id, loopback_addr_of, send_raw_hello_on, send_request_on,
    send_run_on, send_run_with_claim,
};

/// What [`SpyExecutor`] claims is pending. Not 0: a zero total is what a
/// dropped breakdown and an empty queue look like alike.
const SPY_PENDING: u64 = 3;

/// Counts invocations and remembers the limits it was handed, so a test can
/// assert both "the executor was never reached" and "it ran sandboxed".
#[derive(Default)]
struct SpyExecutor {
    invocations: AtomicUsize,
    last_limits: Mutex<Option<JobLimits>>,
    last_payload: Mutex<Option<Vec<u8>>>,
}

impl SpyExecutor {
    fn invocations(&self) -> usize {
        self.invocations.load(Ordering::SeqCst)
    }
    fn last_limits(&self) -> Option<JobLimits> {
        *self.last_limits.lock().unwrap()
    }
    fn last_payload(&self) -> Vec<u8> {
        self.last_payload
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_default()
    }
}

impl JobExecutor for SpyExecutor {
    fn execute<'a>(
        &'a self,
        job: ReceivedJob,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<JobResponse>> + Send + 'a>> {
        Box::pin(async move {
            self.invocations.fetch_add(1, Ordering::SeqCst);
            *self.last_limits.lock().unwrap() = Some(job.limits);
            *self.last_payload.lock().unwrap() = Some(job.payload);
            // Probe must answer Probed: a directory entry is built from that
            // shape, and a spy that answered Output for everything silently
            // failed the directory tests while the transport was fine.
            Ok(match job.request {
                JobRequest::Probe => JobResponse::Probed {
                    host_triple: "test-triple".to_string(),
                    vox: "0.0.0-test".to_string(),
                    task_kinds: vec![vox_mesh_types::TaskKind::VoxScript],
                    engines: Vec::new(),
                },
                // Likewise for QueueStats: a spy that fell through to Output
                // here would make the queue-stats tests assert on a shape the
                // transport never carried.
                JobRequest::QueueStats => JobResponse::QueueStats(protocol::QueueStats {
                    pending_count: SPY_PENDING,
                    pending_by_kind: vec![(vox_mesh_types::TaskKind::VoxScript, SPY_PENDING)],
                    pending_by_priority: vec![(5, SPY_PENDING)],
                    max_concurrent: 2,
                }),
                _ => JobResponse::Output(b"ok".to_vec()),
            })
        })
    }
}

struct Server {
    id: iroh::EndpointId,
    addr: iroh::EndpointAddr,
    trust: Arc<MeshTrust>,
    spy: Arc<SpyExecutor>,
    exec: Arc<SpyExecutor>,
    _dir: tempfile::TempDir,
}

impl MeshTarget for Server {
    fn mesh_addr(&self) -> iroh::EndpointAddr {
        self.addr.clone()
    }
}

async fn start_server() -> Server {
    let dir = tempfile::tempdir().unwrap();
    let trust = Arc::new(MeshTrust::at(&dir.path().join("mesh_trust.json")));
    let spy = Arc::new(SpyExecutor::default());
    let ep = vox_mesh_transport::endpoint::bind(SecretKey::generate())
        .await
        .expect("bind server");
    let (id, addr) = (ep.id(), ep.addr());
    let exec: Arc<dyn JobExecutor> = spy.clone();
    tokio::spawn(vox_mesh_transport::endpoint::serve(
        ep,
        Arc::clone(&trust),
        exec,
        None,
    ));
    Server {
        id,
        addr,
        trust,
        spy: spy.clone(),
        exec: spy,
        _dir: dir,
    }
}

#[tokio::test]
async fn an_untrusted_peer_cannot_reach_the_executor() {
    let server = start_server().await;
    let client = client_endpoint(SecretKey::generate()).await;

    // No `trust()` call: this peer is a stranger.
    let conn = client
        .connect(server.addr.clone(), ALPN)
        .await
        .expect("connect");

    let result = timeout(
        Duration::from_secs(10),
        send_request_on(&conn, JobRequest::Probe),
    )
    .await
    .expect("no timeout");

    // Non-vacuous on two axes: the executor must not run, AND the request must
    // actually fail. Asserting only the first would still pass if the whole
    // connection had failed for an unrelated reason.
    assert!(
        result.is_err(),
        "a stranger's request must not succeed, got {result:?}"
    );
    assert_eq!(
        server.spy.invocations(),
        0,
        "an untrusted peer must never reach the executor"
    );
    assert!(
        timeout(Duration::from_secs(5), conn.closed()).await.is_ok(),
        "the server must close on a stranger rather than leaving them connected"
    );
}

#[tokio::test]
async fn a_trusted_peer_gets_a_sandbox_by_default() {
    let server = start_server().await;
    server.trust.trust(&client_id(), None).unwrap();
    let resp = send_run_on(&server, JobId(1), TaskKind::VoxScript, &[0u8; 128]).await;

    assert!(matches!(resp, JobResponse::Output(_)), "{resp:?}");
    let limits = server.spy.last_limits().expect("executor saw limits");
    assert_eq!(
        limits.isolation,
        Isolation::Interpreter,
        "pairing must not grant native execution"
    );
    assert!(limits.wall_clock <= Duration::from_secs(300));
}

#[tokio::test]
async fn untrust_closes_a_live_connection() {
    // iroh has no Endpoint-level "close everything to this peer", so MeshTrust
    // holds the handles. Without this, revocation is a file write and nothing
    // else — the peer keeps the connection it already has.
    let server = start_server().await;
    let sk = SecretKey::generate();
    let client_id = sk.public();
    let client = client_endpoint(sk).await;
    server.trust.trust(&client_id, None).unwrap();

    let conn = client
        .connect(server.addr.clone(), ALPN)
        .await
        .expect("connect");
    // Give the server's accept task a moment to register the connection.
    for _ in 0..50 {
        if server.trust.is_trusted(&client_id) {
            tokio::time::sleep(Duration::from_millis(20)).await;
            break;
        }
    }

    server.trust.untrust(&client_id).unwrap();

    assert!(
        timeout(Duration::from_secs(5), conn.closed()).await.is_ok(),
        "untrust must close the live connection, not just edit a file"
    );
}

#[tokio::test]
async fn idle_trusted_connections_release_handshake_permits() {
    let server = start_server().await;
    server.trust.trust(&client_id(), None).unwrap();
    let client = client_endpoint(common::client_sk()).await;
    let mut idle = Vec::new();
    for _ in 0..64 {
        idle.push(
            client
                .connect(server.addr.clone(), ALPN)
                .await
                .expect("idle trusted connection"),
        );
    }
    let active = client
        .connect(server.addr.clone(), ALPN)
        .await
        .expect("connection after idle peers");
    let response = timeout(
        Duration::from_secs(3),
        send_request_on(&active, JobRequest::Probe),
    )
    .await
    .expect("idle trusted connections must not consume handshake permits")
    .expect("probe response");
    assert!(matches!(response, JobResponse::Probed { .. }));
    drop(idle);
}

#[tokio::test]
async fn live_registration_is_removed_after_connection_handler_exits() {
    let server = start_server().await;
    server.trust.trust(&client_id(), None).unwrap();
    let client = client_endpoint(common::client_sk()).await;
    let conn = client
        .connect(server.addr.clone(), ALPN)
        .await
        .expect("connect");
    let response = send_request_on(&conn, JobRequest::Probe)
        .await
        .expect("probe response");
    assert!(matches!(response, JobResponse::Probed { .. }));
    timeout(Duration::from_secs(5), async {
        while server.trust.registered_connections(&client_id()) != 0 {
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("handler exit must unregister the live connection");
}

#[tokio::test]
async fn a_payload_larger_than_the_cap_is_refused_before_any_transfer() {
    let server = start_server().await;
    server.trust.trust(&client_id(), None).unwrap();

    let over = JobLimits::default().max_payload_bytes + 1;
    let resp = send_run_with_claim(&server, JobId(4), TaskKind::VoxScript, b"", over).await;

    match resp {
        JobResponse::Failed(msg) => assert!(msg.contains("exceeds"), "{msg}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
    assert_eq!(
        server.spy.invocations(),
        0,
        "the cap must be enforced before the executor is reached"
    );
}

#[tokio::test]
async fn the_server_id_is_the_public_key() {
    // The EndpointId IS the ed25519 public key; trust is keyed on it, so a
    // divergence here would silently break every allowlist lookup.
    let server = start_server().await;
    assert_eq!(server.id, server.addr.id);
}

// ---------------------------------------------------------------------------
// Task 3.2 acceptance criterion: the peer directory that replaces
// `federation_directory()`. This is goal 4's only real acceptance test, so it
// runs two live endpoints rather than asserting against a mock.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_trusted_probed_peer_appears_and_a_dropped_peer_disappears() {
    let server = start_server().await;
    let sk = SecretKey::generate();
    let client_id = sk.public();
    let client = client_endpoint(sk).await;

    // The directory is built by the CLIENT against peers IT trusts, so the
    // client's store is the one that must name the server.
    let dir = tempfile::tempdir().unwrap();
    let trust = Arc::new(MeshTrust::at(&dir.path().join("mesh_trust.json")));
    trust
        .trust_with_addrs(&server.id, Some("server"), &loopback_addr_of(&server))
        .unwrap();
    // ...and the server must trust the client back, or the probe is refused.
    server.trust.trust(&client_id, None).unwrap();

    let listed = vox_mesh_transport::directory(&client, &trust).await;
    assert_eq!(
        listed.len(),
        1,
        "a trusted, reachable peer must appear: {listed:?}"
    );
    assert_eq!(listed[0].endpoint_id, server.id);
    assert_eq!(listed[0].label.as_deref(), Some("server"));
    assert!(
        !listed[0].host_triple.is_empty(),
        "the entry must carry what the selector routes on"
    );

    // Drop the peer: revoking trust is what an operator does, and the directory
    // must stop offering it immediately.
    trust.untrust(&server.id).unwrap();
    let after = vox_mesh_transport::directory(&client, &trust).await;
    assert!(after.is_empty(), "an untrusted peer must vanish: {after:?}");
}

#[tokio::test]
async fn a_probed_peer_carries_the_addresses_it_was_dialled_on() {
    let server = start_server().await;
    let sk = SecretKey::generate();
    let client = client_endpoint(sk.clone()).await;
    let dir = tempfile::tempdir().unwrap();
    let trust = Arc::new(MeshTrust::at(&dir.path().join("mesh_trust.json")));
    let addrs = loopback_addr_of(&server);
    trust
        .trust_with_addrs(&server.id, Some("server"), &addrs)
        .unwrap();
    server.trust.trust(&sk.public(), None).unwrap();

    let listed = vox_mesh_transport::directory(&client, &trust).await;
    assert_eq!(listed.len(), 1, "the probed peer must appear: {listed:?}");
    assert_eq!(
        listed[0].addrs, addrs,
        "directory must echo the addresses the probe was dialled on"
    );
}

#[tokio::test]
async fn an_unreachable_peer_is_omitted_rather_than_failing_the_whole_directory() {
    // One dark machine must not hide the others -- the old HTTP directory
    // returned a list someone asserted; this one returns what answered.
    let server = start_server().await;
    let client = client_endpoint(SecretKey::generate()).await;
    let dir = tempfile::tempdir().unwrap();
    let trust = Arc::new(MeshTrust::at(&dir.path().join("mesh_trust.json")));

    // A peer that will never answer: valid id, address nothing listens on.
    let ghost = SecretKey::generate().public();
    trust
        .trust_with_addrs(&ghost, Some("ghost"), &["127.0.0.1:1".parse().unwrap()])
        .unwrap();
    trust
        .trust_with_addrs(&server.id, Some("real"), &loopback_addr_of(&server))
        .unwrap();
    server.trust.trust(&client.id(), None).unwrap();

    let listed = vox_mesh_transport::directory(&client, &trust).await;
    assert_eq!(
        listed
            .iter()
            .map(|e| e.label.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("real")],
        "the reachable peer survives the unreachable one: {listed:?}"
    );
}

#[test]
fn a_peer_with_no_stored_address_is_not_dialable() {
    // Regression guard for the mDNS finding: an EndpointId alone cannot be
    // dialed, so pairing MUST capture addresses or the directory is always empty.
    let dir = tempfile::tempdir().unwrap();
    let trust = MeshTrust::at(&dir.path().join("mesh_trust.json"));
    let id = SecretKey::generate().public();
    trust.trust(&id, None).unwrap();
    assert!(
        trust.rows()[0].addrs.is_empty(),
        "plain trust() records no addresses; trust_with_addrs is the pairing path"
    );
}

// ---------------------------------------------------------------------------
// Task 3.3 acceptance criteria: queue depth over the mesh, replacing
// `GET /v1/populi/queue/stats`. The point of the move is that the number comes
// from a peer we probed rather than from a control plane we were told about,
// so both tests run live endpoints.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn queue_depth_comes_from_the_peer_that_answered() {
    let server = start_server().await;
    let sk = SecretKey::generate();
    let client_id = sk.public();
    let client = client_endpoint(sk).await;
    let dir = tempfile::tempdir().unwrap();
    let trust = Arc::new(MeshTrust::at(&dir.path().join("mesh_trust.json")));
    trust
        .trust_with_addrs(&server.id, Some("server"), &loopback_addr_of(&server))
        .unwrap();
    server.trust.trust(&client_id, None).unwrap();

    let totals = vox_mesh_transport::queue_stats(&client, &trust).await;

    assert_eq!(totals.peers_answered, 1, "{totals:?}");
    assert_eq!(
        totals.pending_count, SPY_PENDING,
        "the depth must be the peer's own number, not a local guess"
    );
    // The breakdowns are the Axis-visible half of the contract; a total that
    // survived while they were dropped would still be a regression.
    assert_eq!(
        totals.pending_by_kind,
        vec![(vox_mesh_types::TaskKind::VoxScript, SPY_PENDING)]
    );
    assert_eq!(totals.pending_by_priority, vec![(5u8, SPY_PENDING)]);
}

#[tokio::test]
async fn two_peers_depths_add_up() {
    // Aggregation is the only logic here that is not a round-trip, and a
    // single-peer test cannot tell summing from "take the last answer".
    let a = start_server().await;
    let b = start_server().await;
    let sk = SecretKey::generate();
    let client_id = sk.public();
    let client = client_endpoint(sk).await;
    let dir = tempfile::tempdir().unwrap();
    let trust = Arc::new(MeshTrust::at(&dir.path().join("mesh_trust.json")));
    for s in [&a, &b] {
        trust
            .trust_with_addrs(&s.id, None, &loopback_addr_of(s))
            .unwrap();
        s.trust.trust(&client_id, None).unwrap();
    }

    let totals = vox_mesh_transport::queue_stats(&client, &trust).await;

    assert_eq!(totals.peers_answered, 2, "{totals:?}");
    assert_eq!(totals.pending_count, SPY_PENDING * 2);
    assert_eq!(
        totals.pending_by_kind,
        vec![(vox_mesh_types::TaskKind::VoxScript, SPY_PENDING * 2)]
    );
}

#[tokio::test]
async fn an_untrusted_caller_learns_nothing_about_queue_depth() {
    // Queue depth is a capacity signal. A stranger must not get it, for the
    // same reason they must not get a probe answer.
    let server = start_server().await;
    let client = client_endpoint(SecretKey::generate()).await;
    let dir = tempfile::tempdir().unwrap();
    let trust = Arc::new(MeshTrust::at(&dir.path().join("mesh_trust.json")));
    // The caller trusts the server; the server does NOT trust the caller.
    trust
        .trust_with_addrs(&server.id, Some("server"), &loopback_addr_of(&server))
        .unwrap();

    let totals = vox_mesh_transport::queue_stats(&client, &trust).await;

    assert_eq!(
        totals.peers_answered, 0,
        "an untrusted caller must be told nothing: {totals:?}"
    );
    assert_eq!(totals.pending_count, 0);
    assert_eq!(
        server.spy.invocations(),
        0,
        "the trust gate must sit in front of the executor for QueueStats too"
    );
}

// ---------------------------------------------------------------------------
// Task 8: PROTO 2 — sender job ids, honest version refusal, payload framing
// ---------------------------------------------------------------------------

#[tokio::test]
async fn a_payload_of_exactly_the_declared_size_is_accepted_and_reaches_the_executor() {
    let server = start_server().await;
    server.trust.trust(&client_id(), None).unwrap();
    let payload = b"pub fn main() { print(\"hi\") }".to_vec();
    let resp = send_run_on(&server, JobId(1), TaskKind::VoxScript, &payload).await;
    assert!(matches!(resp, JobResponse::Output(_)), "{resp:?}");
    assert_eq!(server.exec.last_payload(), payload);
}

#[tokio::test]
async fn a_payload_shorter_than_its_claim_is_refused_not_truncated() {
    let server = start_server().await;
    server.trust.trust(&client_id(), None).unwrap();
    let resp = send_run_with_claim(&server, JobId(2), TaskKind::VoxScript, b"short", 999).await;
    assert!(
        matches!(resp, JobResponse::Failed(ref m) if m.contains("claim")),
        "{resp:?}"
    );
}

#[tokio::test]
async fn a_voxscript_payload_over_four_mib_is_refused_before_transfer() {
    let server = start_server().await;
    server.trust.trust(&client_id(), None).unwrap();
    let resp =
        send_run_with_claim(&server, JobId(3), TaskKind::VoxScript, b"", 5 * 1024 * 1024).await;
    assert!(
        matches!(resp, JobResponse::Failed(ref m) if m.contains("exceeds")),
        "{resp:?}"
    );
    assert_eq!(server.exec.invocations(), 0);
}

#[tokio::test]
async fn a_v1_peer_is_told_which_machine_to_upgrade() {
    let server = start_server().await;
    server.trust.trust(&client_id(), None).unwrap();
    let (resp, close) = send_raw_hello_on(
        &server,
        Hello {
            proto: 1,
            ..Hello::current()
        },
    )
    .await;
    assert!(
        matches!(resp, Some(JobResponse::Failed(ref m)) if m.contains("v1") && m.contains("v2")),
        "{resp:?}"
    );
    assert_eq!(close, Some(REFUSED_PROTO));
    assert_eq!(REFUSED_PROTO, 4003);
}

#[test]
fn a_new_trailing_field_is_not_readable_from_an_old_sender() {
    // Same variant index as JobResponse::Probed (0). A dummy at index 0 would
    // shift this to Output; postcard `from_bytes` then succeeds on leftover
    // bytes, which is the masquerade the fixture was written to avoid.
    #[derive(serde::Serialize)]
    enum OldResp {
        Probed {
            host_triple: String,
            vox: String,
            task_kinds: Vec<TaskKind>,
        },
    }
    let bytes = postcard::to_allocvec(&OldResp::Probed {
        host_triple: "t".into(),
        vox: "v".into(),
        task_kinds: vec![],
    })
    .unwrap();
    assert!(
        postcard::from_bytes::<JobResponse>(&bytes).is_err(),
        "#[serde(default)] does not buy wire compatibility under postcard; PROTO does"
    );
}

#[test]
fn isolation_default_is_the_interpreter_and_there_is_no_third_tier() {
    assert_eq!(Isolation::DEFAULT_FOR_MESH, Isolation::Interpreter);
    for v in [Isolation::Interpreter, Isolation::Native] {
        match v {
            Isolation::Interpreter | Isolation::Native => {}
        }
    }
}

#[test]
fn proto_is_two_and_limits_carry_every_bound() {
    assert_eq!(vox_mesh_transport::protocol::PROTO, 2);
    let l = JobLimits::default();
    assert_eq!(l.max_payload_for(TaskKind::VoxScript), 4 * 1024 * 1024);
    assert_eq!(l.max_memory_bytes, 512 * 1024 * 1024);
    assert_eq!(l.max_steps, 50_000_000);
    assert_eq!(l.max_disk_bytes, 32 * 1024 * 1024);
    assert_eq!(l.max_files, 4_096);
    assert!(l.max_concurrent >= 2);
}
