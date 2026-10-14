//! TLS termination for the MCP HTTP gateway (capability-profiles plan 04).
//!
//! The provider is selected explicitly: unified workspace builds compile rustls with both
//! `ring` and `aws_lc_rs`, and `ServerConfig::builder()` panics when it cannot pick one.
//! `ring` is the workspace's pinned provider (contracts/crypto/transport-providers.v1.json).

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tokio_rustls::server::TlsStream;

use super::bind_policy::TlsMaterial;

/// A client that stalls mid-handshake is dropped after this long.
const TLS_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
/// Completed handshakes waiting for axum to pick them up.
const PENDING_CONNECTIONS: usize = 64;

/// Resolve cert/key paths into [`TlsMaterial`]. Checks readability only; PEM parsing happens in
/// [`load_tls_acceptor`] at startup.
pub fn tls_material_from_paths(cert: Option<PathBuf>, key: Option<PathBuf>) -> TlsMaterial {
    match (cert, key) {
        (None, None) => TlsMaterial::Absent,
        (Some(_), None) => TlsMaterial::Invalid(
            "VOX_MCP_HTTP_TLS_CERT is set but VOX_MCP_HTTP_TLS_KEY is not".to_string(),
        ),
        (None, Some(_)) => TlsMaterial::Invalid(
            "VOX_MCP_HTTP_TLS_KEY is set but VOX_MCP_HTTP_TLS_CERT is not".to_string(),
        ),
        (Some(cert), Some(key)) => {
            for (label, path) in [("certificate", &cert), ("private key", &key)] {
                if let Err(e) = std::fs::File::open(path) {
                    return TlsMaterial::Invalid(format!(
                        "TLS {label} {} is not readable: {e}",
                        path.display()
                    ));
                }
            }
            TlsMaterial::Ready { cert, key }
        }
    }
}

pub(super) fn load_tls_acceptor(cert: &Path, key: &Path) -> anyhow::Result<TlsAcceptor> {
    let certs = CertificateDer::pem_file_iter(cert)
        .with_context(|| format!("read TLS certificate {}", cert.display()))?
        .collect::<Result<Vec<_>, _>>()
        .with_context(|| format!("parse TLS certificate {}", cert.display()))?;
    anyhow::ensure!(
        !certs.is_empty(),
        "TLS certificate {} contains no PEM certificate",
        cert.display()
    );
    let private_key = PrivateKeyDer::from_pem_file(key)
        .with_context(|| format!("read TLS private key {}", key.display()))?;
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .context("rustls protocol versions")?
    .with_no_client_auth()
    .with_single_cert(certs, private_key)
    .with_context(|| {
        format!(
            "rustls rejected certificate {} / key {}",
            cert.display(),
            key.display()
        )
    })?;
    Ok(TlsAcceptor::from(Arc::new(config)))
}

/// `axum::serve::Listener` that yields only connections whose TLS handshake completed.
/// Handshakes run in their own tasks so one slow client cannot stall `accept`.
pub(super) struct TlsListener {
    rx: tokio::sync::mpsc::Receiver<(TlsStream<TcpStream>, SocketAddr)>,
    local_addr: SocketAddr,
}

impl TlsListener {
    pub(super) fn spawn(tcp: TcpListener, acceptor: TlsAcceptor) -> std::io::Result<Self> {
        let local_addr = tcp.local_addr()?;
        let (tx, rx) = tokio::sync::mpsc::channel(PENDING_CONNECTIONS);
        tokio::spawn(async move {
            loop {
                let (sock, peer) = match tcp.accept().await {
                    Ok(v) => v,
                    Err(e) => {
                        tracing::warn!(error = %e, "mcp-http tcp accept failed");
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        continue;
                    }
                };
                if tx.is_closed() {
                    break;
                }
                let acceptor = acceptor.clone();
                let tx = tx.clone();
                tokio::spawn(async move {
                    match tokio::time::timeout(TLS_HANDSHAKE_TIMEOUT, acceptor.accept(sock)).await {
                        Ok(Ok(stream)) => {
                            let _ = tx.send((stream, peer)).await;
                        }
                        Ok(Err(e)) => {
                            tracing::debug!(%peer, error = %e, "mcp-http tls handshake failed")
                        }
                        Err(_) => tracing::debug!(%peer, "mcp-http tls handshake timed out"),
                    }
                });
            }
        });
        Ok(Self { rx, local_addr })
    }
}

impl axum::serve::Listener for TlsListener {
    type Io = TlsStream<TcpStream>;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        match self.rx.recv().await {
            Some(conn) => conn,
            None => std::future::pending().await,
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        Ok(self.local_addr)
    }
}

/// Write an rcgen self-signed `localhost` certificate + key into `dir`.
#[cfg(test)]
pub(super) fn write_self_signed(dir: &Path) -> (PathBuf, PathBuf) {
    let ck = rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    let cert = dir.join("cert.pem");
    let key = dir.join("key.pem");
    std::fs::write(&cert, ck.cert.pem()).unwrap();
    std::fs::write(&key, ck.signing_key.serialize_pem()).unwrap();
    (cert, key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_material_absent_when_unset() {
        assert_eq!(tls_material_from_paths(None, None), TlsMaterial::Absent);
    }

    #[test]
    fn tls_material_rejects_half_configuration() {
        let only_cert = tls_material_from_paths(Some("c.pem".into()), None);
        assert!(
            matches!(only_cert, TlsMaterial::Invalid(ref m) if m.contains("VOX_MCP_HTTP_TLS_KEY"))
        );
        let only_key = tls_material_from_paths(None, Some("k.pem".into()));
        assert!(
            matches!(only_key, TlsMaterial::Invalid(ref m) if m.contains("VOX_MCP_HTTP_TLS_CERT"))
        );
    }

    #[test]
    fn tls_material_rejects_unreadable_paths() {
        let dir = tempfile::tempdir().unwrap();
        let m = tls_material_from_paths(
            Some(dir.path().join("missing-cert.pem")),
            Some(dir.path().join("missing-key.pem")),
        );
        assert!(matches!(m, TlsMaterial::Invalid(ref msg) if msg.contains("missing-cert.pem")));
    }

    #[test]
    fn tls_material_ready_for_readable_files() {
        let dir = tempfile::tempdir().unwrap();
        let (cert, key) = write_self_signed(dir.path());
        assert_eq!(
            tls_material_from_paths(Some(cert.clone()), Some(key.clone())),
            TlsMaterial::Ready { cert, key }
        );
    }

    #[test]
    fn load_tls_acceptor_accepts_self_signed_pair() {
        let dir = tempfile::tempdir().unwrap();
        let (cert, key) = write_self_signed(dir.path());
        load_tls_acceptor(&cert, &key).expect("acceptor from rcgen pair");
    }

    #[test]
    fn load_tls_acceptor_rejects_non_pem() {
        let dir = tempfile::tempdir().unwrap();
        let cert = dir.path().join("cert.pem");
        let key = dir.path().join("key.pem");
        std::fs::write(&cert, b"not a certificate").unwrap();
        std::fs::write(&key, b"not a key").unwrap();
        let Err(e) = load_tls_acceptor(&cert, &key) else {
            panic!("non-PEM input must be rejected");
        };
        let err = format!("{e:#}");
        assert!(err.contains("cert.pem"), "{err}");
    }
}
