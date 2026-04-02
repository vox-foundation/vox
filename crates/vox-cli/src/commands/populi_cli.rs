//! `vox populi …` — local registry and HTTP control plane (requires `--features populi`).

use std::net::SocketAddr;
use std::path::PathBuf;

use crate::commands::populi_lifecycle::{
    OverlayProviderArg, PopuliConnectivityMode, PopuliLifecycleCmd,
};
use anyhow::Context;
use clap::{Subcommand, ValueEnum};

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum PopuliAdminSwitch {
    On,
    Off,
}

/// Operator HTTP actions (`POST /v1/populi/admin/*`; mesh or admin bearer via Clavis env).
#[derive(Subcommand)]
pub enum PopuliAdminCmd {
    /// Cooperative drain (blocks new exec lease grant/renew and A2A claims for this node id).
    Maintenance {
        /// `NodeRecord.id`
        #[arg(long)]
        node: String,
        #[arg(long, value_enum)]
        state: PopuliAdminSwitch,
        /// Absolute Unix ms deadline when `--state on` (server drops drain after this time). Conflicts with `--for-minutes` (absolute wins).
        #[arg(long)]
        until_unix_ms: Option<u64>,
        /// When `--state on`, set deadline to now + this many minutes on the server (capped by server max).
        #[arg(long)]
        for_minutes: Option<u64>,
    },
    /// Quarantine (hard block on A2A claims for this node id).
    Quarantine {
        /// `NodeRecord.id`
        #[arg(long)]
        node: String,
        #[arg(long, value_enum)]
        state: PopuliAdminSwitch,
    },
    /// Force-remove a remote exec lease row by id (no holder cooperation; mesh/admin bearer).
    ExecLeaseRevoke {
        /// `RemoteExecLeaseGrantResponse.lease_id` from grant (or `GET /v1/populi/exec/leases`).
        #[arg(long)]
        lease_id: String,
    },
}

/// Populi mesh subcommands.
#[derive(Subcommand)]
pub enum PopuliCli {
    /// Start a private populi network with secure defaults.
    Up {
        /// Connectivity strategy.
        #[arg(long, value_enum, default_value_t = PopuliConnectivityMode::Lan)]
        mode: PopuliConnectivityMode,
        /// Populi scope id (auto-generated when omitted).
        #[arg(long)]
        scope: Option<String>,
        /// GPU advertisement policy (`auto` uses probe defaults).
        #[arg(long, default_value = "auto")]
        gpus: String,
        /// Control-plane bind address.
        #[arg(long, default_value = "127.0.0.1:9847")]
        bind: String,
        /// Overlay provider selection (`auto` probes available providers).
        #[arg(long, value_enum, default_value_t = OverlayProviderArg::Auto)]
        overlay_provider: OverlayProviderArg,
        /// Allow local insecure mode (disables required mesh token).
        #[arg(long, default_value_t = false)]
        insecure_local: bool,
    },
    /// Stop the populi process started by `vox populi up`.
    Down,
    /// Show populi network status, health, and overlay diagnostics.
    Status {
        /// Emit JSON (also implied by global `--json`).
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Print local env and on-disk registry snapshot.
    #[command(name = "registry-snapshot", visible_alias = "local-status")]
    RegistrySnapshot {
        /// Override registry file (default: `VOX_MESH_REGISTRY_PATH` or `~/.vox/cache/mens/local-registry.json`).
        #[arg(long)]
        registry: Option<PathBuf>,
        /// Emit JSON (also implied by global `--json`).
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Run the HTTP populi control plane (`GET /v1/populi/nodes`, `POST` join/heartbeat).
    Serve {
        /// Listen address (e.g. `127.0.0.1:9847` or `0.0.0.0:9847`).
        #[arg(long, default_value = "127.0.0.1:9847")]
        bind: String,
        /// Seed in-memory state from this registry file on startup (optional).
        #[arg(long)]
        registry: Option<PathBuf>,
    },
    /// Maintenance and quarantine toggles on a running control plane.
    Admin {
        /// Control plane base (`VOX_ORCHESTRATOR_MESH_CONTROL_URL`, then `VOX_MESH_CONTROL_ADDR`, when omitted).
        #[arg(long)]
        control_url: Option<String>,
        #[command(subcommand)]
        cmd: PopuliAdminCmd,
    },
}

fn resolve_populi_control_base(url_override: Option<String>) -> anyhow::Result<String> {
    let raw = if let Some(u) = url_override {
        u
    } else if let Ok(u) = std::env::var("VOX_ORCHESTRATOR_MESH_CONTROL_URL") {
        u.trim().to_string()
    } else if let Some(u) = vox_populi::populi_env().control_addr {
        u
    } else {
        anyhow::bail!(
            "set --control-url or VOX_ORCHESTRATOR_MESH_CONTROL_URL or VOX_MESH_CONTROL_ADDR"
        );
    };
    vox_populi::normalize_http_control_base(raw.trim())
        .ok_or_else(|| anyhow::anyhow!("invalid or bind-all control URL: {}", raw.trim()))
}

/// Run a `vox populi` subcommand.
pub async fn run(cmd: PopuliCli, global_json: bool) -> anyhow::Result<()> {
    match cmd {
        PopuliCli::Up {
            mode,
            scope,
            gpus,
            bind,
            overlay_provider,
            insecure_local,
        } => {
            crate::commands::populi_lifecycle::run(
                PopuliLifecycleCmd::Up {
                    mode,
                    scope,
                    gpus,
                    bind,
                    overlay_provider,
                    insecure_local,
                },
                global_json,
            )
            .await
        }
        PopuliCli::Down => {
            crate::commands::populi_lifecycle::run(PopuliLifecycleCmd::Down, global_json).await
        }
        PopuliCli::Status { json } => {
            crate::commands::populi_lifecycle::run(PopuliLifecycleCmd::Status { json }, global_json)
                .await
        }
        PopuliCli::RegistrySnapshot { registry, json } => {
            let path = registry.unwrap_or_else(vox_populi::local_registry_path);
            let reg = vox_populi::LocalRegistry::new(path.clone());
            let file = reg.load()?;
            let env = vox_populi::populi_env();
            let self_id = env
                .node_id
                .clone()
                .unwrap_or_else(|| format!("local-{}", vox_runtime::simple_id::simple_hex_id()));
            let self_record =
                vox_populi::node_record_for_current_process(self_id, env.control_addr.clone());
            let as_json = json || global_json;
            if as_json {
                let v = serde_json::json!({
                    "populi_env": env,
                    "registry_path": reg.path().display().to_string(),
                    "registry": file,
                    "self_record": self_record,
                });
                println!("{}", serde_json::to_string_pretty(&v)?);
            } else {
                println!("Mesh env:");
                println!("  VOX_MESH_ENABLED: {}", env.enabled);
                if let Some(ref id) = env.node_id {
                    println!("  VOX_MESH_NODE_ID: {id}");
                }
                if !env.labels.is_empty() {
                    println!("  VOX_MESH_LABELS: {}", env.labels.join(","));
                }
                if let Some(ref a) = env.control_addr {
                    println!("  VOX_MESH_CONTROL_ADDR: {a}");
                }
                if let Some(ref p) = env.registry_path {
                    println!("  VOX_MESH_REGISTRY_PATH: {p}");
                }
                if let Some(ref s) = env.scope_id {
                    println!("  VOX_MESH_SCOPE_ID: {s}");
                }
                println!();
                println!("Registry file: {}", reg.path().display());
                println!("  nodes: {}", file.nodes.len());
                for n in &file.nodes {
                    println!(
                        "    - {} @ {} ms (caps cpu_cores={:?})",
                        n.id, n.last_seen_unix_ms, n.capabilities.cpu_cores
                    );
                }
                println!();
                println!("This process (probe) node id: {}", self_record.id);
            }
            Ok(())
        }
        PopuliCli::Serve { bind, registry } => {
            let addr: SocketAddr = bind
                .parse()
                .with_context(|| format!("invalid --bind address: {bind}"))?;
            let state = if let Some(p) = registry {
                vox_populi::transport::PopuliTransportState::load_from_path(&p)
                    .await
                    .with_context(|| format!("load registry {}", p.display()))?
            } else {
                vox_populi::transport::PopuliTransportState::new_for_serve()
            };
            vox_populi::transport::serve(addr, state)
                .await
                .with_context(|| format!("populi HTTP serve on {addr}"))?;
            Ok(())
        }
        PopuliCli::Admin { control_url, cmd } => {
            let base = resolve_populi_control_base(control_url)?;
            let client = vox_populi::http_client::PopuliHttpClient::new(&base).with_env_token();
            match cmd {
                PopuliAdminCmd::Maintenance {
                    node,
                    state,
                    until_unix_ms,
                    for_minutes,
                } => {
                    let node_id = node.trim().to_string();
                    if node_id.is_empty() {
                        anyhow::bail!("--node must be non-empty");
                    }
                    let on = matches!(state, PopuliAdminSwitch::On);
                    if !on && (until_unix_ms.is_some() || for_minutes.is_some()) {
                        anyhow::bail!("--until-unix-ms / --for-minutes require --state on");
                    }
                    if until_unix_ms.is_some() && for_minutes.is_some() {
                        anyhow::bail!("use only one of --until-unix-ms or --for-minutes");
                    }
                    let maintenance_for_ms = for_minutes.map(|m| m.saturating_mul(60_000));
                    client
                        .admin_maintenance(&vox_populi::transport::AdminMaintenanceRequest {
                            node_id,
                            maintenance: on,
                            maintenance_until_unix_ms: until_unix_ms,
                            maintenance_for_ms,
                        })
                        .await
                        .map_err(|e| anyhow::anyhow!(e))?;
                }
                PopuliAdminCmd::Quarantine { node, state } => {
                    let node_id = node.trim().to_string();
                    if node_id.is_empty() {
                        anyhow::bail!("--node must be non-empty");
                    }
                    client
                        .admin_quarantine(&vox_populi::transport::AdminQuarantineRequest {
                            node_id,
                            quarantined: matches!(state, PopuliAdminSwitch::On),
                        })
                        .await
                        .map_err(|e| anyhow::anyhow!(e))?;
                }
                PopuliAdminCmd::ExecLeaseRevoke { lease_id } => {
                    let lease_id = lease_id.trim().to_string();
                    if lease_id.is_empty() {
                        anyhow::bail!("--lease-id must be non-empty");
                    }
                    client
                        .admin_exec_lease_revoke(
                            &vox_populi::transport::AdminExecLeaseRevokeRequest { lease_id },
                        )
                        .await
                        .map_err(|e| anyhow::anyhow!(e))?;
                }
            }
            if global_json {
                println!("{}", serde_json::json!({ "ok": true }));
            } else {
                println!("ok");
            }
            Ok(())
        }
    }
}
