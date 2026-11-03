//! Profile-gated boot against the real daemon step table (capability profiles plan 03).

use std::path::{Path, PathBuf};

use vox_orchestrator::OrchestratorConfig;
use vox_orchestrator_d::boot_plan::{BootOutcome, DependencyMode, run_boot_plan};
use vox_orchestrator_d::daemon_steps::{DAEMON_STEPS, DaemonBootCtx};
use vox_orchestrator_mcp::profiles::{ActiveProfile, Subsystem};

const POPULI_STEPS: [&str; 3] = [
    "populi_federation_poller",
    "a2a_remote_result_poller",
    "a2a_remote_worker_poller",
];

fn project_dir() -> tempfile::TempDir {
    assert!(
        !std::env::var("VOX_WORKSPACE_JOURNEY_STORE")
            .is_ok_and(|v| v.trim().eq_ignore_ascii_case("canonical")),
        "unset VOX_WORKSPACE_JOURNEY_STORE=canonical: these tests observe the project-local store"
    );
    let tmp = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        tmp.path().join("Vox.toml"),
        "[package]\nname = \"boot-test\"\nversion = \"0.0.0\"\n",
    )
    .expect("write Vox.toml");
    tmp
}

fn store_db(dir: &Path) -> PathBuf {
    dir.join(vox_db::store::DEFAULT_PROJECT_STORE_PATH)
}

async fn boot(ids: &[&str], dir: &Path) -> (DaemonBootCtx, BootOutcome) {
    let profile = ActiveProfile::resolve(ids).expect("profile resolves");
    let mut ctx = DaemonBootCtx::new(
        OrchestratorConfig::for_testing(),
        Some(dir),
        profile.clone(),
        dir.join("plugins"),
    );
    let out = run_boot_plan(
        DAEMON_STEPS,
        &mut ctx,
        &profile.subsystems,
        DependencyMode::for_profile(&profile),
    )
    .await
    .expect("boot plan completes");
    (ctx, out)
}

/// D1: `attention` is DB-free, so it is the profile that must open no turso connection.
#[tokio::test(flavor = "multi_thread")]
async fn attention_profile_opens_no_store_and_starts_no_populi_pollers() {
    let dir = project_dir();
    let (ctx, out) = boot(&["attention"], dir.path()).await;

    assert!(!out.ran.contains(&"db"), "db step ran: {:?}", out.ran);
    for step in POPULI_STEPS {
        assert!(!out.ran.contains(&step), "{step} ran: {:?}", out.ran);
    }
    assert!(ctx.db.is_none());
    assert!(
        !store_db(dir.path()).exists(),
        "attention boot opened {}",
        store_db(dir.path()).display()
    );
    assert!(ctx.populi_poll_join.lock().unwrap().is_none());

    let state = ctx.state.as_ref().expect("server_state core step ran");
    assert!(state.db.is_none());
    assert_eq!(out.report.booted, ctx.profile.subsystems);
    assert!(out.report.failed.is_empty(), "{:?}", out.report.failed);
    assert_eq!(state.boot_report_snapshot(), out.report);
}

/// D1: `models` requires `db` (scoreboard, bench evidence, chat history) but nothing that hangs off it.
#[tokio::test(flavor = "multi_thread")]
async fn models_profile_boots_db_but_no_sessions_or_populi_pollers() {
    let dir = project_dir();
    let (ctx, out) = boot(&["models"], dir.path()).await;

    assert!(out.ran.contains(&"db"), "{:?}", out.ran);
    assert!(store_db(dir.path()).exists());
    assert!(out.report.booted.contains(&Subsystem::Db));
    assert!(ctx.state.as_ref().expect("state").db.is_some());
    for step in POPULI_STEPS
        .iter()
        .copied()
        .chain(["sessions", "agent_fleet"])
    {
        assert!(!out.ran.contains(&step), "{step} ran: {:?}", out.ran);
    }
    assert!(ctx.populi_poll_join.lock().unwrap().is_none());
    assert_eq!(out.report.booted, ctx.profile.subsystems);
    assert!(out.report.failed.is_empty(), "{:?}", out.report.failed);
}

#[tokio::test(flavor = "multi_thread")]
async fn knowledge_profile_opens_store_but_not_agent_fleet() {
    let dir = project_dir();
    let (ctx, out) = boot(&["knowledge"], dir.path()).await;

    // Positive control: the store observation above is live.
    assert!(out.ran.contains(&"db"), "{:?}", out.ran);
    assert!(store_db(dir.path()).exists());
    assert!(ctx.state.as_ref().expect("state").db.is_some());

    assert!(!out.ran.contains(&"agent_fleet"), "{:?}", out.ran);
    assert!(!out.report.booted.contains(&Subsystem::AgentFleet));
    assert!(!out.report.failed.contains_key(&Subsystem::AgentFleet));
    assert!(out.report.booted.contains(&Subsystem::Sessions));
}
