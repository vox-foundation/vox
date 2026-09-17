//! Shared live-endpoint helpers for mesh-transport integration tests.
//!
//! Each integration test binary compiles this module on its own, so helpers
//! used only by `interp_executor` look unused from `security`.
#![allow(dead_code)]

use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use iroh::{Endpoint, EndpointId, SecretKey};
use tokio::time::timeout;
use vox_mesh_transport::endpoint::{JobExecutor, ReceivedJob};
use vox_mesh_transport::protocol::{self, ALPN, Hello, JobId, JobRequest, JobResponse};
use vox_mesh_transport::trust::{MeshTrust, TrustLevel};
use vox_mesh_types::TaskKind;

/// Answers Probe / QueueStats / Run without executing. Used by tests that only
/// need `serve` to stay up (mailbox) or that record invocations themselves.
#[derive(Debug, Default)]
pub struct SpyExecutor;

impl JobExecutor for SpyExecutor {
    fn execute<'a>(
        &'a self,
        job: ReceivedJob,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<JobResponse>> + Send + 'a>> {
        Box::pin(async move {
            Ok(match job.request {
                JobRequest::Probe => JobResponse::Probed {
                    host_triple: "test-triple".to_string(),
                    vox: "0.0.0-test".to_string(),
                    task_kinds: vec![TaskKind::VoxScript],
                    engines: Vec::new(),
                },
                JobRequest::QueueStats => JobResponse::QueueStats(protocol::QueueStats::default()),
                JobRequest::Cancel { .. } => JobResponse::Failed("nothing to cancel".to_string()),
                JobRequest::Run { .. } => JobResponse::Output(b"ok".to_vec()),
            })
        })
    }
}

/// A bound server plus the trust store the tests mutate.
#[derive(Clone)]
pub struct Server {
    pub id: EndpointId,
    pub addr: iroh::EndpointAddr,
    pub trust: Arc<MeshTrust>,
    _dir: Arc<tempfile::TempDir>,
}

/// Anything that can be dialed as a mesh job server.
pub trait MeshTarget {
    fn mesh_addr(&self) -> iroh::EndpointAddr;
}

impl MeshTarget for Server {
    fn mesh_addr(&self) -> iroh::EndpointAddr {
        self.addr.clone()
    }
}

/// Stable test-client identity so [`send_run_on`] and `trust(&client_id())`
/// name the same peer.
pub fn client_sk() -> SecretKey {
    SecretKey::from_bytes(&[11u8; 32])
}

pub fn client_sk_a() -> SecretKey {
    client_sk()
}

pub fn client_sk_b() -> SecretKey {
    SecretKey::from_bytes(&[22u8; 32])
}

pub fn client_id() -> EndpointId {
    client_sk().public()
}

/// A client endpoint. Built with the same `Minimal` preset, so nothing in this
/// test suite reaches a relay or a DNS server.
pub async fn client_endpoint(sk: SecretKey) -> Endpoint {
    Endpoint::builder(iroh::endpoint::presets::Minimal)
        .secret_key(sk)
        .bind()
        .await
        .expect("bind client")
}

pub async fn start_server_with(
    make: impl FnOnce(Arc<MeshTrust>) -> Arc<dyn JobExecutor>,
) -> Server {
    let dir = tempfile::tempdir().unwrap();
    let trust = Arc::new(MeshTrust::at(&dir.path().join("mesh_trust.json")));
    let exec = make(Arc::clone(&trust));
    let ep = vox_mesh_transport::endpoint::bind(SecretKey::generate())
        .await
        .expect("bind server");
    let (id, addr) = (ep.id(), ep.addr());
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
        _dir: Arc::new(dir),
    }
}

/// Trust `id` at an explicit level. Pairing still goes through [`MeshTrust::trust`].
pub fn trust_with(trust: &MeshTrust, id: &EndpointId, level: TrustLevel) -> anyhow::Result<()> {
    match level {
        TrustLevel::Sandboxed => trust.trust(id, None),
        TrustLevel::Native => trust.grant_native(id, None),
    }
}

/// The server's port on loopback.
///
/// `Endpoint::addr()` advertises LAN/VPN addresses, never loopback, and dialing
/// this host's own LAN IP is both slow and environment-dependent. The endpoint
/// binds `0.0.0.0`, so loopback reaches it and the test stays hermetic.
pub fn loopback_addr_of(server: &impl MeshTarget) -> Vec<std::net::SocketAddr> {
    let port = server
        .mesh_addr()
        .ip_addrs()
        .next()
        .expect("a bound endpoint advertises at least one address")
        .port();
    vec![format!("127.0.0.1:{port}").parse().unwrap()]
}

pub async fn send_request_on(
    conn: &iroh::endpoint::Connection,
    request: JobRequest,
) -> Result<JobResponse> {
    let (mut send, mut recv) = conn.open_bi().await?;
    protocol::write_frame(&mut send, &Hello::current()).await?;
    protocol::write_frame(&mut send, &request).await?;
    send.finish()?;
    protocol::read_frame(&mut recv, 16 * 1024 * 1024).await
}

pub async fn send_run_on(
    server: &impl MeshTarget,
    job_id: JobId,
    kind: TaskKind,
    payload: &[u8],
) -> JobResponse {
    send_run_with_claim(server, job_id, kind, payload, payload.len() as u64).await
}

pub async fn send_run_on_owned(
    server: Server,
    job_id: JobId,
    kind: TaskKind,
    payload: &[u8],
) -> JobResponse {
    send_run_on(&server, job_id, kind, payload).await
}

pub async fn send_run_with_claim(
    server: &impl MeshTarget,
    job_id: JobId,
    kind: TaskKind,
    payload: &[u8],
    claim: u64,
) -> JobResponse {
    let client = client_endpoint(client_sk()).await;
    let conn = client
        .connect(server.mesh_addr(), ALPN)
        .await
        .expect("connect");
    timeout(
        Duration::from_secs(60),
        send_run_on_conn(&conn, job_id, kind, payload, claim),
    )
    .await
    .expect("no timeout")
    .expect("response")
}

pub async fn send_run_from_owned(
    client: Endpoint,
    server: Server,
    job_id: JobId,
    kind: TaskKind,
    payload: &[u8],
) -> JobResponse {
    send_run_from(client, &server, job_id, kind, payload).await
}

pub async fn send_run_from(
    client: Endpoint,
    server: &impl MeshTarget,
    job_id: JobId,
    kind: TaskKind,
    payload: &[u8],
) -> JobResponse {
    let conn = client
        .connect(server.mesh_addr(), ALPN)
        .await
        .expect("connect");
    timeout(
        Duration::from_secs(60),
        send_run_on_conn(&conn, job_id, kind, payload, payload.len() as u64),
    )
    .await
    .expect("no timeout")
    .expect("response")
}

pub async fn send_cancel_from(
    client: Endpoint,
    server: &impl MeshTarget,
    job_id: JobId,
) -> JobResponse {
    let conn = client
        .connect(server.mesh_addr(), ALPN)
        .await
        .expect("connect");
    timeout(
        Duration::from_secs(10),
        send_request_on(&conn, JobRequest::Cancel { job_id }),
    )
    .await
    .expect("no timeout")
    .expect("response")
}

pub async fn send_run_on_conn(
    conn: &iroh::endpoint::Connection,
    job_id: JobId,
    kind: TaskKind,
    payload: &[u8],
    claim: u64,
) -> Result<JobResponse> {
    let (mut send, mut recv) = conn.open_bi().await?;
    protocol::write_frame(&mut send, &Hello::current()).await?;
    protocol::write_frame(
        &mut send,
        &JobRequest::Run {
            job_id,
            kind,
            payload_bytes: claim,
        },
    )
    .await?;
    protocol::write_frame(&mut send, &payload.to_vec()).await?;
    send.finish()?;
    protocol::read_frame(&mut recv, 16 * 1024 * 1024).await
}

pub async fn send_raw_hello_on(
    server: &impl MeshTarget,
    hello: Hello,
) -> (Option<JobResponse>, Option<u32>) {
    let client = client_endpoint(client_sk()).await;
    let conn = client
        .connect(server.mesh_addr(), ALPN)
        .await
        .expect("connect");
    let (mut send, mut recv) = conn.open_bi().await.expect("open_bi");
    protocol::write_frame(&mut send, &hello)
        .await
        .expect("write hello");
    let _ = send.finish();
    let resp = timeout(
        Duration::from_secs(10),
        protocol::read_frame(&mut recv, 16 * 1024 * 1024),
    )
    .await
    .ok()
    .and_then(Result::ok);
    let close = timeout(Duration::from_secs(5), conn.closed())
        .await
        .ok()
        .and_then(|err| match err {
            iroh::endpoint::ConnectionError::ApplicationClosed(c) => {
                Some(c.error_code.into_inner() as u32)
            }
            _ => None,
        });
    (resp, close)
}
