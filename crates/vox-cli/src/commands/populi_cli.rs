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
    /// Mesh node management (join, leave, list).
    Node {
        #[command(subcommand)]
        cmd: PopuliNodeCmd,
    },
    /// Dispatch a script for remote execution on the mesh.
    Dispatch {
        /// Path to the .vox script or compiled bundle.
        script: PathBuf,
        /// Target node id (optional).
        #[arg(long)]
        node: Option<String>,
        /// Execution timeout in seconds.
        #[arg(long, default_value_t = 30)]
        timeout: u64,
        /// Control plane base URL.
        #[arg(long)]
        control_url: Option<String>,
        /// Send as a pre-compiled bundle. If input is a .vox file, it will be bundled first.
        #[arg(long)]
        bundle: bool,
        /// Required capability labels for routing (k=v,...).
        #[arg(long, value_delimiter = ',')]
        routing_labels: Vec<String>,
        /// Return immediately and poll for results later.
        #[arg(long)]
        detach: bool,
    },
    /// Retrieve the result of a detached dispatch by its unique id.
    Result {
        /// Dispatch ID returned from a detached execution.
        dispatch_id: String,
        /// Control plane base URL.
        #[arg(long)]
        control_url: Option<String>,
    },
}

/// Mesh node management subcommands.
#[derive(Subcommand)]
pub enum PopuliNodeCmd {
    /// Join the mesh and start a worker listener for dispatch requests.
    Join {
        /// Control plane base URL.
        #[arg(long)]
        control_url: Option<String>,
        /// Labels to advertise (k=v,...).
        #[arg(long, value_delimiter = ',')]
        labels: Vec<String>,
        /// Address to listen on for incoming dispatch requests (e.g. 0.0.0.0:9848).
        #[arg(long, default_value = "0.0.0.0:9848")]
        bind: String,
    },
    /// Leave the mesh.
    Leave {
        /// Node id (defaults to current process id from env).
        #[arg(long)]
        node: Option<String>,
        /// Control plane base URL.
        #[arg(long)]
        control_url: Option<String>,
    },
    /// List all nodes in the mesh.
    List {
        /// Control plane base URL.
        #[arg(long)]
        control_url: Option<String>,
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
        PopuliCli::Node { cmd } => match cmd {
            PopuliNodeCmd::Join {
                control_url,
                labels,
                bind,
            } => {
                let base = resolve_populi_control_base(control_url)?;
                let addr: SocketAddr = bind
                    .parse()
                    .with_context(|| format!("invalid --bind address: {bind}"))?;

                // Build record
                let env = vox_populi::populi_env();
                let id = env
                    .node_id
                    .clone()
                    .unwrap_or_else(|| format!("node-{}", vox_primitives::id::simple_hex_id()));

                let mut record = vox_populi::node_record_for_current_process(id, Some(bind.clone()));
                for lab in labels {
                    if !record.capabilities.labels.contains(&lab) {
                        record.capabilities.labels.push(lab);
                    }
                }

                // Join
                let client = vox_populi::http_client::PopuliHttpClient::new(&base).with_env_token();
                let updated = client
                    .join(&record)
                    .await
                    .map_err(|e| anyhow::anyhow!("Mesh join failed: {}", e))?;

                println!("Joined mesh at {} as node {}", base, updated.id);

                // Start heartbeat loop and worker listener
                let state = vox_populi::transport::PopuliTransportState::new_for_serve();
                
                // Spawn heartbeat
                let base_cl = base.clone();
                let timeout = vox_populi::http_lifecycle::populi_http_timeout_ms_from_env();
                let interval = vox_populi::http_lifecycle::populi_heartbeat_interval_secs_from_env();
                
                if interval > 0 {
                    tokio::spawn(async move {
                        use sysinfo::System;
                        let mut sys = System::new_all();
                        
                        let mut tick = tokio::time::interval(std::time::Duration::from_secs(interval));
                        let client = vox_populi::http_client::PopuliHttpClient::new_with_timeout(&base_cl, std::time::Duration::from_millis(timeout))
                            .with_env_token();
                        let mut current_record = updated.clone();
                        
                        loop {
                            tick.tick().await;
                            sys.refresh_cpu_all();
                            sys.refresh_memory();
                            
                            current_record.cpu_usage_pct = Some(sys.global_cpu_usage());
                            current_record.memory_free_bytes = Some(sys.available_memory());
                            
                            let _ = client.heartbeat(&current_record).await;
                        }
                    });
                }

                println!("Worker listening for dispatch on {}", addr);
                vox_populi::transport::serve(addr, state)
                    .await
                    .with_context(|| format!("populi worker serve on {addr}"))?;

                Ok(())
            }
            PopuliNodeCmd::Leave { node, control_url } => {
                let base = resolve_populi_control_base(control_url)?;
                let env = vox_populi::populi_env();
                let id = node.or(env.node_id).ok_or_else(|| {
                    anyhow::anyhow!("node id required (set --node or VOX_MESH_NODE_ID)")
                })?;

                let client = vox_populi::http_client::PopuliHttpClient::new(&base).with_env_token();
                let found = client
                    .leave(&id)
                    .await
                    .map_err(|e| anyhow::anyhow!("Mesh leave failed: {}", e))?;

                if found {
                    println!("Node {} left mesh {}", id, base);
                } else {
                    println!("Node {} was not found in mesh {}", id, base);
                }
                Ok(())
            }
            PopuliNodeCmd::List { control_url } => {
                let base = resolve_populi_control_base(control_url)?;
                let client = vox_populi::http_client::PopuliHttpClient::new(&base).with_env_token();
                let reg = client
                    .list_nodes()
                    .await
                    .map_err(|e| anyhow::anyhow!("Failed to list nodes: {}", e))?;

                if global_json {
                    println!("{}", serde_json::to_string_pretty(&reg)?);
                } else {
                    println!("Mesh nodes at {}:", base);
                    for n in reg.nodes {
                        println!(
                            "  - {} (version: {}, triple: {}, caps: {:?})",
                            n.id,
                            n.version,
                            n.host_triple.as_deref().unwrap_or("unknown"),
                            n.capabilities.labels
                        );
                    }
                }
                Ok(())
            }
        },
        PopuliCli::Dispatch {
            script,
            node,
            timeout,
            control_url,
            bundle,
            routing_labels,
            detach,
        } => {
            let base = resolve_populi_control_base(control_url)?;
            let client = vox_populi::http_client::PopuliHttpClient::new(&base).with_env_token();

            // Phase 3: Autonomous Target Resolution
            let mut target_triple = None;
            if let Some(node_id) = &node {
                // Query node metadata to find target triple
                if let Ok(reg) = client.list_nodes().await {
                    if let Some(n) = reg.nodes.iter().find(|n| n.id == *node_id) {
                        target_triple = n.host_triple.clone();
                    }
                }
            }

            let (b64_source, is_bundle, source_blake3_hex) = if bundle && script.extension().map_or(false, |ext| ext == "vox") {
                // Auto-bundle source to a temp dir
                let tmp_bundle_dir = std::env::temp_dir().join("vox-bundle-dispatch");
                let _ = std::fs::remove_dir_all(&tmp_bundle_dir);
                
                // Use resolved triple if found, otherwise let it be None (host native)
                crate::commands::bundle::run(&script, &tmp_bundle_dir, target_triple.as_deref(), true, crate::cli_args::BundleMode::Script).await?;
                
                // Find the binary in the temp dir
                let mut entries = std::fs::read_dir(&tmp_bundle_dir)?;
                let mut bin_path = None;
                while let Some(Ok(entry)) = entries.next() {
                    if entry.path().is_file() {
                        bin_path = Some(entry.path());
                        break;
                    }
                }
                
                let bin_path = bin_path.ok_or_else(|| anyhow::anyhow!("Bundling failed to produce a binary"))?;
                let bin_bytes = std::fs::read(&bin_path)?;
                let hash_hex = blake3::hash(&bin_bytes).to_hex().to_string();
                use base64::Engine as _;
                (base64::engine::general_purpose::STANDARD.encode(bin_bytes), true, Some(hash_hex))
            } else if bundle {
                // Input is already a bundle binary
                let bin_bytes = std::fs::read(&script)?;
                let hash_hex = blake3::hash(&bin_bytes).to_hex().to_string();
                use base64::Engine as _;
                (base64::engine::general_purpose::STANDARD.encode(bin_bytes), true, Some(hash_hex))
            } else {
                // Send as source
                let source = std::fs::read_to_string(&script)
                    .with_context(|| format!("failed to read script {}", script.display()))?;
                let hash_hex = blake3::hash(source.as_bytes()).to_hex().to_string();
                use base64::Engine as _;
                (base64::engine::general_purpose::STANDARD.encode(&source), false, Some(hash_hex))
            };

            let req = vox_populi::transport::DispatchRequest {
                source: b64_source,
                node_id: node,
                timeout_secs: timeout,
                is_bundle,
                source_blake3_hex,
                required_labels: if routing_labels.is_empty() { None } else { Some(routing_labels) },
                is_detached: detach,
            };

            let resp = client
                .dispatch(&req)
                .await
                .map_err(|e| anyhow::anyhow!("Dispatch failed: {e}"))?;

            if global_json {
                println!("{}", serde_json::to_string_pretty(&resp)?);
            } else {
                if resp.success {
                    println!("✓ Dispatch Success");
                    println!("  Node:     {}", resp.node_id);
                    println!("  Duration: {:.2}s", resp.duration_ms as f64 / 1000.0);
                    if let Some(code) = resp.exit_code {
                        println!("  ExitCode: {}", code);
                    }
                    if resp.is_truncated {
                        println!("  Warning:  Output was truncated due to length limits (10MB).");
                    }
                    println!("\nOutput:\n{}", resp.output);
                } else {
                    eprintln!("✗ Dispatch Failure");
                    eprintln!("  Node:     {}", resp.node_id);
                    eprintln!("  Duration: {:.2}s", resp.duration_ms as f64 / 1000.0);
                    if let Some(code) = resp.exit_code {
                        eprintln!("  ExitCode: {}", code);
                    }
                    if let Some(err) = resp.error {
                        eprintln!("  Error:    {}", err);
                    }
                    if resp.is_truncated {
                        eprintln!("  Warning:  Output was truncated due to length limits (10MB).");
                    }
                    eprintln!("\nOutput:\n{}", resp.output);
                    anyhow::bail!("Remote execution failed");
                }
            }
            Ok(())
        }
        PopuliCli::Result {
            dispatch_id,
            control_url,
        } => {
            let base = resolve_populi_control_base(control_url)?;
            let client = vox_populi::http_client::PopuliHttpClient::new(&base).with_env_token();
            let resp = client
                .dispatch_result_poll(&dispatch_id)
                .await
                .map_err(|e| anyhow::anyhow!("Result poll failed: {e}"))?;

            if global_json {
                println!("{}", serde_json::to_string_pretty(&resp)?);
            } else {
                if resp.success {
                    println!("✓ Result Fetched for Dispatch {}", dispatch_id);
                    println!("  Node:     {}", resp.node_id);
                    println!("  Duration: {:.2}s", resp.duration_ms as f64 / 1000.0);
                    if let Some(code) = resp.exit_code {
                        println!("  ExitCode: {}", code);
                    }
                    println!("\nOutput:\n{}", resp.output);
                } else {
                    eprintln!("✗ Result Error for Dispatch {}", dispatch_id);
                    if let Some(err) = resp.error {
                        eprintln!("\nError:\n{}", err);
                    }
                }
            }
            Ok(())
        }
    }
}
