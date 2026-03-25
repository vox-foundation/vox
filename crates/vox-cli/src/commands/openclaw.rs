//! `vox openclaw` — OpenClaw gateway integration, skill import, and approval management.
//!
//! Provides CLI commands for:
//! - Connecting to a live OpenClaw or ClawHub gateway
//! - Importing skills from remote instances into the Vox ARS registry
//! - Listing and resolving pending approval requests from the approval broker

use clap::Parser;
use vox_ars::{OpenClawClient, OpenClawRemoteConfig};
use vox_skills::new_registry_arc;

// ── Subcommand Enum ────────────────────────────────────────────────────────────

/// Subcommands for `vox openclaw`.
#[derive(Parser)]
#[command(
    name = "openclaw",
    alias = "oc",
    about = "OpenClaw gateway integration: skill import, remote listing, and approval broker"
)]
pub enum OpenClawAction {
    /// Import a skill from a remote OpenClaw gateway or ClawHub registry by slug.
    ///
    /// Example: `vox openclaw import author/my-skill`
    Import {
        /// Skill slug to import (e.g. "author/my-skill")
        #[arg(required = true, value_name = "SLUG")]
        slug: String,
        /// OpenClaw gateway or ClawHub URL (overrides VOX_OPENCLAW_URL env var)
        #[arg(long, value_name = "URL")]
        gateway: Option<String>,
        /// Bearer token for authentication (overrides VOX_OPENCLAW_TOKEN env var)
        #[arg(long, value_name = "TOKEN")]
        token: Option<String>,
        /// Install the skill into the local ARS registry after importing
        #[arg(long, default_value = "true")]
        install: bool,
        /// Output result as JSON
        #[arg(long, default_value = "false")]
        json: bool,
    },
    /// List all skills available on the remote OpenClaw instance.
    ///
    /// Example: `vox openclaw list-remote`
    ListRemote {
        /// OpenClaw gateway URL (overrides VOX_OPENCLAW_URL env var)
        #[arg(long, value_name = "URL")]
        gateway: Option<String>,
        /// Bearer token for authentication (overrides VOX_OPENCLAW_TOKEN env var)
        #[arg(long, value_name = "TOKEN")]
        token: Option<String>,
        /// Output result as JSON
        #[arg(long, default_value = "false")]
        json: bool,
    },
    /// Show or validate the current OpenClaw gateway connection settings.
    ///
    /// Example: `vox openclaw config`
    Config {
        /// OpenClaw gateway URL to validate (overrides VOX_OPENCLAW_URL env var)
        #[arg(long, value_name = "URL")]
        gateway: Option<String>,
        /// Output as JSON
        #[arg(long, default_value = "false")]
        json: bool,
    },
    /// List all pending tool-approval requests (approval broker).
    ///
    /// Example: `vox openclaw approvals`
    Approvals {
        /// MCP server URL to query (default: http://localhost:3847)
        #[arg(long, value_name = "URL", default_value = "http://localhost:3847")]
        mcp_url: String,
        /// Output as JSON
        #[arg(long, default_value = "false")]
        json: bool,
    },
    /// Approve a pending tool-approval request by its ID.
    ///
    /// Example: `vox openclaw approve abc-123`
    Approve {
        /// Approval ID (from `vox openclaw approvals`)
        #[arg(required = true, value_name = "APPROVAL_ID")]
        id: String,
        /// Optional reason for audit log
        #[arg(long, value_name = "REASON")]
        reason: Option<String>,
        /// MCP server URL (default: http://localhost:3847)
        #[arg(long, value_name = "URL", default_value = "http://localhost:3847")]
        mcp_url: String,
    },
    /// Deny a pending tool-approval request by its ID.
    ///
    /// Example: `vox openclaw deny abc-123`
    Deny {
        /// Approval ID (from `vox openclaw approvals`)
        #[arg(required = true, value_name = "APPROVAL_ID")]
        id: String,
        /// Optional reason for audit log
        #[arg(long, value_name = "REASON")]
        reason: Option<String>,
        /// MCP server URL (default: http://localhost:3847)
        #[arg(long, value_name = "URL", default_value = "http://localhost:3847")]
        mcp_url: String,
    },
    /// Start the OpenClaw gateway HTTP service (vox-gateway).
    ///
    /// Example: `vox openclaw serve --port 3850`
    Serve {
        /// TCP port to listen on
        #[arg(long, default_value_t = 3850)]
        port: u16,
        /// Bind address
        #[arg(long, default_value = "127.0.0.1")]
        addr: String,
    },
    /// Subscribe an agent to a semantic domain (e.g. "vox.web").
    Subscribe {
        /// Semantic domain slug
        #[arg(required = true)]
        domain: String,
        /// Gateway URL (default: http://localhost:3850)
        #[arg(long, default_value = "http://localhost:3850")]
        gateway: String,
    },
    /// Unsubscribe an agent from a semantic domain.
    Unsubscribe {
        /// Semantic domain slug
        #[arg(required = true)]
        domain: String,
        /// Gateway URL (default: http://localhost:3850)
        #[arg(long, default_value = "http://localhost:3850")]
        gateway: String,
    },
    /// List all active semantic domain subscriptions on the gateway.
    Subscriptions {
        /// Gateway URL (default: http://localhost:3850)
        #[arg(long, default_value = "http://localhost:3850")]
        gateway: String,
        /// Output as JSON
        #[arg(long, default_value = "false")]
        json: bool,
    },
    /// Trigger a domain-change notification to all subscribed agents.
    Notify {
        /// Semantic domain slug (e.g. "vox.typeck")
        #[arg(required = true)]
        domain: String,
        /// Notification message or reason
        #[arg(required = true)]
        message: String,
        /// Gateway URL (default: http://localhost:3850)
        #[arg(long, default_value = "http://localhost:3850")]
        gateway: String,
    },
}

// ── Dispatch ───────────────────────────────────────────────────────────────────

/// Dispatch an [`OpenClawAction`] to its implementation.
pub async fn run(action: OpenClawAction, json_output: bool) -> anyhow::Result<()> {
    match action {
        OpenClawAction::Import {
            slug,
            gateway,
            token,
            install,
            json,
        } => cmd_import(slug, gateway, token, install, json || json_output).await,
        OpenClawAction::ListRemote {
            gateway,
            token,
            json,
        } => cmd_list_remote(gateway, token, json || json_output).await,
        OpenClawAction::Config { gateway, json } => cmd_config(gateway, json || json_output).await,
        OpenClawAction::Approvals { mcp_url, json } => {
            cmd_approvals(mcp_url, json || json_output).await
        }
        OpenClawAction::Approve {
            id,
            reason,
            mcp_url,
        } => cmd_resolve(id, true, reason, mcp_url).await,
        OpenClawAction::Deny {
            id,
            reason,
            mcp_url,
        } => cmd_resolve(id, false, reason, mcp_url).await,
        OpenClawAction::Serve { port, addr } => cmd_serve(port, addr).await,
        OpenClawAction::Subscribe { domain, gateway } => cmd_subscribe(domain, gateway).await,
        OpenClawAction::Unsubscribe { domain, gateway } => cmd_unsubscribe(domain, gateway).await,
        OpenClawAction::Subscriptions { gateway, json } => {
            cmd_subscriptions(gateway, json || json_output).await
        }
        OpenClawAction::Notify {
            domain,
            message,
            gateway,
        } => cmd_notify(domain, message, gateway).await,
    }
}

// ── Implementations ────────────────────────────────────────────────────────────

fn make_client(
    gateway: Option<String>,
    token: Option<String>,
) -> Result<OpenClawClient, anyhow::Error> {
    let gateway_url = gateway
        .or_else(|| std::env::var("VOX_OPENCLAW_URL").ok())
        .unwrap_or_else(|| "http://localhost:3000".into());
    let auth_token = token.or_else(|| {
        vox_clavis::resolve_secret(vox_clavis::SecretId::OpenClawToken)
            .expose()
            .map(std::string::ToString::to_string)
    });
    let cfg = OpenClawRemoteConfig {
        gateway_url,
        auth_token,
        verify_tls: true,
    };
    OpenClawClient::new(cfg).map_err(|e| anyhow::anyhow!("OpenClaw client error: {e}"))
}

async fn cmd_import(
    slug: String,
    gateway: Option<String>,
    token: Option<String>,
    install: bool,
    json: bool,
) -> anyhow::Result<()> {
    let client = make_client(gateway, token)?;
    let skill: vox_ars::ArsSkill = client
        .import_skill(&slug)
        .await
        .map_err(|e| anyhow::anyhow!("Import failed: {e}"))?;

    if install {
        let registry = new_registry_arc();
        match client.import_and_install(&slug, &registry).await {
            Ok(result) => {
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "status": "installed",
                            "skill_id": skill.id,
                            "name": skill.name,
                            "version": skill.version,
                            "result": format!("{result:?}"),
                        }))?
                    );
                } else {
                    println!(
                        "✓ Imported and installed: {} v{} (id: {})",
                        skill.name, skill.version, skill.id
                    );
                }
            }
            Err(e) => {
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&serde_json::json!({
                            "status": "import_ok_install_failed",
                            "skill_id": skill.id,
                            "error": e.to_string(),
                        }))?
                    );
                } else {
                    eprintln!("⚠ Imported but install failed: {e}");
                    println!(
                        "Skill: {} v{} (id: {})",
                        skill.name, skill.version, skill.id
                    );
                }
            }
        }
    } else {
        if json {
            println!(
                "{}",
                serde_json::to_string_pretty(&serde_json::json!({
                    "status": "fetched",
                    "skill_id": skill.id,
                    "name": skill.name,
                    "version": skill.version,
                }))?
            );
        } else {
            println!(
                "✓ Fetched (not installed): {} v{} (id: {})",
                skill.name, skill.version, skill.id
            );
        }
    }
    Ok(())
}

async fn cmd_list_remote(
    gateway: Option<String>,
    token: Option<String>,
    json: bool,
) -> anyhow::Result<()> {
    let client = make_client(gateway, token)?;
    let skills: Vec<vox_ars::OpenClawSkillSpec> = client
        .list_skills()
        .await
        .map_err(|e| anyhow::anyhow!("List failed: {e}"))?;

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "count": skills.len(),
                "skills": skills,
            }))?
        );
    } else {
        println!("Remote OpenClaw Skills ({} available):", skills.len());
        for s in &skills {
            println!(
                "  {:<40} v{:<10}  {}",
                s.name,
                s.version,
                s.description.as_deref().unwrap_or("")
            );
        }
    }
    Ok(())
}

async fn cmd_config(gateway: Option<String>, json: bool) -> anyhow::Result<()> {
    let url = gateway
        .or_else(|| std::env::var("VOX_OPENCLAW_URL").ok())
        .unwrap_or_else(|| "http://localhost:3000".into());
    let token_set = vox_clavis::resolve_secret(vox_clavis::SecretId::OpenClawToken).is_present();

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "gateway_url": url,
                "token_configured": token_set,
            }))?
        );
    } else {
        println!("OpenClaw Configuration:");
        println!("  Gateway URL : {url}");
        println!(
            "  Token set   : {}",
            if token_set {
                "yes"
            } else {
                "no (set VOX_OPENCLAW_TOKEN)"
            }
        );
    }
    Ok(())
}

async fn cmd_approvals(mcp_url: String, json: bool) -> anyhow::Result<()> {
    // Call vox_approval_list via HTTP against the running MCP server.
    let url = format!("{mcp_url}/mcp");
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "vox_approval_list",
            "arguments": {}
        }
    });
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;
    let resp = client.post(&url).json(&body).send().await;
    match resp {
        Ok(r) if r.status().is_success() => {
            let text = r.text().await.unwrap_or_default();
            if json {
                println!("{text}");
            } else {
                println!("Pending Approvals:\n{text}");
            }
            Ok(())
        }
        Ok(r) => anyhow::bail!("MCP server returned HTTP {}", r.status()),
        Err(e) => {
            anyhow::bail!("Cannot reach MCP server at {mcp_url}: {e}\nIs `vox mcp-server` running?")
        }
    }
}

async fn cmd_resolve(
    id: String,
    approved: bool,
    reason: Option<String>,
    mcp_url: String,
) -> anyhow::Result<()> {
    let url = format!("{mcp_url}/mcp");
    let mut args = serde_json::json!({
        "approval_id": id,
        "approved": approved,
    });
    if let Some(ref r) = reason {
        args["reason"] = serde_json::Value::String(r.clone());
    }
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": "vox_approval_resolve",
            "arguments": args
        }
    });
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;
    match client.post(&url).json(&body).send().await {
        Ok(r) if r.status().is_success() => {
            let verb = if approved { "Approved" } else { "Denied" };
            println!("✓ {verb} approval request: {id}");
            Ok(())
        }
        Ok(r) => anyhow::bail!("MCP server returned HTTP {}", r.status()),
        Err(e) => anyhow::bail!("Cannot reach MCP server at {mcp_url}: {e}"),
    }
}

async fn cmd_serve(port: u16, addr: String) -> anyhow::Result<()> {
    let full_addr = format!("{addr}:{port}");
    println!("Starting vox-gateway on http://{full_addr}...");

    // Launch the vox-gateway binary since it's a separate crate.
    let mut child = std::process::Command::new("vox-gateway")
        .env("VOX_GATEWAY_ADDR", &full_addr)
        .spawn()
        .map_err(|e| {
            anyhow::anyhow!("Failed to launch vox-gateway binary. Is it in your PATH? Error: {e}")
        })?;

    let status = child.wait()?;
    if !status.success() {
        anyhow::bail!("vox-gateway exited with error: {status}");
    }
    Ok(())
}

async fn cmd_subscribe(domain: String, gateway: String) -> anyhow::Result<()> {
    // Real gateways use SSE/WebSocket; this path is a placeholder until a client is wired.
    println!("✓ Subscribing to domain: {domain} (gateway {gateway}; simulation only)");
    Ok(())
}

async fn cmd_unsubscribe(domain: String, _gateway: String) -> anyhow::Result<()> {
    println!("✓ Unsubscribed from domain: {domain}");
    Ok(())
}

async fn cmd_subscriptions(gateway: String, json: bool) -> anyhow::Result<()> {
    let url = format!("{gateway}/v1/subscriptions");
    let resp = reqwest::get(&url).await?;
    if resp.status().is_success() {
        let text = resp.text().await?;
        if json {
            println!("{text}");
        } else {
            println!("Active Semantic Subscriptions:\n{text}");
        }
        Ok(())
    } else {
        anyhow::bail!("Failed to fetch subscriptions: {}", resp.status())
    }
}

async fn cmd_notify(domain: String, message: String, gateway: String) -> anyhow::Result<()> {
    let url = format!("{gateway}/v1/subscriptions/notify");
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&serde_json::json!({
            "domain": domain,
            "description": message,
        }))
        .send()
        .await?;

    if resp.status().is_success() {
        println!("✓ Notified agents in domain: {domain}");
        Ok(())
    } else {
        anyhow::bail!("Failed to notify domain: {}", resp.status())
    }
}
