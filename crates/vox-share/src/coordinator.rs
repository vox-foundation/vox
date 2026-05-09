//! Coordinator: wires app + proxy + tunnel-backend together.

use crate::backend::{BackendKind, TunnelBackend, TunnelHandle};
use crate::backends::cloudflare::CloudflareBackend;
use crate::backends::lan::LanBackend;
use crate::error::{ShareError, ShareResult};
use crate::proxy::{build_app as build_proxy_app, ProxyConfig};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;
use tokio::net::TcpListener;

/// Configuration for a share session.
#[derive(Debug, Clone)]
pub struct ShareConfig {
    pub backend: BackendKind,
    /// The localhost port the bundled Vox app is listening on.
    pub upstream_port: u16,
    /// Where to bind the proxy (0 = OS-pick).
    pub proxy_port: u16,
    /// Auto-shutdown after this duration. None = unbounded.
    pub duration: Option<Duration>,
    /// Path to the bundled app binary. None = assume already running.
    pub app_binary: Option<PathBuf>,
    /// Time to wait for the tunnel to come up.
    pub connect_timeout: Duration,
}

/// An active share session. Drop or call `shutdown` to clean up.
pub struct ShareSession {
    pub tunnel_handle: TunnelHandle,
    pub proxy_port: u16,
    proxy_shutdown: tokio::sync::oneshot::Sender<()>,
    duration_timer: Option<tokio::task::JoinHandle<()>>,
    _app_child: Option<tokio::process::Child>,
}

impl ShareSession {
    pub async fn start(cfg: ShareConfig) -> ShareResult<Self> {
        // Optionally spawn the app binary.
        let app_child = if let Some(path) = &cfg.app_binary {
            let child = tokio::process::Command::new(path)
                .env("PORT", cfg.upstream_port.to_string())
                .kill_on_drop(true)
                .spawn()
                .map_err(|e| ShareError::Config(format!("spawn app: {}", e)))?;
            Some(child)
        } else {
            None
        };

        // Bind the proxy listener.
        let bind_addr: SocketAddr = SocketAddr::from(([127, 0, 0, 1], cfg.proxy_port));
        let listener = TcpListener::bind(bind_addr).await?;
        let actual_proxy_port = listener.local_addr()?.port();

        let proxy_cfg = ProxyConfig {
            upstream_addr: SocketAddr::from(([127, 0, 0, 1], cfg.upstream_port)),
            bind_addr,
        };
        let proxy_app = build_proxy_app(proxy_cfg);

        let (proxy_shutdown_tx, proxy_shutdown_rx) = tokio::sync::oneshot::channel::<()>();
        tokio::spawn(async move {
            let server = axum::serve(listener, proxy_app);
            let proxy_shutdown_rx = async move { let _ = proxy_shutdown_rx.await; };
            tokio::pin!(proxy_shutdown_rx);
            tokio::select! {
                res = server => { let _ = res; }
                _ = &mut proxy_shutdown_rx => {}
            }
        });

        // Bring up the tunnel backend.
        let backend: Box<dyn TunnelBackend> = make_backend(cfg.backend);
        backend.preflight().await?;
        let tunnel_handle = backend.start(actual_proxy_port, cfg.connect_timeout).await?;

        // Optional auto-shutdown timer.
        let duration_timer = cfg.duration.map(|d| {
            tokio::spawn(async move {
                tokio::time::sleep(d).await;
            })
        });

        Ok(ShareSession {
            tunnel_handle,
            proxy_port: actual_proxy_port,
            proxy_shutdown: proxy_shutdown_tx,
            duration_timer,
            _app_child: app_child,
        })
    }

    pub async fn shutdown(self) {
        self.tunnel_handle.shutdown();
        if let Some(t) = self.duration_timer {
            t.abort();
        }
        let _ = self.proxy_shutdown.send(());
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

fn make_backend(kind: BackendKind) -> Box<dyn TunnelBackend> {
    match kind {
        BackendKind::Lan => Box::new(LanBackend::new()),
        BackendKind::Cloudflare => Box::new(CloudflareBackend::new()),
        // S3 adds LocalhostRun; S4 adds Tailscale.
        BackendKind::LocalhostRun | BackendKind::Tailscale => {
            unimplemented!("backend {:?} ships in a later phase", kind)
        }
    }
}
