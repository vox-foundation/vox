//! Populi lifecycle helpers shared by `vox populi` subcommands.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use anyhow::{Context, bail};
use clap::{Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const DEFAULT_BIND: &str = "127.0.0.1:9847";

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum PopuliConnectivityMode {
    Lan,
    Overlay,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum OverlayProviderArg {
    Auto,
    Tailscale,
    Wireguard,
    Tunnel,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OverlayProvider {
    Tailscale,
    Wireguard,
    Tunnel,
}

impl OverlayProvider {
    fn as_str(self) -> &'static str {
        match self {
            Self::Tailscale => "tailscale",
            Self::Wireguard => "wireguard",
            Self::Tunnel => "tunnel",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PopuliDaemonState {
    pid: Option<u32>,
    bind: String,
    mode: String,
    control_url: String,
    env_file: String,
    overlay_provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OverlayDiagnostics {
    provider: String,
    available: bool,
    connected: bool,
    detail: String,
}

#[derive(Subcommand)]
pub enum PopuliLifecycleCmd {
    /// Start a private local/overlay mesh with secure defaults.
    Up {
        /// Connectivity strategy.
        #[arg(long, value_enum, default_value_t = PopuliConnectivityMode::Lan)]
        mode: PopuliConnectivityMode,
        /// Mesh scope id (auto-generated when omitted).
        #[arg(long)]
        scope: Option<String>,
        /// GPU advertisement policy (`auto` currently maps to env-driven probe defaults).
        #[arg(long, default_value = "auto")]
        gpus: String,
        /// Control-plane bind address.
        #[arg(long, default_value = DEFAULT_BIND)]
        bind: String,
        /// Overlay provider selection (`auto` probes available providers).
        #[arg(long, value_enum, default_value_t = OverlayProviderArg::Auto)]
        overlay_provider: OverlayProviderArg,
        /// Allow local insecure mode (disables required mesh token).
        #[arg(long, default_value_t = false)]
        insecure_local: bool,
    },
    /// Stop the populi control-plane process started by `vox populi up`.
    Down,
    /// Show populi health, security posture, and overlay diagnostics.
    Status {
        /// Emit JSON output (also implied by root `--json`).
        #[arg(long, default_value_t = false)]
        json: bool,
    },
}

pub async fn run(cmd: PopuliLifecycleCmd, global_json: bool) -> anyhow::Result<()> {
    match cmd {
        PopuliLifecycleCmd::Up {
            mode,
            scope,
            gpus,
            bind,
            overlay_provider,
            insecure_local,
        } => {
            let root = workspace_root()?;
            let state_dir = root.join(".vox").join("populi");
            fs::create_dir_all(&state_dir)
                .with_context(|| format!("create {}", state_dir.display()))?;
            let env_file = state_dir.join("mesh.env");
            let state_file = state_dir.join("mesh-state.json");

            let mut env_map = load_env_file(&env_file).unwrap_or_default();
            env_map.insert("VOX_MESH_ENABLED".to_string(), "1".to_string());
            env_map.insert(
                "VOX_MESH_MODE".to_string(),
                match mode {
                    PopuliConnectivityMode::Lan => "lan".to_string(),
                    PopuliConnectivityMode::Overlay => "overlay".to_string(),
                },
            );
            env_map.insert("VOX_MESH_NODE_ID".to_string(), default_node_id());
            if gpus.trim().eq_ignore_ascii_case("auto") {
                env_map.entry("VOX_MESH_ADVERTISE_GPU".to_string()).or_insert("1".to_string());
            }

            let scope_id = scope
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .or_else(|| env_map.get("VOX_MESH_SCOPE_ID").cloned())
                .unwrap_or_else(default_scope_id);
            env_map.insert("VOX_MESH_SCOPE_ID".to_string(), scope_id.clone());

            let token = if insecure_local && matches!(mode, PopuliConnectivityMode::Lan) {
                String::new()
            } else {
                env_map
                    .get("VOX_MESH_TOKEN")
                    .cloned()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(generate_populi_token)
            };
            if token.is_empty() {
                env_map.remove("VOX_MESH_TOKEN");
            } else {
                env_map.insert("VOX_MESH_TOKEN".to_string(), token.clone());
            }
            if !token.is_empty() {
                let bootstrap_token = generate_populi_token();
                let bootstrap_expires = vox_populi::wall_clock_unix_ms() + 10 * 60 * 1000;
                env_map.insert("VOX_MESH_BOOTSTRAP_TOKEN".to_string(), bootstrap_token);
                env_map.insert(
                    "VOX_MESH_BOOTSTRAP_EXPIRES_UNIX_MS".to_string(),
                    bootstrap_expires.to_string(),
                );
            }

            let mut provider_name: Option<String> = None;
            let control_url = match mode {
                PopuliConnectivityMode::Lan => format!("http://{}", bind.trim()),
                PopuliConnectivityMode::Overlay => {
                    let provider = choose_overlay_provider(overlay_provider);
                    provider_name = provider.map(|p| p.as_str().to_string());
                    overlay_control_url(provider, &bind)
                }
            };
            env_map.insert("VOX_MESH_CONTROL_ADDR".to_string(), control_url.clone());
            env_map.insert(
                "VOX_ORCHESTRATOR_MESH_CONTROL_URL".to_string(),
                control_url.clone(),
            );

            save_env_file(&env_file, &env_map)?;

            let exe = std::env::current_exe().context("resolve current executable path")?;
            let mut child = std::process::Command::new(exe);
            child
                .arg("populi")
                .arg("serve")
                .arg("--bind")
                .arg(bind.trim())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            for (k, v) in &env_map {
                child.env(k, v);
            }
            let spawned = child.spawn().context("spawn `vox populi serve`")?;
            let pid = spawned.id();

            let state = PopuliDaemonState {
                pid: Some(pid),
                bind: bind.trim().to_string(),
                mode: match mode {
                    PopuliConnectivityMode::Lan => "lan".to_string(),
                    PopuliConnectivityMode::Overlay => "overlay".to_string(),
                },
                control_url: control_url.clone(),
                env_file: env_file.display().to_string(),
                overlay_provider: provider_name,
            };
            save_state_file(&state_file, &state)?;

            println!("Populi started");
            println!("  mode: {}", state.mode);
            println!("  control: {}", state.control_url);
            println!("  scope: {}", scope_id);
            if token.is_empty() {
                println!("  token: disabled (--insecure-local)");
            } else {
                println!("  token: generated and stored in {}", env_file.display());
            }
            println!("  pid: {}", pid);
                println!("  next: vox populi status");
            Ok(())
        }
        PopuliLifecycleCmd::Down => {
            let root = workspace_root()?;
            let state_file = root.join(".vox").join("populi").join("mesh-state.json");
            let state = load_state_file(&state_file).with_context(|| {
                format!(
                    "read populi state; run `vox populi up` first ({})",
                    state_file.display()
                )
            })?;
            let pid = state.pid.context("populi state has no pid")?;
            terminate_pid(pid)?;
            fs::remove_file(&state_file).ok();
            println!("Populi stopped (pid {pid})");
            Ok(())
        }
        PopuliLifecycleCmd::Status { json } => {
            let root = workspace_root()?;
            let mesh_dir = root.join(".vox").join("populi");
            let env_file = mesh_dir.join("mesh.env");
            let state_file = mesh_dir.join("mesh-state.json");
            let env_map = load_env_file(&env_file).unwrap_or_default();
            let state = load_state_file(&state_file).ok();

            let control = state
                .as_ref()
                .map(|s| s.control_url.clone())
                .or_else(|| env_map.get("VOX_MESH_CONTROL_ADDR").cloned())
                .unwrap_or_default();
            let health_ok = if control.is_empty() {
                false
            } else {
                control_plane_health(&control).await
            };

            let diagnostics = overlay_diagnostics();
            let token_set = env_map
                .get("VOX_MESH_TOKEN")
                .is_some_and(|v| !v.trim().is_empty());
            let scope_set = env_map
                .get("VOX_MESH_SCOPE_ID")
                .is_some_and(|v| !v.trim().is_empty());

            if json || global_json {
                let out = serde_json::json!({
                    "state": state,
                    "env_file": env_file.display().to_string(),
                    "health_ok": health_ok,
                    "security": {
                        "token_set": token_set,
                        "scope_set": scope_set
                    },
                    "overlay": diagnostics,
                });
                println!("{}", serde_json::to_string_pretty(&out)?);
            } else {
                println!("Populi status");
                println!("  env_file: {}", env_file.display());
                if let Some(st) = &state {
                    println!("  mode: {}", st.mode);
                    if let Some(pid) = st.pid {
                        println!("  pid: {}", pid);
                    }
                    println!("  control: {}", st.control_url);
                }
                println!("  health: {}", if health_ok { "ok" } else { "down" });
                println!(
                    "  security: token={}, scope={}",
                    if token_set { "set" } else { "missing" },
                    if scope_set { "set" } else { "missing" }
                );
                for diag in diagnostics {
                    println!(
                        "  overlay:{} available={} connected={} detail={}",
                        diag.provider, diag.available, diag.connected, diag.detail
                    );
                }
            }
            Ok(())
        }
    }
}

fn workspace_root() -> anyhow::Result<PathBuf> {
    std::env::current_dir().context("resolve current directory")
}

fn default_scope_id() -> String {
    format!("scope-{}", Uuid::new_v4().simple())
}

fn default_node_id() -> String {
    format!("node-{}", Uuid::new_v4().simple())
}

fn generate_populi_token() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn load_env_file(path: &Path) -> Option<BTreeMap<String, String>> {
    let raw = fs::read_to_string(path).ok()?;
    let mut map = BTreeMap::new();
    for line in raw.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = t.split_once('=') {
            map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    Some(map)
}

fn save_env_file(path: &Path, map: &BTreeMap<String, String>) -> anyhow::Result<()> {
    let mut out = String::from("# Generated by `vox populi up`\n");
    for (k, v) in map {
        out.push_str(k);
        out.push('=');
        out.push_str(v);
        out.push('\n');
    }
    fs::write(path, out).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn load_state_file(path: &Path) -> anyhow::Result<PopuliDaemonState> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("parse {}", path.display()))
}

fn save_state_file(path: &Path, state: &PopuliDaemonState) -> anyhow::Result<()> {
    let text = serde_json::to_string_pretty(state)?;
    fs::write(path, text).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}

fn choose_overlay_provider(arg: OverlayProviderArg) -> Option<OverlayProvider> {
    match arg {
        OverlayProviderArg::Tailscale => Some(OverlayProvider::Tailscale),
        OverlayProviderArg::Wireguard => Some(OverlayProvider::Wireguard),
        OverlayProviderArg::Tunnel => Some(OverlayProvider::Tunnel),
        OverlayProviderArg::Auto => {
            let checks = overlay_diagnostics();
            checks.into_iter().find_map(|d| {
                if d.available && d.connected {
                    match d.provider.as_str() {
                        "tailscale" => Some(OverlayProvider::Tailscale),
                        "wireguard" => Some(OverlayProvider::Wireguard),
                        "tunnel" => Some(OverlayProvider::Tunnel),
                        _ => None,
                    }
                } else {
                    None
                }
            })
        }
    }
}

fn overlay_control_url(provider: Option<OverlayProvider>, bind: &str) -> String {
    let port = bind
        .rsplit(':')
        .next()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(9847);
    match provider {
        Some(OverlayProvider::Tailscale) => {
            if let Ok(ip) = tailscale_ip() {
                return format!("http://{ip}:{port}");
            }
            format!("http://127.0.0.1:{port}")
        }
        _ => format!("http://127.0.0.1:{port}"),
    }
}

fn overlay_diagnostics() -> Vec<OverlayDiagnostics> {
    vec![
        overlay_diag_tailscale(),
        overlay_diag_wireguard(),
        overlay_diag_tunnel(),
    ]
}

fn overlay_diag_tailscale() -> OverlayDiagnostics {
    let available = command_ok("tailscale", &["version"]);
    let connected = if available {
        command_ok("tailscale", &["status"])
    } else {
        false
    };
    let detail = if !available {
        "tailscale command not found".to_string()
    } else if connected {
        "tailscale reachable".to_string()
    } else {
        "tailscale installed but not connected".to_string()
    };
    OverlayDiagnostics {
        provider: "tailscale".to_string(),
        available,
        connected,
        detail,
    }
}

fn overlay_diag_wireguard() -> OverlayDiagnostics {
    let (exe, args): (&str, &[&str]) = if cfg!(windows) {
        ("wg", &["show"])
    } else {
        ("wg", &["show"])
    };
    let available = command_ok(exe, args);
    let connected = available;
    let detail = if available {
        "wireguard command available".to_string()
    } else {
        "wireguard command not found".to_string()
    };
    OverlayDiagnostics {
        provider: "wireguard".to_string(),
        available,
        connected,
        detail,
    }
}

fn overlay_diag_tunnel() -> OverlayDiagnostics {
    let cloudflared = command_ok("cloudflared", &["--version"]);
    let ngrok = command_ok("ngrok", &["version"]);
    let available = cloudflared || ngrok;
    let detail = if cloudflared {
        "cloudflared available".to_string()
    } else if ngrok {
        "ngrok available".to_string()
    } else {
        "no supported tunnel CLI found".to_string()
    };
    OverlayDiagnostics {
        provider: "tunnel".to_string(),
        available,
        connected: available,
        detail,
    }
}

fn command_ok(exe: &str, args: &[&str]) -> bool {
    std::process::Command::new(exe)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

fn tailscale_ip() -> anyhow::Result<String> {
    let out = std::process::Command::new("tailscale")
        .args(["ip", "-4"])
        .output()
        .context("run `tailscale ip -4`")?;
    if !out.status.success() {
        bail!("tailscale ip failed");
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let ip = text
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty())
        .context("tailscale produced no IPv4 address")?;
    Ok(ip.to_string())
}

fn terminate_pid(pid: u32) -> anyhow::Result<()> {
    if cfg!(windows) {
        let status = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status()
            .context("run taskkill")?;
        if !status.success() {
            bail!("taskkill failed for pid {pid}");
        }
        return Ok(());
    }

    let status = std::process::Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .status()
        .context("run kill")?;
    if !status.success() {
        bail!("kill failed for pid {pid}");
    }
    Ok(())
}

async fn control_plane_health(control_url: &str) -> bool {
    let url = format!("{}/health", control_url.trim_end_matches('/'));
    reqwest::Client::new()
        .get(url)
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await
        .is_ok_and(|r| r.status().is_success())
}
