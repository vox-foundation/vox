//! `vox dashboard` — launch local orchestration SPA in browser.
use anyhow::Result;

#[derive(clap::Args, Clone, Debug)]
pub struct DashboardArgs {
    /// Port to serve on (default: VOX_DASHBOARD_PORT or 3921).
    #[arg(long)]
    pub port: Option<u16>,
    /// Open browser automatically (default: true).
    #[arg(long, default_value_t = true)]
    pub open: bool,
    /// Run in --app mode (chromium app window, no browser chrome).
    #[arg(long)]
    pub app_mode: bool,
}

pub async fn run(args: DashboardArgs) -> Result<()> {
    let port = args.port
        .or_else(|| std::env::var("VOX_DASHBOARD_PORT").ok().and_then(|s| s.parse().ok()))
        .unwrap_or(3921);
    // Ensure the orchestrator daemon is running in the background with HTTP + Dashboard enabled
    let daemon_path = crate::process_supervision::resolve_managed_binary_path("vox-orchestrator-d");
    
    println!("Starting Vox Orchestrator on port {} with Dashboard enabled...", port);
    
    // We spawn it detached so it survives the CLI exit
    #[cfg(target_os = "windows")]
    let mut child = std::process::Command::new(&daemon_path);
    #[cfg(not(target_os = "windows"))]
    let mut child = std::process::Command::new(&daemon_path);
    
    child.env("VOX_MCP_HTTP_ENABLED", "1")
         .env("VOX_DASHBOARD_ENABLED", "1")
         .env("VOX_MCP_HTTP_PORT", port.to_string())
         .env("VOX_ORCHESTRATOR_DAEMON_SOCKET", format!("127.0.0.1:{}", port + 1));
    
    // Try to spawn it
    match child.spawn() {
        Ok(_) => {
            println!("[VOX_DASHBOARD_READY: http://127.0.0.1:{}/dashboard]", port);
            if args.open {
                let url = format!("http://127.0.0.1:{}/dashboard", port);
                #[cfg(target_os = "windows")]
                let _ = std::process::Command::new("cmd").args(&["/C", "start", &url]).spawn();
                #[cfg(target_os = "macos")]
                let _ = std::process::Command::new("open").arg(&url).spawn();
                #[cfg(target_os = "linux")]
                let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
            }
            // Keep the CLI process alive so the user sees logs if they want, or they can ctrl-c
            tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
        }
        Err(e) => {
            anyhow::bail!("Failed to start orchestrator daemon: {}", e);
        }
    }

    Ok(())
}
