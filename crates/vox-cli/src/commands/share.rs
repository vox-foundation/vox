//! `vox share` — public-URL tunnel for Vox apps.
//!
//! S1: LAN backend only. S2 adds Cloudflare default. S3 adds localhost.run fallback.

use anyhow::Result;
use clap::Args;
use std::time::Duration;
use vox_share::auth::AuthMode;
use vox_share::{BackendKind, ShareConfig, ShareSession};

#[derive(Args, Debug)]
#[command(about = "Share a Vox app via a public URL tunnel")]
pub struct ShareArgs {
    /// Tunnel backend (lan, cloudflare, localhost-run, tailscale). Default: cloudflare.
    #[arg(long, default_value = "cloudflare")]
    pub backend: String,

    /// Port the bundled app listens on.
    #[arg(long, default_value = "7860")]
    pub port: u16,

    /// Auto-shutdown duration (e.g. 8h, 30m, 5s, none). Default: 8h.
    #[arg(long, default_value = "8h")]
    pub duration: String,

    /// Use dev server pipeline instead of vox bundle (faster iteration).
    #[arg(long)]
    pub dev: bool,

    /// Accept Cloudflare ToS without prompting (required in CI / non-interactive environments).
    #[arg(long)]
    pub accept_tos: bool,

    /// Authentication mode: none, basic:user:pass. Default: url-token (auto-generated).
    #[arg(long, default_value = "token")]
    pub auth: String,

    /// Keep Cloudflare backend even if SSE routes detected (SSE will be buffered/broken).
    #[arg(long)]
    pub allow_buffered_streaming: bool,
}

pub async fn run(args: ShareArgs) -> Result<()> {
    let backend: BackendKind = args
        .backend
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid --backend `{}`: {}", args.backend, e))?;

    let auth_mode = if args.auth == "token" {
        AuthMode::random_token()
    } else {
        args.auth
            .parse::<AuthMode>()
            .map_err(|e| anyhow::anyhow!("invalid --auth: {}", e))?
    };

    let duration = parse_duration(&args.duration)?;

    println!("[vox share] Backend: {}", backend);

    if matches!(backend, BackendKind::Cloudflare) {
        vox_share::consent::ensure_consent(args.accept_tos, false)
            .map_err(|e| anyhow::anyhow!("{}", e))?;
    }

    println!("[vox share] Proxy port is OS-assigned.");
    println!(
        "[vox share] Note: bundle integration ships in S8. Run your app on port {} separately.",
        args.port
    );

    // Allow fallback to localhost.run when using Cloudflare (the default).
    // If the user explicitly passed --backend cloudflare or any other backend,
    // we still allow fallback only for Cloudflare (it's the fallback-eligible backend).
    let allow_fallback = matches!(backend, BackendKind::Cloudflare);

    let cfg = ShareConfig {
        backend,
        upstream_port: args.port,
        proxy_port: 0,
        duration,
        app_binary: None,
        connect_timeout: Duration::from_secs(10),
        allow_fallback,
        auth_mode,
        allow_buffered_streaming: args.allow_buffered_streaming,
    };

    let session = ShareSession::start(cfg)
        .await
        .map_err(|e| anyhow::anyhow!("share session: {}", e))?;

    println!("[vox share] Public URL: {}", session.public_url);
    println!(
        "[vox share] Local proxy: http://127.0.0.1:{}",
        session.proxy_port
    );
    println!("[vox share] Press Ctrl+C to stop.");

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            println!("[vox share] Shutting down.");
        }
        _ = async {
            if let Some(d) = duration {
                tokio::time::sleep(d).await;
            } else {
                std::future::pending::<()>().await;
            }
        } => {
            println!("[vox share] Duration elapsed.");
        }
    }

    session.shutdown().await;
    println!("[vox share] Done.");
    Ok(())
}

fn parse_duration(s: &str) -> Result<Option<Duration>> {
    if s == "none" {
        return Ok(None);
    }
    if s.len() < 2 {
        anyhow::bail!("bad duration `{}` — use e.g. 8h, 30m, 5s, or none", s);
    }
    let (num, unit) = s.split_at(s.len() - 1);
    let n: u64 = num
        .parse()
        .map_err(|_| anyhow::anyhow!("bad duration `{}` — use e.g. 8h, 30m, 5s", s))?;
    Ok(Some(match unit {
        "s" => Duration::from_secs(n),
        "m" => Duration::from_secs(n * 60),
        "h" => Duration::from_secs(n * 3600),
        _ => anyhow::bail!("duration unit must be s/m/h or `none`, got `{}`", unit),
    }))
}
