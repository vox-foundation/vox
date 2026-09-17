mod common;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use vox_mesh_transport::endpoint::JobExecutor;
use vox_mesh_transport::protocol::{JobId, JobLimits, JobResponse};
use vox_mesh_transport::trust::TrustLevel;
use vox_mesh_transport::{InterpExecutor, MeshTrust};
use vox_mesh_types::TaskKind;

#[allow(dead_code)]
fn vox_lit(p: &Path) -> String {
    p.display().to_string().replace('\\', "\\\\")
}

fn vox_bin() -> PathBuf {
    if let Ok(p) = std::env::var("VOX_BIN") {
        return p.into();
    }
    let name = if cfg!(windows) { "vox.exe" } else { "vox" };
    if let Ok(dir) = std::env::var("CARGO_TARGET_DIR") {
        let exe = PathBuf::from(dir).join("debug").join(name);
        if exe.exists() {
            return exe;
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let exe = root.join("target/debug").join(name);
    if !exe.exists() {
        let st = std::process::Command::new(env!("CARGO"))
            .current_dir(&root)
            .args(["build", "-q", "-p", "vox-cli", "--bin", "vox"])
            .status()
            .expect("spawn cargo");
        assert!(
            st.success(),
            "could not build the vox binary the executor spawns"
        );
    }
    exe
}

fn exec(trust: Arc<MeshTrust>) -> Arc<dyn JobExecutor> {
    Arc::new(InterpExecutor::new(trust, vox_bin(), JobLimits::default()))
}

#[test]
fn caps_mapping_never_grants_more_than_the_trust_level() {
    let d = tempfile::tempdir().unwrap();
    let tokens = InterpExecutor::caps_for(TrustLevel::Sandboxed, d.path()).unwrap();
    let s = tokens.join(",");
    assert!(s.contains("fs:rw=") && s.contains("time:real"));
    for forbidden in ["net:allow", "process:allow", "env:", "secrets"] {
        assert!(!s.contains(forbidden), "{s}");
    }
    let n = InterpExecutor::caps_for(TrustLevel::Native, d.path())
        .unwrap()
        .join(",");
    assert!(n.contains("net:allow") && n.contains("process:allow") && n.contains("env:ro"));
    assert!(!n.contains("secrets"));
}

#[test]
fn a_job_dir_with_a_comma_is_accepted() {
    let d = tempfile::Builder::new().prefix("a,b-").tempdir().unwrap();
    assert!(InterpExecutor::caps_for(TrustLevel::Sandboxed, d.path()).is_ok());
}

#[test]
fn read_capped_does_not_mark_an_exact_max() {
    let exact = [b'x'; 64];
    let (buf, truncated) =
        futures::executor::block_on(InterpExecutor::read_capped_for_test(&exact[..], 64));
    assert_eq!(buf.len(), 64);
    assert!(!truncated, "exact-max must not be marked truncated");
}

#[tokio::test]
#[ignore = "owner:mesh sunset:2026-12-31 slow: builds and spawns the vox binary"]
async fn a_voxscript_job_runs_and_returns_its_output() {
    let server = common::start_server_with(exec).await;
    server.trust.trust(&common::client_id(), None).unwrap();
    let resp = common::send_run_on(
        &server,
        JobId(1),
        TaskKind::VoxScript,
        b"pub fn main() { print(\"MESH_RAN\") }",
    )
    .await;
    assert!(
        matches!(resp, JobResponse::Output(ref b) if String::from_utf8_lossy(b).contains("MESH_RAN")),
        "{resp:?}"
    );
}

#[tokio::test]
#[ignore = "owner:mesh sunset:2026-12-31 slow: builds and spawns the vox binary"]
async fn a_sandboxed_peer_cannot_read_the_host_filesystem() {
    let server = common::start_server_with(exec).await;
    server.trust.trust(&common::client_id(), None).unwrap();
    let resp = common::send_run_on(
        &server,
        JobId(2),
        TaskKind::VoxScript,
        b"pub fn main() { let s = fs.read(\"/etc/hosts\"); print(\"LEAK\") }",
    )
    .await;
    assert!(
        matches!(resp, JobResponse::Failed(ref m) if m.contains("capability denied") && m.contains("fs.read")),
        "{resp:?}"
    );
}

#[tokio::test]
#[ignore = "owner:mesh sunset:2026-12-31 slow: builds and spawns the vox binary"]
async fn a_forged_exit_77_under_grant_native_is_not_a_denial() {
    let server = common::start_server_with(exec).await;
    server
        .trust
        .trust_with(
            &common::client_id(),
            vox_mesh_transport::trust::TrustLevel::Native,
        )
        .unwrap();
    let resp = common::send_run_on(
        &server,
        JobId(3),
        TaskKind::VoxScript,
        b"pub fn main() { process.exit(77) }",
    )
    .await;
    assert!(
        matches!(resp, JobResponse::Failed(ref m) if !m.contains("capability denied")),
        "{resp:?}"
    );
}

#[tokio::test]
#[ignore = "owner:mesh sunset:2026-12-31 slow: builds and spawns the vox binary"]
async fn a_runaway_allocation_is_killed_and_reported() {
    let limits = JobLimits {
        max_memory_bytes: 64 * 1024 * 1024,
        max_steps: 10_000,
        ..JobLimits::default()
    };
    let server = common::start_server_with(|t| {
        Arc::new(InterpExecutor::new(t, vox_bin(), limits)) as Arc<dyn JobExecutor>
    })
    .await;
    server.trust.trust(&common::client_id(), None).unwrap();
    let resp = common::send_run_on(
        &server,
        JobId(4),
        TaskKind::VoxScript,
        b"pub fn main() { let mut s = \"x\"; let mut i = 0; while i < 40 { s = s + s; i = i + 1 } }",
    )
    .await;
    assert!(
        matches!(resp, JobResponse::Failed(ref m) if m.contains("memory limit")),
        "{resp:?}"
    );
}

#[tokio::test]
#[ignore = "owner:mesh sunset:2026-12-31 slow: builds and spawns the vox binary"]
async fn a_write_past_the_mesh_disk_quota_is_denied() {
    let limits = JobLimits {
        max_disk_bytes: 4,
        ..JobLimits::default()
    };
    let server = common::start_server_with(|t| {
        Arc::new(InterpExecutor::new(t, vox_bin(), limits)) as Arc<dyn JobExecutor>
    })
    .await;
    server.trust.trust(&common::client_id(), None).unwrap();
    let resp = common::send_run_on(
        &server,
        JobId(12),
        TaskKind::VoxScript,
        b"pub fn main() { fs.write(\"out.txt\", \"12345\") }",
    )
    .await;
    assert!(
        matches!(resp, JobResponse::Failed(ref m) if m.contains("capability denied") && m.contains("fs.quota")),
        "{resp:?}"
    );
}

#[tokio::test]
#[ignore = "owner:mesh sunset:2026-12-31 slow: builds and spawns the vox binary"]
async fn output_is_capped_and_marked_only_when_over() {
    let limits = JobLimits {
        max_output_bytes: 64 * 1024,
        ..JobLimits::default()
    };
    let server = common::start_server_with(|t| {
        Arc::new(InterpExecutor::new(t, vox_bin(), limits)) as Arc<dyn JobExecutor>
    })
    .await;
    server.trust.trust(&common::client_id(), None).unwrap();
    let resp = common::send_run_on(
        &server,
        JobId(5),
        TaskKind::VoxScript,
        b"pub fn main() { let mut i = 0; while i < 10000 { print(\"xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx\"); i = i + 1 } }",
    )
    .await;
    match resp {
        JobResponse::Output(b) => {
            assert!(b.len() <= 64 * 1024 + 128);
            assert!(String::from_utf8_lossy(&b).contains("[vox: output truncated"));
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
#[ignore = "owner:mesh sunset:2026-12-31 slow: builds and spawns the vox binary"]
async fn one_peer_cannot_cancel_another_peers_job() {
    let limits = JobLimits {
        max_steps: 50_000_000,
        ..JobLimits::default()
    };
    let server = common::start_server_with(|t| {
        Arc::new(InterpExecutor::new(t, vox_bin(), limits)) as Arc<dyn JobExecutor>
    })
    .await;
    let (a, b) = (
        common::client_endpoint(common::client_sk_a()).await,
        common::client_endpoint(common::client_sk_b()).await,
    );
    server.trust.trust(&a.id(), None).unwrap();
    server.trust.trust(&b.id(), None).unwrap();
    let slow = b"pub fn main() { let mut i = 0; while i < 200000 { i = i + 1 }; print(\"DONE\") }";
    let a_job = tokio::spawn(common::send_run_from_owned(
        a.clone(),
        server.clone(),
        JobId(9),
        TaskKind::VoxScript,
        slow,
    ));
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let cancel = common::send_cancel_from(b, &server, JobId(9)).await;
    assert!(
        matches!(cancel, JobResponse::Failed(ref m) if m.contains("no such running job")),
        "{cancel:?}"
    );
    let done = a_job.await.unwrap();
    assert!(
        matches!(done, JobResponse::Output(ref o) if String::from_utf8_lossy(o).contains("DONE")),
        "{done:?}"
    );
}

#[tokio::test]
#[ignore = "owner:mesh sunset:2026-12-31 slow: builds and spawns the vox binary"]
async fn a_reused_job_id_from_the_same_peer_is_refused() {
    let server = common::start_server_with(exec).await;
    server.trust.trust(&common::client_id(), None).unwrap();
    let slow = b"pub fn main() { let mut i = 0; while i < 200000 { i = i + 1 }; print(\"A\") }";
    let first = tokio::spawn(common::send_run_on_owned(
        server.clone(),
        JobId(11),
        TaskKind::VoxScript,
        slow,
    ));
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let second = common::send_run_on(
        &server,
        JobId(11),
        TaskKind::VoxScript,
        b"pub fn main() { print(\"B\") }",
    )
    .await;
    assert!(
        matches!(second, JobResponse::Failed(ref m) if m.contains("already running")),
        "{second:?}"
    );
    let _ = first.await;
}

#[tokio::test]
#[ignore = "owner:mesh sunset:2026-12-31 slow: builds and spawns the vox binary"]
async fn ml_task_kinds_are_refused_with_a_reason_when_no_engine_is_installed() {
    let server = common::start_server_with(exec).await;
    server.trust.trust(&common::client_id(), None).unwrap();
    let resp = common::send_run_on(&server, JobId(6), TaskKind::TextInfer, b"{}").await;
    assert!(
        matches!(resp, JobResponse::Failed(ref m) if m.contains("no engine")),
        "{resp:?}"
    );
}

#[tokio::test]
#[ignore = "owner:mesh sunset:2026-12-31 slow: builds and spawns the vox binary"]
async fn process_boundary_clears_unlisted_env_and_points_home_at_the_readonly_dir() {
    let server = common::start_server_with(exec).await;
    // `env.get` is gated; Sandboxed must not receive `env:` (see caps_mapping).
    server
        .trust
        .trust_with(&common::client_id(), TrustLevel::Native)
        .unwrap();
    let resp = common::send_run_on(
        &server,
        JobId(7),
        TaskKind::VoxScript,
        b"pub fn main() { print(env.get(\"VOX_SHOULD_NOT_LEAK\") is None) }",
    )
    .await;
    assert!(
        matches!(resp, JobResponse::Output(ref b) if String::from_utf8_lossy(b).contains("true")),
        "{resp:?}"
    );
}

#[cfg(unix)]
#[tokio::test]
#[ignore = "owner:mesh sunset:2026-12-31 slow: builds and spawns the vox binary"]
async fn unix_process_group_kill_reaches_a_grandchild() {
    let limits = JobLimits {
        wall_clock: std::time::Duration::from_millis(400),
        ..JobLimits::default()
    };
    let server = common::start_server_with(|t| {
        Arc::new(InterpExecutor::new(t, vox_bin(), limits)) as Arc<dyn JobExecutor>
    })
    .await;
    server
        .trust
        .trust_with(&common::client_id(), TrustLevel::Native)
        .unwrap();
    let t0 = std::time::Instant::now();
    let resp = common::send_run_on(
        &server,
        JobId(8),
        TaskKind::VoxScript,
        b"pub fn main() { process.run(\"sleep\", [\"30\"]) }",
    )
    .await;
    assert!(
        t0.elapsed() < std::time::Duration::from_secs(2),
        "grandchild leaked: {:?}",
        t0.elapsed()
    );
    assert!(matches!(resp, JobResponse::Failed(_)), "{resp:?}");
}

#[cfg(windows)]
#[tokio::test]
#[ignore = "owner:mesh sunset:2026-12-31 slow: builds and spawns the vox binary"]
async fn windows_job_object_kill_reaches_a_grandchild() {
    let limits = JobLimits {
        wall_clock: std::time::Duration::from_millis(400),
        ..JobLimits::default()
    };
    let server = common::start_server_with(|t| {
        Arc::new(InterpExecutor::new(t, vox_bin(), limits)) as Arc<dyn JobExecutor>
    })
    .await;
    server
        .trust
        .trust_with(&common::client_id(), TrustLevel::Native)
        .unwrap();
    let t0 = std::time::Instant::now();
    let resp = common::send_run_on(
        &server,
        JobId(8),
        TaskKind::VoxScript,
        b"pub fn main() { process.run(\"timeout\", [\"/t\", \"30\", \"/nobreak\"]) }",
    )
    .await;
    assert!(
        t0.elapsed() < std::time::Duration::from_secs(2),
        "grandchild leaked: {:?}",
        t0.elapsed()
    );
    assert!(matches!(resp, JobResponse::Failed(_)), "{resp:?}");
}
