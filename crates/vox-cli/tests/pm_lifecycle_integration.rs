//! PM lifecycle integration: path-only graph (`add` / `lock` / `sync` / `update` / `remove`)
//! plus a **local TCP stub** for registry `download` JSON (no external network).
//!
//! Uses process cwd because handlers resolve `Vox.toml` / `vox.lock` relative to `.`.

use serial_test::serial;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use vox_cli::commands::pm::{self, PmCli};
use vox_cli::commands::{add, lock, remove, sync, update};
use vox_db::VoxDb;
use vox_pm::lockfile::PackageSource;
use vox_pm::{DownloadResponse, Lockfile, SemVer, content_hash};

static PM_WORKDIR_GUARD: Mutex<()> = Mutex::new(());

struct Pushd<'a> {
    previous: std::path::PathBuf,
    _guard: std::sync::MutexGuard<'a, ()>,
}

impl<'a> Pushd<'a> {
    fn new(dir: &Path, guard: std::sync::MutexGuard<'a, ()>) -> Self {
        let previous = std::env::current_dir().expect("cwd");
        std::env::set_current_dir(dir).expect("chdir fixture");
        Self {
            previous,
            _guard: guard,
        }
    }
}

impl Drop for Pushd<'_> {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.previous);
    }
}

const MINIMAL_VOX_TOML: &str = r#"[package]
name = "pm-fixture"
version = "0.1.0"
"#;

const VOX_TOML_ONE_REGISTRY_DEP: &str = r#"[package]
name = "pm-fixture"
version = "0.1.0"

[dependencies]
regfrozen = "*"
"#;

fn prepare_sidecar(tmp: &Path) -> PathBuf {
    let dep = tmp.join("sidecar");
    fs::create_dir_all(dep.join("src")).unwrap();
    fs::write(dep.join("README.txt"), b"stub").unwrap();
    dep
}

/// Accept one connection, return 200 + [`DownloadResponse`] JSON for any `/download` request.
async fn spawn_one_shot_registry_stub(artifact: Vec<u8>) -> u16 {
    let h = content_hash(&artifact);
    let body = serde_json::to_string(&DownloadResponse {
        content_hash: h,
        data: artifact,
    })
    .expect("serialize DownloadResponse");

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind stub registry");
    let port = listener.local_addr().expect("addr").port();

    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("stub accept");
        let mut buf = vec![0u8; 24_576];
        let _ = stream.read(&mut buf).await;
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        let _ = stream.write_all(head.as_bytes()).await;
        let _ = stream.write_all(body.as_bytes()).await;
    });

    port
}

async fn seed_pm_local_index(root: &Path, name: &str, version: &str, artifact: &[u8]) {
    let db_dir = root.join(".vox_modules");
    fs::create_dir_all(&db_dir).unwrap();
    let db_path = db_dir.join("local_store.db");
    let db = VoxDb::open(db_path.to_str().expect("utf-8 db path"))
        .await
        .expect("open local PM index");
    db.record_pm_registry_mirror(name, version, artifact)
        .await
        .expect("record_pm_registry_mirror");
}

#[tokio::test]
#[serial(pm_workdir)]
async fn pm_path_dep_lock_sync_update_round_trip() {
    let g = PM_WORKDIR_GUARD.lock().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    prepare_sidecar(tmp.path());

    let root = tmp.path().join("proj");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("Vox.toml"), MINIMAL_VOX_TOML).unwrap();

    let _push = Pushd::new(&root, g);

    add::run("sidecar", None, Some("../sidecar"))
        .await
        .expect("vox add");
    lock::run(false).await.expect("vox lock");
    sync::run(None, false).await.expect("vox sync");
    sync::run(None, true).await.expect("vox sync --frozen");
    lock::run(true).await.expect("vox lock --locked");
    update::run().await.expect("vox update");

    let lock = Lockfile::load(&root.join("vox.lock")).expect("load vox.lock");
    assert!(
        lock.packages.contains_key("sidecar"),
        "lockfile should contain path dep"
    );

    remove::run("sidecar").await.expect("vox remove");
    lock::run(false)
        .await
        .expect("vox lock refresh after remove");

    let toml = fs::read_to_string(root.join("Vox.toml")).unwrap();
    assert!(
        !toml.contains("sidecar"),
        "Vox.toml should drop removed dep: {toml}"
    );
    let lock = Lockfile::load(&root.join("vox.lock")).expect("load vox.lock");
    assert!(
        lock.packages.is_empty(),
        "lockfile should be empty after removing sole dep"
    );
}

#[tokio::test]
#[serial(pm_workdir)]
async fn pm_lock_locked_rejects_stale_lock() {
    let g = PM_WORKDIR_GUARD.lock().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let _ = prepare_sidecar(tmp.path());
    let root = tmp.path().join("proj2");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("Vox.toml"), MINIMAL_VOX_TOML).unwrap();

    let _push = Pushd::new(&root, g);

    add::run("sidecar", None, Some("../sidecar")).await.unwrap();
    lock::run(false).await.unwrap();

    add::run("extra", None, Some("../sidecar")).await.unwrap();

    let err = lock::run(true).await.expect_err("stale lock should fail");
    let msg = err.to_string();
    assert!(
        msg.contains("out of date"),
        "expected stale lock message, got: {msg}"
    );

    lock::run(false).await.unwrap();
    lock::run(true).await.unwrap();
}

#[tokio::test]
#[serial(pm_workdir)]
async fn pm_registry_sync_downloads_locked_artifact() {
    let g = PM_WORKDIR_GUARD.lock().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("regproj");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("Vox.toml"), MINIMAL_VOX_TOML).unwrap();

    let artifact: &[u8] = b"vox-pm-registry-integration-artifact-bytes";
    let h = content_hash(artifact);
    let mut lock = Lockfile::new();
    lock.add(
        "regfixture",
        &SemVer::parse("0.1.0").unwrap(),
        &h,
        PackageSource::Registry,
        vec![],
        vec![],
    );
    lock.save(&root.join("vox.lock")).unwrap();

    let port = spawn_one_shot_registry_stub(artifact.to_vec()).await;
    tokio::task::yield_now().await;

    let _push = Pushd::new(&root, g);
    let base = format!("http://127.0.0.1:{port}");
    sync::run(Some(&base), false)
        .await
        .expect("vox sync against stub registry");

    let blob = root
        .join(".vox_modules")
        .join("dl")
        .join("regfixture")
        .join("0.1.0")
        .join("artifact.bin");
    assert_eq!(fs::read(&blob).unwrap(), artifact);
}

#[tokio::test]
#[serial(pm_workdir)]
async fn pm_registry_sync_frozen_matches_manifest_after_lock() {
    let g = PM_WORKDIR_GUARD.lock().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("frozenproj");
    fs::create_dir_all(&root).unwrap();
    let artifact: &[u8] = b"sync-frozen-registry-bytes-xyz";
    seed_pm_local_index(&root, "regfrozen", "0.1.0", artifact).await;
    fs::write(root.join("Vox.toml"), VOX_TOML_ONE_REGISTRY_DEP).unwrap();

    let _push = Pushd::new(&root, g);
    lock::run(false).await.expect("vox lock");

    let port = spawn_one_shot_registry_stub(artifact.to_vec()).await;
    tokio::task::yield_now().await;
    let base = format!("http://127.0.0.1:{port}");
    sync::run(Some(&base), true)
        .await
        .expect("vox sync --frozen with stub registry");

    let blob = root
        .join(".vox_modules")
        .join("dl")
        .join("regfrozen")
        .join("0.1.0")
        .join("artifact.bin");
    assert_eq!(fs::read(&blob).unwrap(), artifact);
}

#[tokio::test]
#[serial(pm_workdir)]
async fn pm_cli_mirror_indexes_artifact() {
    let g = PM_WORKDIR_GUARD.lock().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("mirrorcli");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("Vox.toml"), MINIMAL_VOX_TOML).unwrap();
    let art = root.join("pkg.blob");
    fs::write(&art, b"mirror-cli-direct").unwrap();
    let _push = Pushd::new(&root, g);
    pm::run(PmCli::Mirror {
        name: "mirpkg".to_string(),
        version: "2.0.0".to_string(),
        file: Some(art),
        from_registry: None,
    })
    .await
    .expect("vox pm mirror");
    let db_path = root.join(".vox_modules").join("local_store.db");
    let db = VoxDb::open(db_path.to_str().expect("utf-8"))
        .await
        .expect("open index");
    let vers = db.get_package_versions("mirpkg").await.expect("versions");
    assert_eq!(vers.len(), 1);
    assert_eq!(vers[0].0, "2.0.0");
}

#[tokio::test]
#[serial(pm_workdir)]
async fn pm_cli_mirror_from_registry_stub() {
    let g = PM_WORKDIR_GUARD.lock().unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("mirrorfromreg");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("Vox.toml"), MINIMAL_VOX_TOML).unwrap();
    let artifact: &[u8] = b"mirror-via-registry-download-json";
    let port = spawn_one_shot_registry_stub(artifact.to_vec()).await;
    tokio::task::yield_now().await;
    let _push = Pushd::new(&root, g);
    let base = format!("http://127.0.0.1:{port}");
    pm::run(PmCli::Mirror {
        name: "regmirror".to_string(),
        version: "0.1.0".to_string(),
        file: None,
        from_registry: Some(base),
    })
    .await
    .expect("vox pm mirror --from-registry");
    let db_path = root.join(".vox_modules").join("local_store.db");
    let db = VoxDb::open(db_path.to_str().expect("utf-8"))
        .await
        .expect("open index");
    let vers = db
        .get_package_versions("regmirror")
        .await
        .expect("versions");
    assert_eq!(vers.len(), 1);
    assert_eq!(vers[0].0, "0.1.0");
}
