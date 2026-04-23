//! `vox dashboard` — launch local orchestration SPA in browser.
use anyhow::{Context, Result};
use std::path::PathBuf;
use tokio::time::{sleep, Duration};

#[derive(clap::Args, Clone, Debug)]
pub struct DashboardArgs {
    #[clap(subcommand)]
    pub command: Option<DashboardCommand>,

    /// Port to serve on (default: VOX_DASHBOARD_PORT or 3921).
    #[arg(long)]
    pub port: Option<u16>,
    /// Open browser automatically (default: true).
    #[arg(long, default_value_t = true)]
    pub open: bool,
    /// Run in --app mode (chromium app window, no browser chrome).
    #[arg(long)]
    pub app_mode: bool,
    /// Run in the foreground instead of detaching
    #[arg(long)]
    pub foreground: bool,
}

#[derive(clap::Subcommand, Clone, Debug)]
pub enum DashboardCommand {
    /// Stop the dashboard daemon
    Stop,
}

struct DashboardLauncher {
    port: u16,
    open: bool,
    app_mode: bool,
    daemon_path: PathBuf,
}

impl DashboardLauncher {
    async fn launch(&self, foreground: bool) -> Result<()> {
        let state_dir = vox_config::state_dir().unwrap_or_else(|| std::env::temp_dir().join("vox"));
        std::fs::create_dir_all(&state_dir).ok();
        let log_path = state_dir.join("dashboard.log");
        let pid_path = state_dir.join("dashboard.pid");
        
        let out_file = std::fs::File::create(&log_path).context("create dashboard.log")?;
        let err_file = out_file.try_clone().context("clone dashboard.log")?;
        
        println!("Starting Vox Orchestrator on port {} with Dashboard enabled...", self.port);
        
        let mut child_cmd = std::process::Command::new(&self.daemon_path);
        child_cmd.env("VOX_MCP_HTTP_ENABLED", "1")
                 .env("VOX_DASHBOARD_ENABLED", "1")
                 .env("VOX_MCP_HTTP_PORT", self.port.to_string());
                 
        if !foreground {
            child_cmd.stdout(std::process::Stdio::from(out_file))
                     .stderr(std::process::Stdio::from(err_file));

            #[cfg(not(target_os = "windows"))]
            {
                use std::os::unix::process::CommandExt;
                unsafe {
                    child_cmd.pre_exec(|| {
                        libc::setsid();
                        Ok(())
                    });
                }
            }
            
            #[cfg(target_os = "windows")]
            {
                use std::os::windows::process::CommandExt;
                child_cmd.creation_flags(0x00000200 | 0x00000008);
            }
        }
        
        let mut child_process = child_cmd.spawn().context("spawn orchestrator daemon")?;
        
        if !foreground {
            std::fs::write(&pid_path, child_process.id().to_string()).context("write dashboard.pid")?;
        }
        
        let url = format!("http://127.0.0.1:{}/dashboard", self.port);
        let health_url = format!("http://127.0.0.1:{}/health", self.port);
        
        let client = reqwest::Client::new();
        let mut ready = false;
        
        for _ in 0..40 {
            if let Ok(resp) = client.get(&health_url).send().await {
                if resp.status().is_success() {
                    ready = true;
                    break;
                }
            }
            sleep(Duration::from_millis(250)).await;
        }
        
        if ready {
            println!("[VOX_DASHBOARD_READY: {}]", url);
            if self.open {
                #[cfg(target_os = "windows")]
                {
                    if self.app_mode {
                        let _ = std::process::Command::new("cmd").args(&["/C", "start", "chrome", &format!("--app={}", url)]).spawn();
                    } else {
                        let _ = std::process::Command::new("cmd").args(&["/C", "start", &url]).spawn();
                    }
                }
                #[cfg(target_os = "macos")]
                {
                    if self.app_mode {
                        let _ = std::process::Command::new("open").arg("-n").arg("-a").arg("Google Chrome").arg(&format!("--args --app={}", url)).spawn();
                    } else {
                        let _ = std::process::Command::new("open").arg(&url).spawn();
                    }
                }
                #[cfg(target_os = "linux")]
                {
                    if self.app_mode {
                        let _ = std::process::Command::new("google-chrome").arg(&format!("--app={}", url)).spawn();
                    } else {
                        let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
                    }
                }
            }
        } else {
            eprintln!("Warning: Daemon did not bind to port {} within 10 seconds.", self.port);
            if !foreground {
                if let Ok(logs) = std::fs::read_to_string(&log_path) {
                    let lines: Vec<&str> = logs.lines().collect();
                    for line in lines.iter().rev().take(50).rev() {
                        eprintln!("{}", line);
                    }
                }
            }
            anyhow::bail!("Failed to start orchestrator daemon within timeout");
        }
        
        if foreground {
            let _ = child_process.wait();
        }
        
        Ok(())
    }
}

pub async fn run(args: DashboardArgs) -> Result<()> {
    if let Some(DashboardCommand::Stop) = args.command {
        return stop_dashboard().await;
    }

    let port = args.port
        .or_else(|| std::env::var("VOX_DASHBOARD_PORT").ok().and_then(|s| s.parse().ok()))
        .unwrap_or(3921);
        
    let daemon_path = crate::process_supervision::resolve_managed_binary_path("vox-orchestrator-d");
    
    let launcher = DashboardLauncher {
        port,
        open: args.open,
        app_mode: args.app_mode,
        daemon_path,
    };
    
    launcher.launch(args.foreground).await
}

async fn stop_dashboard() -> Result<()> {
    let state_dir = vox_config::state_dir().unwrap_or_else(|| std::env::temp_dir().join("vox"));
    let pid_path = state_dir.join("dashboard.pid");
    
    if !pid_path.exists() {
        println!("No dashboard daemon is running (dashboard.pid not found).");
        return Ok(());
    }
    
    let pid_str = std::fs::read_to_string(&pid_path)?;
    let pid: u32 = pid_str.trim().parse().context("Invalid PID in dashboard.pid")?;
    
    println!("Stopping dashboard daemon (PID {})...", pid);
    
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("taskkill").args(&["/PID", &pid.to_string(), "/F"]).output();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = std::process::Command::new("kill").arg("-TERM").arg(pid.to_string()).output();
        sleep(Duration::from_secs(5)).await;
        // Check if still running, kill -9 if so
        let still_running = std::process::Command::new("kill").arg("-0").arg(pid.to_string()).output().map(|o| o.status.success()).unwrap_or(false);
        if still_running {
            let _ = std::process::Command::new("kill").arg("-KILL").arg(pid.to_string()).output();
        }
    }
    
    let _ = std::fs::remove_file(&pid_path);
    println!("Dashboard daemon stopped.");
    
    Ok(())
}
