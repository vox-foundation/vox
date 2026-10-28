//! Optional telemetry upload queue (`vox telemetry …`). See ADR 023 and
//! `docs/src/architecture/telemetry-remote-sink-spec.md`.

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};
use clap::Subcommand;
use vox_telemetry::config::{ConsentState, is_remote_allowed, remote_consent, set_remote_consent};

/// Actions for `vox telemetry consent`.
#[derive(Subcommand, Clone)]
pub enum ConsentAction {
    /// Opt in to remote telemetry upload (persists to ~/.config/vox/remote-consent).
    Grant,
    /// Opt out of remote telemetry upload (persists to ~/.config/vox/remote-consent).
    Deny,
    /// Show current consent state and whether remote upload is allowed.
    Status,
}

/// Actions for `vox telemetry intelligence` (`INTEL-03`).
#[derive(Subcommand, Clone)]
pub enum IntelligenceAction {
    /// Show cached CommunityIntelligenceBundleV1 status, Ed25519 signature health, and opt-out state.
    Status,
    /// Verify cached bundle or report offline fallback readiness (non-blocking).
    Sync,
    /// Print the cached CommunityIntelligenceBundleV1 JSON (or schema summary if no cache file exists).
    Inspect,
}

use crate::telemetry_spool;
fn resolve_spool(spool: Option<PathBuf>) -> PathBuf {
    spool.unwrap_or_else(telemetry_spool::spool_root)
}

/// Subcommands for `vox telemetry`.
#[derive(Subcommand)]
pub enum TelemetryCmd {
    /// Print the effective telemetry configuration and recorder health.
    ///
    /// Shows: master switch state, per-category flags, recorder registration,
    /// spool path, and remote consent state. Exit 0 if telemetry is active;
    /// exit 1 if master switch is off.
    Doctor,
    /// Manage remote telemetry upload consent.
    Consent {
        #[command(subcommand)]
        action: ConsentAction,
    },
    /// Show spool path, pending count, and whether upload URL/token resolve (redacted).
    Status {
        /// Override spool root (default: cwd `.vox/telemetry-upload-queue` or `VOX_TELEMETRY_SPOOL_DIR`).
        #[arg(long)]
        spool: Option<PathBuf>,
    },
    /// Print pending payloads as JSON Lines to stdout (does not dequeue).
    Export {
        #[arg(long)]
        spool: Option<PathBuf>,
        /// Write JSONL to this file instead of stdout.
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Render the exact redacted OTLP/HTTP JSON batch or tabular summary that would be uploaded (offline, no network).
    Preview {
        #[arg(long)]
        spool: Option<PathBuf>,
        /// Maximum number of pending files to include in the preview batch.
        #[arg(long, default_value_t = 50)]
        limit: usize,
        /// Output format: `table` or `json`.
        #[arg(long, default_value = "json")]
        format: String,
    },
    /// Append one JSON object file to the pending queue.
    Enqueue {
        #[arg(long)]
        spool: Option<PathBuf>,
        /// Path to a JSON file (single object).
        #[arg(long)]
        json: PathBuf,
    },
    /// POST each pending JSON to the configured ingest URL; delete file on HTTP 2xx.
    Upload {
        #[arg(long)]
        spool: Option<PathBuf>,
        /// Log actions only; no network or deletes.
        #[arg(long)]
        dry_run: bool,
        /// Override ingest URL (else `VOX_TELEMETRY_UPLOAD_URL` via Clavis).
        #[arg(long)]
        url: Option<String>,
    },
    /// Inspect or verify the Ed25519-signed CommunityIntelligenceBundleV1 cache (`INTEL-03`).
    Intelligence {
        #[command(subcommand)]
        action: IntelligenceAction,
    },
}

pub async fn run(cmd: TelemetryCmd) -> Result<()> {
    match cmd {
        TelemetryCmd::Doctor => {
            let cfg = vox_telemetry::TelemetryConfig::from_env();
            let recorder_active = vox_telemetry::global_recorder().is_some();
            let master = vox_telemetry::is_master_enabled();
            let org_disabled = vox_telemetry::org_policy_disabled();
            let consent = remote_consent();
            let remote_ok = is_remote_allowed();

            println!("vox telemetry doctor");
            println!("─────────────────────────────────────");
            println!("org_policy_disabled:   {org_disabled}");
            println!("master_enabled:        {master}");
            println!("enabled:               {}", cfg.enabled);
            println!("remote_upload:         {}", cfg.remote_upload);
            println!("debug_to_stderr:       {}", cfg.debug_to_stderr);
            println!("─────────────────────────────────────");
            println!("categories:");
            println!("  research_metrics:    {}", cfg.research_metrics);
            println!("  model_calls:         {}", cfg.model_calls);
            println!("  agent_orchestration: {}", cfg.agent_orchestration);
            println!("  build:               {}", cfg.build);
            println!("  errors:              {}", cfg.errors);
            println!("─────────────────────────────────────");
            println!("recorder_registered:   {recorder_active}");
            println!(
                "remote_consent:        {}",
                match consent {
                    ConsentState::Granted => "granted",
                    ConsentState::Denied => "denied",
                    ConsentState::Unset => "unset (run `vox telemetry consent grant` to opt in)",
                }
            );
            println!("remote_upload_allowed: {remote_ok}");

            let spool_root = telemetry_spool::spool_root();
            println!("spool_root:            {}", spool_root.display());

            println!("─────────────────────────────────────");
            if !master {
                println!("STATUS: DISABLED  (VOX_TELEMETRY=off or equivalent)");
                std::process::exit(1);
            } else if !recorder_active {
                println!("STATUS: WARNING   (no recorder registered — events are no-ops)");
            } else if cfg.debug_to_stderr {
                println!("STATUS: DEBUG     (VOX_TELEMETRY=debug — events printed to stderr)");
            } else {
                println!("STATUS: OK");
            }
        }
        TelemetryCmd::Consent { action } => match action {
            ConsentAction::Grant => {
                set_remote_consent(ConsentState::Granted);
                println!("Remote telemetry upload: ENABLED");
                println!(
                    "Events will be uploaded to the Vox analytics endpoint on the next `vox telemetry upload` run."
                );
                println!("To opt out at any time: vox telemetry consent deny");
            }
            ConsentAction::Deny => {
                set_remote_consent(ConsentState::Denied);
                println!("Remote telemetry upload: DISABLED");
                println!("No events will be uploaded. Local spool is unaffected.");
                println!("To opt in at any time: vox telemetry consent grant");
            }
            ConsentAction::Status => {
                let consent = remote_consent();
                let allowed = is_remote_allowed();
                println!(
                    "consent: {}",
                    match consent {
                        ConsentState::Granted => "granted",
                        ConsentState::Denied => "denied",
                        ConsentState::Unset => "unset",
                    }
                );
                println!("remote_upload_allowed: {allowed}");
                if matches!(consent, ConsentState::Unset) {
                    println!("Hint: run `vox telemetry consent grant` to enable remote upload,");
                    println!("       or  `vox telemetry consent deny` to opt out permanently.");
                }
            }
        },
        TelemetryCmd::Status { spool } => {
            let root = resolve_spool(spool);
            let pending_n = telemetry_spool::pending_count(&root);
            let url_r = vox_secrets::resolve_secret(vox_secrets::SecretId::VoxTelemetryUploadUrl);
            let tok_r = vox_secrets::resolve_secret(vox_secrets::SecretId::VoxTelemetryUploadToken);
            println!("spool_root: {}", root.display());
            println!("pending_files: {pending_n}");
            println!(
                "upload_url: {} ({})",
                url_r.redacted(),
                if url_r.is_present() {
                    "resolved"
                } else {
                    "missing"
                }
            );
            println!(
                "upload_token: {} ({})",
                tok_r.redacted(),
                if tok_r.is_present() {
                    "resolved"
                } else {
                    "missing"
                }
            );
        }
        TelemetryCmd::Export { spool, out } => {
            let root = resolve_spool(spool);
            let n = if let Some(p) = out {
                let mut f =
                    fs::File::create(&p).with_context(|| format!("create {}", p.display()))?;
                telemetry_spool::export_jsonl(&root, &mut f)?
            } else {
                let mut stdout = std::io::stdout().lock();
                telemetry_spool::export_jsonl(&root, &mut stdout)?
            };
            eprintln!("exported {n} record(s)");
        }
        TelemetryCmd::Preview {
            spool,
            limit,
            format,
        } => {
            let root = resolve_spool(spool);
            if format.eq_ignore_ascii_case("table") {
                let paths = telemetry_spool::list_pending(&root)?;
                println!("spool_root:    {}", root.display());
                println!("pending_total: {}", paths.len());
                println!("showing:       {}", paths.len().min(limit));
                for p in paths.into_iter().take(limit) {
                    let val = telemetry_spool::read_payload(&p)?;
                    println!(
                        "{}  {}",
                        p.file_name().and_then(|s| s.to_str()).unwrap_or("unknown"),
                        serde_json::to_string(&val).unwrap_or_default()
                    );
                }
            } else {
                let mut stdout = std::io::stdout().lock();
                let n = telemetry_spool::preview_pending(&root, limit, &mut stdout)?;
                eprintln!("previewed {n} record(s) (no network calls made; spool unchanged)");
            }
        }
        TelemetryCmd::Enqueue { spool, json } => {
            let root = resolve_spool(spool);
            let raw =
                fs::read_to_string(&json).with_context(|| format!("read {}", json.display()))?;
            let value: serde_json::Value =
                serde_json::from_str(&raw).context("parse --json file as JSON")?;
            let path = telemetry_spool::enqueue(&root, &value)?;
            println!("{}", path.display());
        }
        TelemetryCmd::Upload {
            spool,
            dry_run,
            url,
        } => {
            let root = resolve_spool(spool);
            let url_str = if let Some(u) = url.filter(|s| !s.trim().is_empty()) {
                u
            } else {
                let r = vox_secrets::resolve_secret(vox_secrets::SecretId::VoxTelemetryUploadUrl);
                r.expose()
                    .map(str::to_owned)
                    .ok_or_else(|| anyhow!("{}", r.remediation))?
            };
            let bearer =
                vox_secrets::resolve_secret(vox_secrets::SecretId::VoxTelemetryUploadToken)
                    .expose()
                    .map(str::to_owned);
            let (ok, fail) =
                telemetry_spool::upload_pending(&root, url_str.trim(), bearer.as_deref(), dry_run)
                    .await?;
            if fail > 0 {
                anyhow::bail!("upload: {ok} ok, {fail} failed (see logs)");
            }
            eprintln!("upload: {ok} file(s) processed successfully");
        }
        TelemetryCmd::Intelligence { action } => {
            use vox_orchestrator::models::intelligence_bundle::{
                default_bundle_cache_path, is_community_intelligence_enabled,
                load_cached_bundle_if_valid,
            };
            let enabled = is_community_intelligence_enabled();
            let cache_path = default_bundle_cache_path();
            match action {
                IntelligenceAction::Status | IntelligenceAction::Sync => {
                    println!("community_intelligence_enabled: {enabled}");
                    println!("cache_path:                     {}", cache_path.display());
                    if let Some(bundle) = load_cached_bundle_if_valid(Some(&cache_path)) {
                        println!("bundle_status:                  VERIFIED (Ed25519 + k>=20)");
                        println!("bundle_version:                 {}", bundle.bundle_version);
                        println!(
                            "model_routing_priors_count:     {}",
                            bundle.model_routing_priors.len()
                        );
                    } else {
                        println!(
                            "bundle_status:                  OFFLINE_FALLBACK (static local priors active)"
                        );
                    }
                }
                IntelligenceAction::Inspect => {
                    if let Some(bundle) = load_cached_bundle_if_valid(Some(&cache_path)) {
                        println!("{}", serde_json::to_string_pretty(&bundle)?);
                    } else {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&serde_json::json!({
                                "status": "offline_fallback",
                                "enabled": enabled,
                                "cache_path": cache_path,
                                "k_anonymity_floor": 20,
                                "min_events_floor": 50,
                                "laplace_epsilon": 1.0,
                            }))?
                        );
                    }
                }
            }
        }
    }
    Ok(())
}
