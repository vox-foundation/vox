//! Coordinator: wires app + proxy + tunnel-backend together.

use crate::auth::AuthMode;
use crate::backend::{BackendKind, TunnelBackend, TunnelHandle};
use crate::backends::cloudflare::CloudflareBackend;
use crate::backends::lan::LanBackend;
use crate::backends::localhost_run::LocalhostRunBackend;
use crate::backends::tailscale::TailscaleBackend;
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
    /// When true and backend is Cloudflare, fall back to localhost.run on failure.
    pub allow_fallback: bool,
    /// Authentication mode for the share session.
    pub auth_mode: AuthMode,
    /// If true, don't auto-switch away from Cloudflare even if SSE routes detected.
    pub allow_buffered_streaming: bool,
}

/// An active share session. Drop or call `shutdown` to clean up.
pub struct ShareSession {
    pub tunnel_handle: TunnelHandle,
    pub proxy_port: u16,
    /// The public URL to share with users — may include auth token decoration.
    pub public_url: String,
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
            auth_mode: cfg.auth_mode.clone(),
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

        // S6: SSE detection — if Cloudflare and SSE routes found, switch to localhost.run.
        if matches!(cfg.backend, BackendKind::Cloudflare) && !cfg.allow_buffered_streaming {
            if crate::sse_detect::has_sse_routes(cfg.upstream_port).await {
                println!("[vox share] App uses streaming (SSE); auto-selected --backend localhost-run for SSE compatibility");
                println!("[vox share] Use --allow-buffered-streaming to keep Cloudflare (SSE will be buffered)");
                let fallback = make_backend(BackendKind::LocalhostRun);
                fallback.preflight().await?;
                let tunnel_handle = fallback.start(actual_proxy_port, cfg.connect_timeout).await?;
                let public_url = cfg.auth_mode.decorate_url(&tunnel_handle.public_url);
                let duration_timer = cfg.duration.map(|d| {
                    tokio::spawn(async move {
                        tokio::time::sleep(d).await;
                    })
                });
                return Ok(ShareSession {
                    tunnel_handle,
                    proxy_port: actual_proxy_port,
                    public_url,
                    proxy_shutdown: proxy_shutdown_tx,
                    duration_timer,
                    _app_child: app_child,
                });
            }
        }

        // Bring up the tunnel backend.
        let backend: Box<dyn TunnelBackend> = make_backend(cfg.backend);
        backend.preflight().await?;
        let tunnel_handle = match backend.start(actual_proxy_port, cfg.connect_timeout).await {
            Ok(h) => h,
            Err(e)
                if cfg.allow_fallback && matches!(cfg.backend, BackendKind::Cloudflare) =>
            {
                println!(
                    "[vox share] Cloudflare unavailable ({}); falling back to localhost.run",
                    e
                );
                let fallback = make_backend(BackendKind::LocalhostRun);
                fallback.preflight().await?;
                fallback.start(actual_proxy_port, cfg.connect_timeout).await?
            }
            Err(e) => return Err(e),
        };

        // Optional auto-shutdown timer.
        let duration_timer = cfg.duration.map(|d| {
            tokio::spawn(async move {
                tokio::time::sleep(d).await;
            })
        });

        let public_url = cfg.auth_mode.decorate_url(&tunnel_handle.public_url);
        Ok(ShareSession {
            tunnel_handle,
            proxy_port: actual_proxy_port,
            public_url,
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
        BackendKind::LocalhostRun => Box::new(LocalhostRunBackend::new()),
        BackendKind::Tailscale => Box::new(TailscaleBackend::new()),
    }
}
