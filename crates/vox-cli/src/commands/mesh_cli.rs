//! `vox mesh …` — local registry and HTTP control plane (requires `--features mesh`).

use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::Context;
use clap::Subcommand;

/// Mesh subcommands.
#[derive(Subcommand)]
pub enum MeshCli {
    /// Print mesh environment, on-disk registry contents, and this process node record.
    Status {
        /// Override registry file (default: `VOX_MESH_REGISTRY_PATH` or `~/.vox/cache/mesh/local-registry.json`).
        #[arg(long)]
        registry: Option<PathBuf>,
        /// Emit JSON (also implied by global `--json`).
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Run the HTTP mesh control plane (`GET /v1/mesh/nodes`, `POST` join/heartbeat).
    Serve {
        /// Listen address (e.g. `127.0.0.1:9847` or `0.0.0.0:9847`).
        #[arg(long, default_value = "127.0.0.1:9847")]
        bind: String,
        /// Seed in-memory state from this registry file on startup (optional).
        #[arg(long)]
        registry: Option<PathBuf>,
    },
}

/// Run a `vox mesh` subcommand.
pub async fn run(cmd: MeshCli, global_json: bool) -> anyhow::Result<()> {
    match cmd {
        MeshCli::Status { registry, json } => {
            let path = registry.unwrap_or_else(vox_mesh::local_registry_path);
            let reg = vox_mesh::LocalRegistry::new(path.clone());
            let file = reg.load()?;
            let env = vox_mesh::mesh_env();
            let self_id = env
                .node_id
                .clone()
                .unwrap_or_else(|| format!("local-{}", uuid_simple()));
            let self_record =
                vox_mesh::node_record_for_current_process(self_id, env.control_addr.clone());
            let as_json = json || global_json;
            if as_json {
                let v = serde_json::json!({
                    "mesh_env": env,
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
        MeshCli::Serve { bind, registry } => {
            let addr: SocketAddr = bind
                .parse()
                .with_context(|| format!("invalid --bind address: {bind}"))?;
            let state = if let Some(p) = registry {
                vox_mesh::transport::MeshTransportState::load_from_path(&p)
                    .await
                    .with_context(|| format!("load registry {}", p.display()))?
            } else {
                vox_mesh::transport::MeshTransportState::new_for_serve()
            };
            vox_mesh::transport::serve(addr, state)
                .await
                .with_context(|| format!("mesh HTTP serve on {addr}"))?;
            Ok(())
        }
    }
}

fn uuid_simple() -> String {
    use std::time::Instant;
    let n = Instant::now().elapsed().as_nanos();
    format!("{n:x}")
}
