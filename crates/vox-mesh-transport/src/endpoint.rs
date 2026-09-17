//! Endpoint construction and the bounded, trust-gated accept loop.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use iroh::endpoint::{Connection, NetReportConfig, presets};
use iroh::{Endpoint, EndpointId, SecretKey};
use tokio::sync::Semaphore;
use tokio::time::timeout;

use crate::mailbox::{self, Inbox};
use crate::protocol::{self, JobLimits, JobRequest, JobResponse};
use crate::trust::MeshTrust;

/// A TLS handshake is ~100 µs of *unauthenticated* work and iroh applies no
/// connection cap of its own, so this is the only thing standing between a
/// spoofed source and the CPU.
const MAX_INFLIGHT_HANDSHAKES: usize = 64;
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const PROTOCOL_IO_TIMEOUT: Duration = Duration::from_secs(15);
const FINAL_CLOSE_TIMEOUT: Duration = Duration::from_secs(1);
const TRUST_RECHECK_INTERVAL: Duration = Duration::from_millis(250);
const MAX_CONNECTIONS_PER_PEER: usize = 128;
/// Once this many permits are in use, make new sources prove reachability
/// before we spend a handshake on them.
const RETRY_THRESHOLD: usize = 32;

/// Close code for a peer that is not on the allowlist.
pub const REFUSED_UNTRUSTED: u32 = 4001;
/// Close code for a payload that exceeds [`JobLimits::max_payload_bytes`].
pub const REFUSED_TOO_LARGE: u32 = 4002;
/// Close code for a peer speaking the wrong [`protocol::PROTO`].
pub const REFUSED_PROTO: u32 = 4003;
/// Close code for mail arriving at a node that serves jobs only.
pub const REFUSED_NO_MAILBOX: u32 = 4004;

/// A job that passed the trust gate and the payload check.
pub struct ReceivedJob {
    pub peer: EndpointId,
    pub request: JobRequest,
    /// Decided by *this* node, never by the sender.
    pub limits: JobLimits,
    /// Bytes that followed a [`JobRequest::Run`] claim. Empty for Probe /
    /// Cancel / QueueStats.
    pub payload: Vec<u8>,
}

/// What actually runs received work. Kept behind a trait so the accept loop can
/// be tested without a sandbox, and so the sandbox can change without touching
/// the transport.
pub trait JobExecutor: Send + Sync + 'static {
    fn execute<'a>(
        &'a self,
        job: ReceivedJob,
    ) -> Pin<Box<dyn Future<Output = Result<JobResponse>> + Send + 'a>>;
}

/// On Windows, report whether inbound UDP to this process is likely to be
/// dropped by the firewall.
///
/// Windows' default inbound action is *block*, and a per-application allow rule
/// is not created automatically for a console binary. The failure this produces
/// is silent and badly misleading: outbound dials still work, so the node looks
/// healthy, and any VPN on the box (Tailscale delivers over `utun`, after its
/// own daemon has accepted the packet) transparently carries the traffic that
/// the LAN cannot. Measured 2026-09-05: LAN dial timed out at 30 s while the
/// tailnet dial to the same peer connected in 73 ms, and one inbound rule took
/// the LAN path to 25.7 ms.
///
/// Returns `None` where the question does not apply, and `Some(advice)` where
/// the operator should act. Deliberately advisory: creating a firewall rule
/// needs elevation, and silently elevating during `mesh join` would be worse
/// than telling the truth.
#[cfg(windows)]
pub fn inbound_firewall_advice(program: &std::path::Path) -> Option<String> {
    Some(format!(
        "Windows blocks inbound UDP by default, so peers on the LAN cannot reach \
         this node until an allow rule exists. Without it the mesh appears to \
         work -- outbound dials succeed, and a VPN will silently carry traffic \
         the LAN cannot. In an elevated PowerShell:\n\n    \
         New-NetFirewallRule -DisplayName vox-mesh-inbound -Direction Inbound \
         -Action Allow -Program '{}' -Protocol UDP -RemoteAddress LocalSubnet \
         -Profile Any\n",
        program.display()
    ))
}

/// Non-Windows hosts do not gate inbound traffic per application by default.
#[cfg(not(windows))]
pub fn inbound_firewall_advice(_program: &std::path::Path) -> Option<String> {
    let no_per_app_inbound_udp_filter: Option<String> = None;
    no_per_app_inbound_udp_filter
}

/// Bind a mesh endpoint.
///
/// `presets::Minimal` is not a default to be revisited — it is the whole
/// isolation guarantee. See the crate docs and ADR-047.
pub async fn bind(sk: SecretKey) -> Result<Endpoint> {
    let ep = Endpoint::builder(presets::Minimal)
        .secret_key(sk)
        // `Endpoint::bind(preset)` takes NO alpns; a server built that way
        // refuses every connection at ALPN negotiation, silently.
        //
        // Two ALPNs, one endpoint: jobs and mail are different protocols
        // (plan Task 3.1) but a second Endpoint on the same secret key would
        // mean one EndpointId reachable at two addresses, which every peer's
        // stored `addrs` would then be half-right about.
        .alpns(vec![protocol::ALPN.to_vec(), mailbox::ALPN.to_vec()])
        // Defence in depth. Under Minimal the relay map is empty, so the
        // default HTTPS latency probes and captive-portal check have no target
        // — but that is a property of another struct's defaults, not of ours.
        .net_report_config(NetReportConfig::minimal())
        .address_lookup(iroh_mdns_address_lookup::MdnsAddressLookup::builder())
        .bind()
        .await?;
    Ok(ep)
}

/// Accept loop. Bounded, trust-gated, and free of 0-RTT.
///
/// `mailbox` is the durable A2A inbox (plan Task 3.1). `None` serves jobs only
/// and refuses mail at the ALPN, which is the honest answer for a node that has
/// nowhere to put it — accepting mail into a discarded buffer would let a peer
/// delete its only copy.
pub async fn serve(
    ep: Endpoint,
    trust: Arc<MeshTrust>,
    exec: Arc<dyn JobExecutor>,
    mailbox: Option<Arc<Inbox>>,
) {
    let gate = Arc::new(Semaphore::new(MAX_INFLIGHT_HANDSHAKES));
    while let Some(incoming) = ep.accept().await {
        if gate.available_permits() < MAX_INFLIGHT_HANDSHAKES - RETRY_THRESHOLD
            && !incoming.remote_addr_validated()
        {
            let _ = incoming.retry();
            continue;
        }
        let Ok(permit) = Arc::clone(&gate).try_acquire_owned() else {
            // `ignore()` sends nothing at all; `refuse()` would answer, which
            // is both work for us and a signal for a scanner.
            incoming.ignore();
            continue;
        };
        let (trust, exec, mailbox) = (Arc::clone(&trust), Arc::clone(&exec), mailbox.clone());
        tokio::spawn(async move {
            // Awaiting `incoming` completes the handshake, so `remote_id()`
            // below is the peer's *proven* public key. Never call
            // `Accepting::into_0rtt()`: there `remote_id()` is fallible and
            // every check that follows becomes advisory.
            let Ok(Ok(conn)) = timeout(HANDSHAKE_TIMEOUT, incoming).await else {
                return;
            };
            drop(permit);
            let remote = conn.remote_id();
            if !trust.is_trusted(&remote) {
                // No protocol-level explanation to a stranger — it would be an
                // oracle for "is this endpoint id known to you".
                conn.close(REFUSED_UNTRUSTED.into(), b"not trusted");
                return;
            }
            let Some(_registration) =
                trust.try_register(remote, conn.clone(), MAX_CONNECTIONS_PER_PEER)
            else {
                conn.close(REFUSED_UNTRUSTED.into(), b"peer connection limit");
                return;
            };
            // Dispatch on the negotiated ALPN. Mail is not a JobRequest
            // variant: it is handed over and forgotten, where a job is a
            // question whose answer the caller waits for.
            let handler_conn = conn.clone();
            let handler_trust = Arc::clone(&trust);
            let handler = async move {
                if handler_conn.alpn() == mailbox::ALPN {
                    match mailbox {
                        Some(inbox) => {
                            mailbox::handle(handler_conn, remote, Arc::clone(&handler_trust), inbox)
                                .await
                        }
                        None => {
                            handler_conn.close(REFUSED_NO_MAILBOX.into(), b"no mailbox configured");
                            Ok(())
                        }
                    }
                } else {
                    handle(handler_conn, remote, exec).await
                }
            };
            let outcome = tokio::select! {
                outcome = handler => outcome,
                () = wait_until_untrusted(&trust, remote) => {
                    conn.close(crate::trust::REVOKED.into(), b"trust revoked");
                    Err(anyhow::anyhow!("trust revoked"))
                }
            };
            if let Err(e) = outcome {
                tracing::debug!(peer = %remote, error = %e, "mesh stream ended");
            }
        });
    }
}

async fn wait_until_untrusted(trust: &MeshTrust, peer: EndpointId) {
    loop {
        tokio::time::sleep(TRUST_RECHECK_INTERVAL).await;
        if !trust.is_trusted(&peer) {
            return;
        }
    }
}

async fn write_response(
    send: &mut iroh::endpoint::SendStream,
    response: &JobResponse,
) -> Result<()> {
    timeout(PROTOCOL_IO_TIMEOUT, protocol::write_frame(send, response)).await??;
    send.finish()?;
    Ok(())
}

/// Serve one connection: greet, check the payload claim, execute, reply.
async fn handle(conn: Connection, peer: EndpointId, exec: Arc<dyn JobExecutor>) -> Result<()> {
    let (mut send, mut recv) = timeout(PROTOCOL_IO_TIMEOUT, conn.accept_bi()).await??;

    let hello: protocol::Hello =
        timeout(PROTOCOL_IO_TIMEOUT, protocol::read_frame(&mut recv, 4096)).await??;
    if let Err(e) = protocol::check_hello(&hello) {
        write_response(&mut send, &JobResponse::Failed(e.to_string())).await?;
        // `Connection::close` may drop stream data not yet delivered to the
        // peer's application. Wait briefly for the FIN (or give up) first.
        let _ = timeout(Duration::from_millis(200), send.stopped()).await;
        conn.close(REFUSED_PROTO.into(), b"proto mismatch");
        let _ = timeout(FINAL_CLOSE_TIMEOUT, conn.closed()).await;
        return Ok(());
    }

    let request: JobRequest = timeout(
        PROTOCOL_IO_TIMEOUT,
        protocol::read_frame(&mut recv, 64 * 1024),
    )
    .await??;
    let limits = JobLimits::default();

    // Checked BEFORE the transfer, so an oversized job costs us a frame rather
    // than a gigabyte of disk. The per-kind cap is tighter than the global one
    // for VoxScript (source is text).
    let payload = if let JobRequest::Run {
        payload_bytes,
        kind,
        ..
    } = &request
    {
        let cap = limits.max_payload_for(kind.clone());
        if *payload_bytes > cap {
            let msg = format!("payload of {payload_bytes} bytes exceeds the {cap} byte cap");
            write_response(&mut send, &JobResponse::Failed(msg)).await?;
            let _ = timeout(FINAL_CLOSE_TIMEOUT, conn.closed()).await;
            return Ok(());
        }
        let max = usize::try_from(payload_bytes.saturating_add(8)).unwrap_or(usize::MAX);
        let p: Vec<u8> =
            timeout(PROTOCOL_IO_TIMEOUT, protocol::read_frame(&mut recv, max)).await??;
        if p.len() as u64 != *payload_bytes {
            let msg = format!(
                "payload length {} does not match the declared claim of {payload_bytes} bytes",
                p.len()
            );
            write_response(&mut send, &JobResponse::Failed(msg)).await?;
            let _ = timeout(FINAL_CLOSE_TIMEOUT, conn.closed()).await;
            return Ok(());
        }
        p
    } else {
        Vec::new()
    };

    let response = exec
        .execute(ReceivedJob {
            peer,
            request,
            limits,
            payload,
        })
        .await
        .unwrap_or_else(|e| JobResponse::Failed(e.to_string()));

    write_response(&mut send, &response).await?;
    // `finish()` signals end-of-stream; it does NOT flush. Dropping the
    // Connection here would close it before the bytes reach the wire and the
    // peer would see `closed by peer: 0` with no payload. Measured during the
    // Task 0.2 spike; see ADR-047.
    let _ = timeout(FINAL_CLOSE_TIMEOUT, conn.closed()).await;
    Ok(())
}

#[cfg(test)]
mod firewall_advice_tests {
    use super::*;

    #[test]
    fn advice_is_windows_only() {
        let got = inbound_firewall_advice(std::path::Path::new("/tmp/vox"));
        if cfg!(windows) {
            let a = got.expect("windows must produce advice");
            assert!(
                a.contains("New-NetFirewallRule"),
                "must give the exact command: {a}"
            );
            assert!(
                a.contains("Inbound"),
                "the rule must be an INBOUND one: {a}"
            );
        } else {
            assert!(got.is_none(), "only Windows gates inbound per application");
        }
    }

    #[test]
    #[cfg(windows)]
    fn advice_names_the_silent_failure_mode() {
        // The danger is not that it breaks loudly -- it is that a VPN hides it.
        let a = inbound_firewall_advice(std::path::Path::new("C:/vox.exe")).unwrap();
        assert!(
            a.contains("VPN"),
            "must warn that a VPN masks the fault: {a}"
        );
    }
}
