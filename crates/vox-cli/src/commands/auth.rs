use anyhow::{Context, Result};
use clap::Subcommand;
use std::io::{self, Write};
use std::sync::{Arc, Mutex, OnceLock};
use vox_identity::storage::{load_identity, save_identity};
use vox_identity::{NodeIdentity, TrustedNodeRegistry};

#[derive(Subcommand, Debug)]
pub enum AuthCmd {
    /// Generate a new Ed25519 node identity (first-time setup).
    Init,
    /// Unlock the local node identity with master password.
    Unlock,
    /// Show this node's public identity (node_id + pubkey).
    Whoami,
    /// Trust another node by adding its public key to the local registry.
    Trust {
        /// The public key of the node to trust (hex string)
        pubkey_hex: String,
        /// Optional human-readable label for the node
        #[arg(long, short)]
        label: Option<String>,
    },
    /// List all trusted nodes.
    TrustList,
    /// Remove a node from the trust registry.
    Untrust {
        /// The node ID to remove
        node_id: String,
    },
    /// Connect a remote VoxDB (Turso) for cloud sync (canonical with `vox login`).
    #[command(alias = "login")]
    Connect {
        #[command(flatten)]
        args: super::login_shared::LoginArgs,
    },
}

static ACTIVE_IDENTITY: OnceLock<Arc<Mutex<NodeIdentity>>> = OnceLock::new();

pub async fn run(cmd: AuthCmd) -> Result<()> {
    match cmd {
        AuthCmd::Init => run_init().await,
        AuthCmd::Unlock => run_unlock().await,
        AuthCmd::Whoami => run_whoami().await,
        AuthCmd::Trust { pubkey_hex, label } => run_trust(pubkey_hex, label).await,
        AuthCmd::TrustList => run_trust_list().await,
        AuthCmd::Untrust { node_id } => run_untrust(node_id).await,
        AuthCmd::Connect { args } => super::login_shared::run_login(args.into()).await,
    }
}

async fn run_init() -> Result<()> {
    println!("Initializing new Vox Node Identity...");

    print!("Choose a strong master password: ");
    io::stdout().flush()?;
    let mut pwd1 = String::new();
    io::stdin().read_line(&mut pwd1)?;
    let pwd1 = pwd1.trim();

    print!("Confirm master password: ");
    io::stdout().flush()?;
    let mut pwd2 = String::new();
    io::stdin().read_line(&mut pwd2)?;
    let pwd2 = pwd2.trim();

    if pwd1 != pwd2 {
        anyhow::bail!("Passwords do not match. Aborting initialization.");
    }
    if pwd1.is_empty() {
        anyhow::bail!("Password cannot be empty.");
    }

    let identity = NodeIdentity::generate();
    save_identity(&identity, pwd1)?;

    println!("\nSuccess! Node identity generated and encrypted at ~/.vox/identity.key.enc");
    println!("Node ID: {}", identity.node_id());

    let pubkey_bytes = vox_crypto::verifying_key_to_bytes(&identity.verifying_key);
    println!("Public Key: {}", hex::encode(pubkey_bytes));

    // Cache it for the session
    ACTIVE_IDENTITY
        .set(Arc::new(Mutex::new(identity)))
        .map_err(|_| anyhow::anyhow!("Failed to set active identity"))?;

    Ok(())
}

async fn run_unlock() -> Result<()> {
    print!("Master Password: ");
    io::stdout().flush()?;
    let mut pwd = String::new();
    io::stdin().read_line(&mut pwd)?;
    let pwd = pwd.trim();

    let identity = load_identity(pwd)?;
    println!("Vault successfully unlocked via Argon2-derived master key.");

    // Cache it
    let _ = ACTIVE_IDENTITY.set(Arc::new(Mutex::new(identity)));
    Ok(())
}

async fn run_whoami() -> Result<()> {
    let identity = match ACTIVE_IDENTITY.get() {
        Some(locked) => locked.lock().unwrap(),
        None => {
            // Try loading from OS keyring if implemented, otherwise ask to unlock
            anyhow::bail!("Node identity is locked. Run `vox auth unlock` first.");
        }
    };

    println!("Node ID: {}", identity.node_id());
    let pubkey_bytes = vox_crypto::verifying_key_to_bytes(&identity.verifying_key);
    println!("Public Key: {}", hex::encode(pubkey_bytes));

    Ok(())
}

async fn run_trust(pubkey_hex: String, label: Option<String>) -> Result<()> {
    if pubkey_hex.len() != 64 {
        anyhow::bail!("Public key must be exactly 64 hex characters.");
    }

    let pubkey_bytes = hex::decode(&pubkey_hex).context("Invalid hex encoding")?;
    let hash = vox_crypto::secure_hash(&pubkey_bytes);
    let node_id = hex::encode(&hash[0..16]);

    let registry = TrustedNodeRegistry::new();
    registry.add(node_id.clone(), pubkey_hex, label.clone())?;

    println!("Successfully trusted node: {}", node_id);
    if let Some(l) = label {
        println!("Label: {}", l);
    }

    Ok(())
}

async fn run_trust_list() -> Result<()> {
    let registry = TrustedNodeRegistry::new();
    let nodes = registry.list()?;

    if nodes.is_empty() {
        println!("No trusted nodes found.");
        return Ok(());
    }

    println!("{:<32} | {:<20} | {:<32}", "Node ID", "Label", "Added At");
    println!("{:-<32}-+-{:-<20}-+-{:-<32}", "", "", "");
    for node in nodes {
        let label = node.label.unwrap_or_else(|| "<none>".to_string());
        println!(
            "{:<32} | {:<20} | {:<32}",
            node.node_id, label, node.added_at
        );
    }

    Ok(())
}

async fn run_untrust(node_id: String) -> Result<()> {
    let registry = TrustedNodeRegistry::new();
    if registry.remove(&node_id)? {
        println!("Successfully removed node {} from trust registry.", node_id);
    } else {
        println!("Node {} was not found in the trust registry.", node_id);
    }
    Ok(())
}
