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
    
    // We own it in the foreground
    #[cfg(target_os = "windows")]
    let mut child = std::process::Command::new(&daemon_path);
    #[cfg(not(target_os = "windows"))]
    let mut child = std::process::Command::new(&daemon_path);
    
    child.env("VOX_MCP_HTTP_ENABLED", "1")
         .env("VOX_DASHBOARD_ENABLED", "1")
         .env("VOX_MCP_HTTP_PORT", port.to_string());
    
    // Try to spawn it
    match child.spawn() {
        Ok(mut child_process) => {
            let url = format!("http://127.0.0.1:{}/dashboard", port);
            
            // Poll socket for up to 5 seconds
            let mut ready = false;
            for _ in 0..50 {
                if std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
                    ready = true;
                    break;
                }
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }

            if ready {
                println!("[VOX_DASHBOARD_READY: {}]", url);
                if args.open {
                    #[cfg(target_os = "windows")]
                    let _ = std::process::Command::new("cmd").args(&["/C", "start", &url]).spawn();
                    #[cfg(target_os = "macos")]
                    let _ = std::process::Command::new("open").arg(&url).spawn();
                    #[cfg(target_os = "linux")]
                    let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
                }
            } else {
                eprintln!("Warning: Daemon did not bind to port {} within 5 seconds.", port);
            }
            
            // Wait for it in the foreground so Ctrl-C kills it naturally
            let _ = child_process.wait();
        }
        Err(e) => {
            anyhow::bail!("Failed to start orchestrator daemon: {}", e);
        }
    }

    Ok(())
}
