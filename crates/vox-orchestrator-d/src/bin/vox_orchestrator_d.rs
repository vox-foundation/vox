//! Long-lived orchestrator owner: TCP JSON-line RPC (ADR 022 Phase B).
//!
//! Requires **`VOX_ORCHESTRATOR_DAEMON_SOCKET`**: TCP bind (`127.0.0.1:9745`) or **`stdio`** / **`-`** for line JSON on stdin/stdout.

use anyhow::Context as _;
use std::path::PathBuf;
use std::sync::Arc;

use vox_orchestrator::{OrchestratorConfig, orch_daemon};
use vox_orchestrator_d::boot_plan::{self, DependencyMode};
use vox_orchestrator_d::daemon_steps::{self, DaemonBootCtx};
use vox_orchestrator_mcp::profiles::BootReport;

/// Well-known file the daemon writes its auth token to at startup (T0.2), read
/// back by [`vox_orchestrator::orch_daemon::OrchDaemonClient::new`] to
/// auto-resolve a token for callers that don't already know one.
fn token_file_path() -> PathBuf {
    vox_config::paths::user_home_dir()
        .join(".vox")
        .join("run")
        .join("orchestrator-daemon.token")
}

/// Resolve this daemon's auth token: use `VOX_ORCHESTRATOR_DAEMON_TOKEN` if an
/// operator (or an explicit spawner like the GUI's `PersistentDaemon`) set it,
/// else generate a fresh random token. Pure — does not touch the token file;
/// see [`persist_daemon_token_file`] for that (only called for a daemon that
/// actually binds the shared TCP socket — see its doc comment for why).
fn resolve_daemon_token(explicit_env_token: Option<String>) -> String {
    match explicit_env_token {
        Some(t) if !t.is_empty() => t,
        _ => uuid::Uuid::new_v4().to_string(),
    }
}

/// (Over)write the well-known token file so [`OrchDaemonClient::new`]
/// callers can auto-resolve this daemon's token.
///
/// Callers MUST only invoke this for a daemon that is about to bind the
/// shared TCP socket, never for a `stdio`-transport daemon. The safety
/// argument in the original version of this comment — "a fresh daemon
/// process always gets a fresh token file: since `TcpListener::bind` fails
/// if another daemon already holds the port, there is never more than one
/// live daemon per bind address" — is true for TCP but does **not** hold for
/// `stdio`: many `vox-orchestrator-d --socket stdio` processes can (and
/// routinely did, e.g. every `vox rollback` invocation via
/// `daemon_ipc::dispatch::call_daemon`) run concurrently with each other and
/// with an unrelated long-lived TCP daemon a GUI session is using — each one
/// unconditionally overwriting this file rotated the TCP daemon's token out
/// from under it, a token-rotation race with no port conflict to prevent it.
/// A `stdio` daemon's token is never read from this file by anyone (its
/// stdio server doesn't even accept a token argument — see
/// [`orch_daemon::run_stdio_server_with_extra`]), so it has nothing to gain
/// and everything to lose by writing here.
///
/// [`OrchDaemonClient::new`]: vox_orchestrator::orch_daemon::OrchDaemonClient::new
fn persist_daemon_token_file(token: &str) -> anyhow::Result<()> {
    let path = token_file_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("creating daemon token dir {}", dir.display()))?;
    }
    std::fs::write(&path, token)
        .with_context(|| format!("writing daemon token file {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&path, perms)
            .with_context(|| format!("setting owner-only perms on {}", path.display()))?;
    }
    // On Windows the file is written under the user's profile directory
    // (`%USERPROFILE%\.vox\run\...`), which is not world-readable by
    // convention; explicit Windows ACL hardening is a possible follow-up, not
    // required now.

    Ok(())
}

/// Refuse a non-loopback TCP bind unless the operator explicitly set
/// `VOX_ORCHESTRATOR_DAEMON_TOKEN` themselves (T0.2). The daemon always has
/// *some* token (auto-generated when unset), but relying on the local-only
/// token *file* to protect a *remote*-reachable socket defeats the purpose: a
/// remote attacker can't read the local file, but a legitimate remote caller
/// also has no way to discover an auto-generated token. Conservative rule:
/// non-loopback binds require the operator to have explicitly configured
/// auth.
fn refuse_non_loopback_without_explicit_token(
    bind: &str,
    explicit_env_token_was_set: bool,
) -> anyhow::Result<()> {
    if !orch_daemon::is_loopback_bind_addr(bind) && !explicit_env_token_was_set {
        anyhow::bail!(
            "refusing to bind vox-orchestrator-d to non-loopback address '{bind}': set VOX_ORCHESTRATOR_DAEMON_TOKEN explicitly before binding to a non-loopback address (an auto-generated token is only meaningfully protective for loopback-local callers)"
        );
    }
    Ok(())
}

fn load_config() -> OrchestratorConfig {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut candidates = Vec::new();
    if let Some(root) = vox_repository::find_project_manifest_root(&cwd) {
        candidates.push(root.join("Vox.toml"));
    }
    candidates.push(PathBuf::from("Vox.toml"));

    let mut config = OrchestratorConfig::default();
    let mut loaded = false;
    for toml_path in candidates {
        if toml_path.is_file() {
            match OrchestratorConfig::load_from_toml(&toml_path) {
                Ok(cfg) => {
                    tracing::info!(path = %toml_path.display(), "loaded orchestrator config from Vox.toml");
                    config = cfg;
                    loaded = true;
                    break;
                }
                Err(e) => tracing::warn!(
                    path = %toml_path.display(),
                    "failed to load Vox.toml: {e}, trying next candidate"
                ),
            }
        }
    }
    if !loaded {
        tracing::info!("no readable Vox.toml found, using defaults");
    }
    config.merge_env_overrides();
    config
}

/// Default worker stack for deep `vox_chat_message` / MCP ExtraDispatch futures.
/// The std/tokio default (~2 MiB) aborts with stack overflow on sync chat turns,
/// which the TCP client sees as an empty `orch.tool_call` frame. Override with
/// `RUST_MIN_STACK` (bytes) when spawning if a larger value is required.
const DEFAULT_ORCH_WORKER_STACK: usize = 32 * 1024 * 1024;

const HELP_TEXT: &str = "\
vox-orchestrator-d: long-lived orchestrator owner (TCP or stdio JSON-line RPC)

USAGE:
    vox-orchestrator-d

Requires VOX_ORCHESTRATOR_DAEMON_SOCKET to be set:
    127.0.0.1:9745   bind a loopback TCP address (or any \"host:port\")
    stdio | -        one JSON-line request in, one JSON-line response out, on stdin/stdout

This binary is normally spawned by another vox process (the CLI, the GUI, or
`vox mcp`), not invoked directly.";

/// `--help`/`-h` before anything else: this binary reads
/// `VOX_ORCHESTRATOR_DAEMON_SOCKET` unconditionally in `async_main`, so
/// without this check `vox-orchestrator-d --help` printed a config error
/// ("VOX_ORCHESTRATOR_DAEMON_SOCKET is required") instead of usage.
fn wants_help(args: &[String]) -> bool {
    args.iter().any(|a| a == "--help" || a == "-h")
}

fn all_required_failed_message(profile_id: &str, report: &BootReport) -> String {
    let failures: Vec<String> = report
        .failed
        .iter()
        .map(|(s, e)| format!("{}: {e}", s.as_str()))
        .collect();
    format!(
        "every subsystem required by profile `{profile_id}` failed to boot ({})",
        failures.join("; ")
    )
}

fn main() -> anyhow::Result<()> {
    if wants_help(&std::env::args().collect::<Vec<_>>()) {
        println!("{HELP_TEXT}");
        return Ok(());
    }
    let stack = std::env::var("RUST_MIN_STACK")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .filter(|&n| n >= 2 * 1024 * 1024)
        .unwrap_or(DEFAULT_ORCH_WORKER_STACK);
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(stack)
        .build()
        .context("building tokio runtime for vox-orchestrator-d")?;
    rt.block_on(async_main())
}

fn resolve_daemon_profile() -> anyhow::Result<vox_orchestrator_mcp::profiles::ActiveProfile> {
    vox_orchestrator_mcp::profiles::ActiveProfile::from_env()
        .map_err(|e| anyhow::anyhow!("{e} (VOX_PROFILE)"))
}

async fn async_main() -> anyhow::Result<()> {
    vox_foundation::tracing::try_init_from_default_env();
    let active_profile = resolve_daemon_profile()?;
    tracing::info!(profile = %active_profile.id_string(), "capability profile resolved");

    let bind_raw = vox_secrets::resolve_secret(vox_secrets::SecretId::VoxOrchestratorDaemonSocket)
        .expose()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "VOX_ORCHESTRATOR_DAEMON_SOCKET is required (e.g. 127.0.0.1:9745 or stdio)"
            )
        })?
        .to_string();
    let is_stdio = orch_daemon::is_stdio_transport(&bind_raw);

    // Daemon auth token (T0.2): explicit env wins (lets a spawner like the
    // GUI's PersistentDaemon inject a token it already knows, avoiding a race
    // with reading the token file before this daemon has written it); else
    // generate a fresh random token. Persisted to the shared token file ONLY
    // for a daemon that actually binds the TCP socket — see
    // `persist_daemon_token_file`'s doc comment for why a `stdio`-transport
    // daemon must never write there (it would rotate an unrelated TCP
    // daemon's token out from under it).
    let explicit_env_token =
        vox_secrets::resolve_secret(vox_secrets::SecretId::VoxOrchestratorDaemonToken)
            .expose()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
    let explicit_env_token_was_set = explicit_env_token.is_some();
    let daemon_token: Arc<str> = resolve_daemon_token(explicit_env_token).into();
    if !is_stdio {
        persist_daemon_token_file(&daemon_token)?;
    }

    let profile = active_profile; // resolved by plan 02 Task 8 (`resolve_daemon_profile`)
    let mut ctx = DaemonBootCtx::new(
        load_config(),
        None,
        profile.clone(),
        daemon_steps::plugins_install_dir(std::env::var("VOX_PLUGINS_DIR").ok()),
    );
    let outcome = boot_plan::run_boot_plan(
        daemon_steps::DAEMON_STEPS,
        &mut ctx,
        &profile.subsystems,
        DependencyMode::for_profile(&profile),
    )
    .await?;
    tracing::info!(
        profile = %profile.id_string(),
        booted = ?outcome.report.booted,
        failed = ?outcome.report.failed,
        skipped = ?outcome.skipped,
        "daemon boot plan finished"
    );
    if boot_plan::every_required_failed(&boot_plan::profile_roots(&profile), &outcome.report) {
        anyhow::bail!(
            "{}",
            all_required_failed_message(&profile.id_string(), &outcome.report)
        );
    }
    let state = ctx
        .state
        .take()
        .context("server_state boot step did not run")?;
    state.set_boot_report(outcome.report);
    let orch = ctx.orch.clone();
    let repository_id = ctx.repository_id.clone();

    // Serve orch.tool_call / orch.resolve_approval / orch.list_pending_approvals
    // against this same ServerState so the GUI runs tools + resolves HITL
    // approvals through the one shared orchestrator (B5 path-c, B3 cross-process).
    let extra: Option<Arc<dyn orch_daemon::ExtraDispatch>> = Some(Arc::new(
        vox_orchestrator_mcp::daemon_extra::McpExtraDispatch::new(state.clone()),
    ));

    if let Err(e) = vox_orchestrator_mcp::http_gateway::spawn_http_gateway_if_enabled(state) {
        if e.downcast_ref::<vox_orchestrator_mcp::http_gateway::bind_policy::BindPolicyError>()
            .is_some()
        {
            return Err(e.context("MCP HTTP gateway bind policy refused to start"));
        }
        tracing::error!(error = %e, "Failed to spawn HTTP gateway");
    }

    if orch_daemon::is_stdio_transport(&bind_raw) {
        return orch_daemon::run_stdio_server_with_extra(repository_id, orch, extra).await;
    }

    let bind = orch_daemon::normalize_tcp_bind_addr(&bind_raw);
    if bind.is_empty() {
        anyhow::bail!("VOX_ORCHESTRATOR_DAEMON_SOCKET is empty after normalization");
    }
    refuse_non_loopback_without_explicit_token(&bind, explicit_env_token_was_set)?;

    orch_daemon::run_tcp_server_with_extra(&bind, repository_id, orch, extra, Some(daemon_token))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_required_failed_message_names_profile_and_failures() {
        let mut report = vox_orchestrator_mcp::profiles::BootReport::default();
        report.failed.insert(
            vox_orchestrator_mcp::profiles::Subsystem::Db,
            "workspace journey store unavailable".into(),
        );
        let msg = all_required_failed_message("knowledge", &report);
        assert!(msg.contains("profile `knowledge`"), "{msg}");
        assert!(
            msg.contains("db: workspace journey store unavailable"),
            "{msg}"
        );
    }

    #[test]
    fn wants_help_detects_long_and_short_flags() {
        assert!(wants_help(&["vox-orchestrator-d".into(), "--help".into()]));
        assert!(wants_help(&["vox-orchestrator-d".into(), "-h".into()]));
        assert!(!wants_help(&["vox-orchestrator-d".into()]));
    }

    #[test]
    fn orchestrator_config_default_constructs() {
        // Cheap smoke: the daemon binary can at least build a default config.
        let cfg = OrchestratorConfig::default();
        let _debug = format!("{cfg:?}");
    }

    #[test]
    fn loopback_bind_never_refused() {
        refuse_non_loopback_without_explicit_token("127.0.0.1:9745", false)
            .expect("loopback bind must be allowed without an explicit token");
        refuse_non_loopback_without_explicit_token("localhost:9745", false)
            .expect("loopback bind must be allowed without an explicit token");
    }

    #[test]
    fn non_loopback_bind_without_explicit_token_is_refused() {
        let err = refuse_non_loopback_without_explicit_token("0.0.0.0:9745", false)
            .expect_err("non-loopback bind without an explicit token must be refused");
        assert!(
            err.to_string().contains("VOX_ORCHESTRATOR_DAEMON_TOKEN"),
            "refusal message should point at the fix: {err}"
        );
    }

    #[test]
    fn non_loopback_bind_with_explicit_token_is_allowed() {
        refuse_non_loopback_without_explicit_token("0.0.0.0:9745", true)
            .expect("non-loopback bind with an explicitly-set token must be allowed");
    }

    #[test]
    fn resolve_daemon_token_prefers_explicit_env_value() {
        let token = resolve_daemon_token(Some("explicit-token-value".to_string()));
        assert_eq!(token, "explicit-token-value");
    }

    #[test]
    fn resolve_daemon_token_generates_when_unset() {
        let token = resolve_daemon_token(None);
        // A generated token is a UUID string, not empty and not the sentinel
        // explicit value used by the sibling test.
        assert!(!token.is_empty());
        assert_ne!(token, "explicit-token-value");
        assert!(
            uuid::Uuid::parse_str(&token).is_ok(),
            "expected a UUID-shaped generated token, got: {token}"
        );
    }

    /// RED test for the rollback-daemon-token fix: a `stdio`-transport daemon
    /// must never persist the shared token file — see
    /// `persist_daemon_token_file`'s doc comment. This only asserts the
    /// decision point (`is_stdio`); `async_main` wires it to skip the actual
    /// `persist_daemon_token_file` call, which isn't independently callable
    /// from a unit test without a running daemon (covered by the doc
    /// comment's reasoning + code review, not re-asserted here).
    #[test]
    fn stdio_transport_is_detected_for_every_documented_spelling() {
        assert!(orch_daemon::is_stdio_transport("stdio"));
        assert!(orch_daemon::is_stdio_transport("-"));
        assert!(orch_daemon::is_stdio_transport("stdin"));
        assert!(!orch_daemon::is_stdio_transport("127.0.0.1:9745"));
    }

    #[test]
    fn resolve_daemon_profile_matches_env_resolution() {
        let got = super::resolve_daemon_profile()
            .map(|p| p.id_string())
            .map_err(|e| e.to_string());
        let want = vox_orchestrator_mcp::profiles::ActiveProfile::from_env()
            .map(|p| p.id_string())
            .map_err(|e| format!("{e} (VOX_PROFILE)"));
        assert_eq!(got, want);
    }
}
