use anyhow::Result;
use clap::{Subcommand, ValueEnum};

fn redact_value(value: &str) -> String {
    if value.chars().count() > 6 {
        let head: String = value.chars().take(4).collect();
        let tail: String = value
            .chars()
            .rev()
            .take(2)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        format!("{head}…{tail} (redacted)")
    } else {
        "***".to_string()
    }
}

fn local_inference_allows_no_cloud_key() -> bool {
    match std::env::var("VOX_INFERENCE_PROFILE")
        .ok()
        .map(|s| s.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("cloud_openai_compatible") | Some("cloud") => false,
        Some("mobile_litert") | Some("litert") => false,
        Some("mobile_coreml") | Some("coreml") => false,
        Some("lan_gateway") | Some("lan") => true,
        Some("desktop_ollama") | Some("ollama") | None => true,
        _ => true,
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum DoctorModeArg {
    Auto,
    Local,
    Cloud,
}

impl From<DoctorModeArg> for vox_clavis::RequirementMode {
    fn from(value: DoctorModeArg) -> Self {
        match value {
            DoctorModeArg::Auto => Self::Auto,
            DoctorModeArg::Local => Self::Local,
            DoctorModeArg::Cloud => Self::Cloud,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum BundleArg {
    MinimalLocalDev,
    MinimalCloudDev,
    GpuCloud,
    PublishReview,
}

impl From<BundleArg> for vox_clavis::SecretBundle {
    fn from(value: BundleArg) -> Self {
        match value {
            BundleArg::MinimalLocalDev => Self::MinimalLocalDev,
            BundleArg::MinimalCloudDev => Self::MinimalCloudDev,
            BundleArg::GpuCloud => Self::GpuCloud,
            BundleArg::PublishReview => Self::PublishReview,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum WorkflowArg {
    Chat,
    Mcp,
    Publish,
    Review,
    DbRemote,
    MensMesh,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ProfileArg {
    Dev,
    Ci,
    Mobile,
    Prod,
}

impl From<WorkflowArg> for vox_clavis::Workflow {
    fn from(value: WorkflowArg) -> Self {
        match value {
            WorkflowArg::Chat => Self::Chat,
            WorkflowArg::Mcp => Self::Mcp,
            WorkflowArg::Publish => Self::Publish,
            WorkflowArg::Review => Self::Review,
            WorkflowArg::DbRemote => Self::DbRemote,
            WorkflowArg::MensMesh => Self::MensMesh,
        }
    }
}

impl From<ProfileArg> for vox_clavis::Profile {
    fn from(value: ProfileArg) -> Self {
        match value {
            ProfileArg::Dev => Self::Dev,
            ProfileArg::Ci => Self::Ci,
            ProfileArg::Mobile => Self::Mobile,
            ProfileArg::Prod => Self::Prod,
        }
    }
}

#[derive(Subcommand, Debug)]
pub enum ClavisCmd {
    /// Show secret readiness for a workflow (credentials / env resolution).
    #[command(name = "status", visible_alias = "doctor")]
    Status {
        #[arg(long, value_enum, default_value_t = WorkflowArg::Chat)]
        workflow: WorkflowArg,
        #[arg(long, value_enum, default_value_t = ProfileArg::Dev)]
        profile: ProfileArg,
        #[arg(long, value_enum, default_value_t = DoctorModeArg::Auto)]
        mode: DoctorModeArg,
        #[arg(long, value_enum)]
        bundle: Option<BundleArg>,
    },
    /// Store a registry token in ~/.vox/auth.json (compat mode).
    Set {
        registry: String,
        token: String,
        #[arg(long)]
        username: Option<String>,
    },
    /// Read a registry token from resolution sources.
    Get { registry: String },
    /// Show backend mode and current availability state.
    BackendStatus,
    /// Migrate plaintext `auth.json` tokens into secure local store.
    MigrateAuthStore,
}

pub async fn run(cmd: ClavisCmd) -> Result<()> {
    match cmd {
        ClavisCmd::Status {
            workflow,
            profile,
            mode,
            bundle,
        } => run_doctor(workflow, profile, mode, bundle).await,
        ClavisCmd::Set {
            registry,
            token,
            username,
        } => {
            let path = vox_clavis::set_registry_token(&registry, &token, username)
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("Stored token for `{registry}` in {}", path.display());
            Ok(())
        }
        ClavisCmd::Get { registry } => {
            match vox_clavis::get_registry_token(&registry) {
                Some(token) => {
                    println!("{registry}: {}", redact_value(&token));
                }
                None => println!("{registry}: (missing)"),
            }
            Ok(())
        }
        ClavisCmd::BackendStatus => {
            let mode = vox_clavis::BackendMode::from_env();
            println!("clavis backend mode: {mode:?}");
            for spec in vox_clavis::all_specs() {
                let res = vox_clavis::resolve_secret(spec.id);
                if matches!(res.status, vox_clavis::ResolutionStatus::BackendUnavailable) {
                    println!(
                        "backend status: unavailable ({})",
                        res.detail.unwrap_or_else(|| "no detail".to_string())
                    );
                    return Ok(());
                }
            }
            println!("backend status: available or env-only fallback");
            Ok(())
        }
        ClavisCmd::MigrateAuthStore => {
            let moved = vox_clavis::migrate_auth_store_to_secure_store()
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("migrated {moved} auth entries to secure store");
            Ok(())
        }
    }
}

async fn run_doctor(
    workflow: WorkflowArg,
    profile: ProfileArg,
    mode: DoctorModeArg,
    bundle: Option<BundleArg>,
) -> Result<()> {
    let wf = vox_clavis::Workflow::from(workflow);
    let profile = vox_clavis::Profile::from(profile);
    let resolved_mode = match mode {
        DoctorModeArg::Auto if local_inference_allows_no_cloud_key() => DoctorModeArg::Local,
        DoctorModeArg::Auto => DoctorModeArg::Cloud,
        m => m,
    };
    let requirements = if let Some(bundle) = bundle {
        vox_clavis::requirements_for_bundle(vox_clavis::SecretBundle::from(bundle))
    } else {
        vox_clavis::requirements_for_profile_mode(
            wf,
            profile,
            vox_clavis::RequirementMode::from(resolved_mode),
        )
    };
    println!("clavis doctor ({workflow:?}, {profile:?})");
    println!("active_mode: {resolved_mode:?}");
    if let Some(bundle) = bundle {
        println!("bundle: {bundle:?}");
    }

    println!("blocking_requirements:");
    for req in requirements.blocking {
        match req {
            vox_clavis::RequirementSet::AllOf(ids) => {
                let mut ok = true;
                for id in ids {
                    let resolved = vox_clavis::resolve_secret(*id);
                    println!(
                        "  - {:?}: {:?} via {:?} {}",
                        id,
                        resolved.status,
                        resolved.source,
                        resolved.redacted()
                    );
                    if let Some(detail) = &resolved.detail {
                        println!("    detail: {detail}");
                    }
                    if !resolved.is_present() {
                        ok = false;
                        println!("    remediation: {}", resolved.remediation);
                    }
                }
                println!(
                    "  => group ALL_OF status: {}",
                    if ok { "satisfied" } else { "missing" }
                );
            }
            vox_clavis::RequirementSet::AnyOf(ids) => {
                let mut ok = false;
                let mut any_present: Option<vox_clavis::SecretId> = None;
                for id in ids {
                    let resolved = vox_clavis::resolve_secret(*id);
                    println!(
                        "  - {:?}: {:?} via {:?} {}",
                        id,
                        resolved.status,
                        resolved.source,
                        resolved.redacted()
                    );
                    if resolved.is_present() {
                        ok = true;
                        any_present = Some(*id);
                    } else if let Some(detail) = &resolved.detail {
                        println!("    detail: {detail}");
                    }
                    if matches!(
                        resolved.status,
                        vox_clavis::ResolutionStatus::DeprecatedAliasUsed
                    ) {
                        println!(
                            "    warning: deprecated alias in use; migrate to `{}`",
                            id.spec().canonical_env
                        );
                    }
                }
                println!(
                    "  => group ANY_OF status: {}{}",
                    if ok { "satisfied" } else { "missing" },
                    any_present
                        .map(|id| format!(" (selected {:?})", id))
                        .unwrap_or_default()
                );
                if !ok {
                    println!(
                        "    remediation: set at least one key in this group or use local profile"
                    );
                }
            }
        }
    }

    if !requirements.optional.is_empty() {
        println!("optional_capabilities:");
        for id in requirements.optional {
            let resolved = vox_clavis::resolve_secret(id);
            println!(
                "  - {:?}: {:?} via {:?} {}",
                id,
                resolved.status,
                resolved.source,
                resolved.redacted()
            );
            if matches!(
                resolved.status,
                vox_clavis::ResolutionStatus::DeprecatedAliasUsed
            ) {
                println!(
                    "    warning: deprecated alias in use; migrate to `{}`",
                    id.spec().canonical_env
                );
            }
        }
    }

    let rf = vox_config::rollout_flag_snapshot();
    println!(
        "rollout_flags: lineage_persist={} workflow_journal_codex_persist={} db_circuit_breaker_env={} db_sync_remote_it_gate={} db_embedded_replica_it_gate={}",
        rf.orchestration_lineage_persist,
        rf.workflow_journal_codex_persist,
        rf.db_circuit_breaker_env,
        rf.db_sync_remote_integration_gate,
        rf.db_embedded_replica_integration_gate
    );
    if !rf.workflow_journal_codex_persist {
        println!(
            "warning: VOX_WORKFLOW_JOURNAL_CODEX_OFF disables Codex workflow journal append (durable replay still depends on workflow_activity_log)"
        );
    }
    if rf.db_circuit_breaker_env {
        println!(
            "warning: VOX_DB_CIRCUIT_BREAKER may gate workflow durability writes under DB stress"
        );
    }
    Ok(())
}
