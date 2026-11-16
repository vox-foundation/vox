//! End-to-end integration tests for `vox-cargo-shim` target-first `FairQueue`,
//! leader-follower coalescing (including `cwd`/toolchain isolation & non-blocking
//! grandchild stdio), `sccache` + `VOX_ISOLATED_TARGET`, and post-queue timeouts.

#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};
use vox_build_queue::metrics::MetricRecord;

fn shim_cargo_bin() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_cargo"))
}

fn clean_shim_cmd() -> Command {
    let mut c = Command::new(shim_cargo_bin());
    for k in "VOX_BROKER_DEPTH VOX_BROKER_BYPASS VOX_BROKER_DEBUG VOX_BROKER_COALESCE \
              VOX_BROKER_NO_SCCACHE VOX_BROKER_SCCACHE VOX_BROKER_RUN_TIMEOUT_SECS \
              VOX_BROKER_BUILD_TIMEOUT_SECS VOX_BROKER_RESERVED_SLOTS VOX_BROKER_MAX_CONCURRENT \
              VOX_ISOLATED_TARGET VOX_WORKSTREAM CARGO_TARGET_DIR CARGO_INCREMENTAL \
              RUSTC_WRAPPER RUSTUP_TOOLCHAIN"
        .split_whitespace()
    {
        c.env_remove(k);
    }
    c
}

fn write_executable(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, body).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

fn make_worktree(base: &Path) -> PathBuf {
    let wt = base.join("wt");
    fs::create_dir_all(wt.join(".cargo")).unwrap();
    fs::create_dir_all(wt.join("crates/demo/src")).unwrap();
    fs::create_dir_all(wt.join("crates/other/src")).unwrap();
    fs::write(wt.join(".cargo/config.toml"), "[env]\n").unwrap();
    fs::write(wt.join("Cargo.toml"), "[workspace]\n").unwrap();
    fs::write(
        wt.join("crates/demo/src/lib.rs"),
        "pub fn v() -> i32 { 1 }\n",
    )
    .unwrap();
    fs::write(
        wt.join("crates/other/src/lib.rs"),
        "pub fn w() -> i32 { 2 }\n",
    )
    .unwrap();
    wt
}

fn read_metrics(broker_home: &Path) -> Vec<MetricRecord> {
    vox_build_queue::metrics::read_records(&broker_home.join("metrics.jsonl"))
}

fn wait_for_file(p: &Path, max_ms: u64) {
    let start = Instant::now();
    while !p.exists() && start.elapsed() < Duration::from_millis(max_ms) {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(p.exists(), "timed out waiting for {}", p.display());
}

#[test]
fn target_first_fair_queue_frees_global_slot_for_other_target() {
    let tmp = tempfile::tempdir().unwrap();
    let broker_home = tmp.path().join("broker");
    let cargo_home = tmp.path().join("cargo-home");
    let wt = make_worktree(tmp.path());

    write_executable(
        &cargo_home.join("bin/cargo"),
        r#"#!/bin/sh
[ -n "$FAKE_READY_FILE" ] && : > "$FAKE_READY_FILE"
[ -n "$FAKE_SLEEP_SECS" ] && sleep "$FAKE_SLEEP_SECS"
[ -n "$FAKE_DONE_FILE" ] && : > "$FAKE_DONE_FILE"
exit 0
"#,
    );

    let (a1_ready, a1_done, b1_done) = (
        tmp.path().join("a1.ready"),
        tmp.path().join("a1.done"),
        tmp.path().join("b1.done"),
    );
    let (target_a, target_b) = (wt.join("target-a"), wt.join("target-b"));

    let (wt1, bh1, ch1, ta1, r1, d1) = (
        wt.clone(),
        broker_home.clone(),
        cargo_home.clone(),
        target_a.clone(),
        a1_ready.clone(),
        a1_done.clone(),
    );
    let t_a1 = std::thread::spawn(move || {
        clean_shim_cmd()
            .current_dir(wt1)
            .args(["build", "-p", "one", "--target-dir"])
            .arg(ta1)
            .env("CARGO_HOME", ch1)
            .env("VOX_BROKER_HOME", bh1)
            .env("VOX_BROKER_MAX_CONCURRENT", "2")
            .env("VOX_BROKER_NO_SCCACHE", "1")
            .env("FAKE_READY_FILE", r1)
            .env("FAKE_DONE_FILE", d1)
            .env("FAKE_SLEEP_SECS", "0.9")
            .status()
            .unwrap()
    });
    wait_for_file(&a1_ready, 3000);

    let (wt2, bh2, ch2, ta2) = (
        wt.clone(),
        broker_home.clone(),
        cargo_home.clone(),
        target_a.clone(),
    );
    let t_a2 = std::thread::spawn(move || {
        clean_shim_cmd()
            .current_dir(wt2)
            .args(["build", "-p", "two", "--target-dir"])
            .arg(ta2)
            .env("CARGO_HOME", ch2)
            .env("VOX_BROKER_HOME", bh2)
            .env("VOX_BROKER_MAX_CONCURRENT", "2")
            .env("VOX_BROKER_NO_SCCACHE", "1")
            .env("FAKE_SLEEP_SECS", "0.05")
            .status()
            .unwrap()
    });

    std::thread::sleep(Duration::from_millis(150));
    assert!(broker_home.join("targets").is_dir());
    let busy = vox_build_queue::global::probe_busy_slots(&broker_home, 2).unwrap();
    assert_eq!(busy, 1, "waiter on target-a must NOT hold global slot #2");

    let s_b1 = clean_shim_cmd()
        .current_dir(&wt)
        .args(["build", "-p", "three", "--target-dir"])
        .arg(&target_b)
        .env("CARGO_HOME", &cargo_home)
        .env("VOX_BROKER_HOME", &broker_home)
        .env("VOX_BROKER_MAX_CONCURRENT", "2")
        .env("VOX_BROKER_NO_SCCACHE", "1")
        .env("FAKE_DONE_FILE", &b1_done)
        .env("FAKE_SLEEP_SECS", "0.05")
        .status()
        .unwrap();
    assert!(s_b1.success() && b1_done.exists());
    assert!(t_a1.join().unwrap().success() && t_a2.join().unwrap().success());

    let recs = read_metrics(&broker_home);
    assert_eq!(recs.len(), 3);
    assert!(
        recs.iter()
            .any(|r| r.target_wait_ms >= 400 && r.slot_wait_ms < 300),
        "expected target-a waiter to record target_wait_ms >= 400ms: {recs:?}"
    );
}

#[test]
fn leader_follower_coalescing_run_build_and_mtime_invalidation() {
    let tmp = tempfile::tempdir().unwrap();
    let broker_home = tmp.path().join("broker");
    let cargo_home = tmp.path().join("cargo-home");
    let wt = make_worktree(tmp.path());
    let counter_file = tmp.path().join("builds.log");

    write_executable(
        &cargo_home.join("bin/cargo"),
        r#"#!/bin/sh
if [ "$1" = "run" ]; then
  echo "ran-binary:$*"
  exit 0
fi
echo "${RUSTUP_TOOLCHAIN:-default}|$(pwd)|$*" >> "$FAKE_COUNTER_FILE"
sleep "${FAKE_SLEEP_SECS:-0.45}"
echo "stdout-ok:$1"
echo "stderr-ok:$1" >&2
exit 0
"#,
    );

    let run_check = |dir: &Path, prefix: &[&str], extra: &[&str]| {
        let dir_c = dir.to_path_buf();
        let (bh_c, ch_c, cf_c) = (
            broker_home.clone(),
            cargo_home.clone(),
            counter_file.clone(),
        );
        let args: Vec<String> = prefix
            .iter()
            .copied()
            .chain(std::iter::once("check"))
            .chain(extra.iter().copied())
            .map(String::from)
            .collect();
        std::thread::spawn(move || {
            clean_shim_cmd()
                .current_dir(dir_c)
                .args(&args)
                .env("CARGO_HOME", ch_c)
                .env("VOX_BROKER_HOME", bh_c)
                .env("VOX_BROKER_NO_SCCACHE", "1")
                .env("FAKE_COUNTER_FILE", cf_c)
                .output()
                .unwrap()
        })
    };

    let line_count = || fs::read_to_string(&counter_file).unwrap().lines().count();

    // 1. Two concurrent identical `cargo check -p demo` coalesce into 1 fake cargo invocation
    let (f1, f2) = (
        run_check(&wt, &[], &["-p", "demo"]),
        run_check(&wt, &[], &["-p", "demo"]),
    );
    let (o1, o2) = (f1.join().unwrap(), f2.join().unwrap());
    assert!(o1.status.success() && o2.status.success());
    assert!(String::from_utf8_lossy(&o1.stdout).contains("stdout-ok:check"));
    assert!(String::from_utf8_lossy(&o2.stdout).contains("stdout-ok:check"));
    assert!(String::from_utf8_lossy(&o2.stderr).contains("stderr-ok:check"));
    assert_eq!(line_count(), 1);

    let recs = read_metrics(&broker_home);
    assert_eq!(recs.iter().filter(|r| r.coalesced_hit).count(), 1);
    assert_eq!(recs.iter().find(|r| r.coalesced_hit).unwrap().compile_ms, 0);

    // 2. CR-01: Distinct sibling cwds (`crates/demo` vs `crates/other`) & `+nightly` do NOT coalesce!
    assert!(
        run_check(&wt.join("crates/demo"), &[], &[])
            .join()
            .unwrap()
            .status
            .success()
    );
    assert!(
        run_check(&wt.join("crates/other"), &[], &[])
            .join()
            .unwrap()
            .status
            .success()
    );
    assert!(
        run_check(&wt, &["+nightly"], &["-p", "demo"])
            .join()
            .unwrap()
            .status
            .success()
    );
    assert_eq!(line_count(), 4);

    // 3. `--message-format=json` bypasses coalescing
    let o_json = run_check(&wt, &[], &["-p", "demo", "--message-format=json"])
        .join()
        .unwrap();
    assert!(o_json.status.success() && line_count() == 5);

    // 4. Updating source mtime invalidates recent leader replay and triggers fresh build
    std::thread::sleep(Duration::from_millis(30));
    fs::write(
        wt.join("crates/demo/src/lib.rs"),
        "pub fn v() -> i32 { 3 }\n",
    )
    .unwrap();
    let o_fresh = run_check(&wt, &[], &["-p", "demo"]).join().unwrap();
    assert!(o_fresh.status.success() && line_count() == 6);

    // 5. `cargo run` coalesces the build phase while still executing `run` for both callers
    let spawn_run = || {
        let (wt_c, bh_c, ch_c, cf_c) = (
            wt.clone(),
            broker_home.clone(),
            cargo_home.clone(),
            counter_file.clone(),
        );
        std::thread::spawn(move || {
            clean_shim_cmd()
                .current_dir(wt_c)
                .args(["run", "-p", "demo", "--", "--hello"])
                .env("CARGO_HOME", ch_c)
                .env("VOX_BROKER_HOME", bh_c)
                .env("VOX_BROKER_NO_SCCACHE", "1")
                .env("FAKE_COUNTER_FILE", cf_c)
                .output()
                .unwrap()
        })
    };
    let (r1, r2) = (spawn_run(), spawn_run());
    let (ro1, ro2) = (r1.join().unwrap(), r2.join().unwrap());
    assert!(ro1.status.success() && ro2.status.success() && line_count() == 7);
    assert!(String::from_utf8_lossy(&ro1.stdout).contains("ran-binary:run"));
    assert!(String::from_utf8_lossy(&ro2.stdout).contains("ran-binary:run"));
}

#[test]
fn post_queue_timeout_and_grandchild_stdio_never_wedge_fair_queue() {
    let tmp = tempfile::tempdir().unwrap();
    let broker_home = tmp.path().join("broker");
    let cargo_home = tmp.path().join("cargo-home");
    let wt = make_worktree(tmp.path());
    let bg_pid_file = tmp.path().join("bg.pid");

    write_executable(
        &cargo_home.join("bin/cargo"),
        r#"#!/bin/sh
[ -n "$FAKE_READY_FILE" ] && : > "$FAKE_READY_FILE"
if [ -n "$FAKE_SPAWN_BG_PID_FILE" ]; then
  sleep 10 &
  echo $! > "$FAKE_SPAWN_BG_PID_FILE"
  echo "leader-with-daemon"
  exit 0
fi
sleep "${FAKE_SLEEP_SECS:-0.1}"
exit 0
"#,
    );

    // CR-02: Coalesceable leader spawns a background daemon inheriting stdout/stderr -> returns < 2s
    let t_bg = Instant::now();
    let bg_out = clean_shim_cmd()
        .current_dir(&wt)
        .args(["check", "-p", "daemon-holder"])
        .env("CARGO_HOME", &cargo_home)
        .env("VOX_BROKER_HOME", &broker_home)
        .env("VOX_BROKER_NO_SCCACHE", "1")
        .env("FAKE_SPAWN_BG_PID_FILE", &bg_pid_file)
        .output()
        .unwrap();
    assert!(bg_out.status.success());
    assert!(
        t_bg.elapsed() < Duration::from_secs(2),
        "grandchild stdio hung leader: {:?}",
        t_bg.elapsed()
    );
    assert!(String::from_utf8_lossy(&bg_out.stdout).contains("leader-with-daemon"));
    if let Ok(raw) = fs::read_to_string(&bg_pid_file)
        && let Ok(pid) = raw.trim().parse::<i32>()
    {
        let _ = Command::new("/bin/kill")
            .args(["-9", &pid.to_string()])
            .status();
    }

    let b1_ready = tmp.path().join("b1.ready");
    let (wt1, bh1, ch1, r1) = (
        wt.clone(),
        broker_home.clone(),
        cargo_home.clone(),
        b1_ready.clone(),
    );
    let b1 = std::thread::spawn(move || {
        clean_shim_cmd()
            .current_dir(wt1)
            .args(["build", "-p", "holder"])
            .env("CARGO_HOME", ch1)
            .env("VOX_BROKER_HOME", bh1)
            .env("VOX_BROKER_NO_SCCACHE", "1")
            .env("FAKE_READY_FILE", r1)
            .env("FAKE_SLEEP_SECS", "1.25")
            .status()
            .unwrap()
    });
    wait_for_file(&b1_ready, 3000);

    // Build 2 waits >1.0s in FairQueue with VOX_BROKER_RUN_TIMEOUT_SECS=1, then runs 0.15s -> exits 0
    let b2_status = clean_shim_cmd()
        .current_dir(&wt)
        .args(["build", "-p", "waiter"])
        .env("CARGO_HOME", &cargo_home)
        .env("VOX_BROKER_HOME", &broker_home)
        .env("VOX_BROKER_NO_SCCACHE", "1")
        .env("VOX_BROKER_RUN_TIMEOUT_SECS", "1")
        .env("FAKE_SLEEP_SECS", "0.15")
        .status()
        .unwrap();
    assert!(b1.join().unwrap().success() && b2_status.success());

    // Build 3 actively hangs for 5s with VOX_BROKER_RUN_TIMEOUT_SECS=1 -> killed in ~1s with exit 124
    let t_hang = Instant::now();
    let b3_out = clean_shim_cmd()
        .current_dir(&wt)
        .args(["build", "-p", "hanger"])
        .env("CARGO_HOME", &cargo_home)
        .env("VOX_BROKER_HOME", &broker_home)
        .env("VOX_BROKER_NO_SCCACHE", "1")
        .env("VOX_BROKER_RUN_TIMEOUT_SECS", "1")
        .env("FAKE_SLEEP_SECS", "5")
        .output()
        .unwrap();
    assert_eq!(b3_out.status.code(), Some(124));
    assert!(t_hang.elapsed() < Duration::from_secs(4));

    let recs = read_metrics(&broker_home);
    assert_eq!(recs.len(), 4);
    assert!(!recs[2].timed_out && recs[3].timed_out);
}

#[test]
fn sccache_stub_and_workstream_isolation_env_injection_and_guards() {
    let tmp = tempfile::tempdir().unwrap();
    let broker_home = tmp.path().join("broker");
    let cargo_home = tmp.path().join("cargo-home");
    let sc_dir = tmp.path().join("sc-bin");
    let wt = make_worktree(tmp.path());

    let sc_stub = sc_dir.join("sccache");
    write_executable(&sc_stub, "#!/bin/sh\nexit 0\n");
    write_executable(
        &cargo_home.join("bin/cargo"),
        r#"#!/bin/sh
echo "WRAPPER=${RUSTC_WRAPPER-<unset>}"
echo "TARGET=${CARGO_TARGET_DIR-<unset>}"
echo "INCR=${CARGO_INCREMENTAL-<unset>}"
echo "ARGS=$*"
exit 0
"#,
    );

    let sys_path =
        std::env::join_paths([&sc_dir, Path::new("/bin"), Path::new("/usr/bin")]).unwrap();

    // 1. Auto-sccache + VOX_ISOLATED_TARGET=1 + VOX_WORKSTREAM=axis-v75
    let out = clean_shim_cmd()
        .current_dir(&wt)
        .args(["check", "--broker-run-timeout=30", "-p", "demo"])
        .env("CARGO_HOME", &cargo_home)
        .env("VOX_BROKER_HOME", &broker_home)
        .env("PATH", &sys_path)
        .env("VOX_ISOLATED_TARGET", "1")
        .env("VOX_WORKSTREAM", "axis-v75")
        .output()
        .unwrap();
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    let canon_sc = sc_stub.canonicalize().unwrap();
    let canon_wt = wt.canonicalize().unwrap();
    assert!(
        stdout.contains(&format!("WRAPPER={}", canon_sc.display())),
        "got: {stdout}"
    );
    assert!(
        stdout.contains(&format!("TARGET={}/target/ws-axis-v75", canon_wt.display())),
        "got: {stdout}"
    );
    assert!(
        stdout.contains("INCR=0") && stdout.contains("ARGS=check -p demo"),
        "got: {stdout}"
    );

    // 2. --manifest-path skips workstream target isolation, VOX_BROKER_NO_SCCACHE=1 skips sccache
    let out_mp = clean_shim_cmd()
        .current_dir(&wt)
        .args(["build", "--manifest-path", "Cargo.toml"])
        .env("CARGO_HOME", &cargo_home)
        .env("VOX_BROKER_HOME", &broker_home)
        .env("PATH", &sys_path)
        .env("VOX_BROKER_NO_SCCACHE", "1")
        .env("VOX_ISOLATED_TARGET", "1")
        .env("VOX_WORKSTREAM", "axis-v75")
        .output()
        .unwrap();
    assert!(out_mp.status.success());
    let stdout_mp = String::from_utf8_lossy(&out_mp.stdout);
    assert!(
        stdout_mp.contains("WRAPPER=<unset>")
            && stdout_mp.contains("TARGET=<unset>")
            && stdout_mp.contains("INCR=<unset>"),
        "got: {stdout_mp}"
    );

    // 3. Recursion guard (VOX_BROKER_DEPTH=2 -> exit 70) & debug mode (VOX_BROKER_DEBUG=1 -> exit 0)
    let depth_out = clean_shim_cmd()
        .arg("check")
        .env("VOX_BROKER_DEPTH", "2")
        .output()
        .unwrap();
    assert_eq!(depth_out.status.code(), Some(70));

    let dbg_out = clean_shim_cmd()
        .args(["check", "--broker-run-timeout", "15", "-p", "demo"])
        .env("CARGO_HOME", &cargo_home)
        .env("VOX_BROKER_DEBUG", "1")
        .output()
        .unwrap();
    assert_eq!(dbg_out.status.code(), Some(0));
    let dbg_err = String::from_utf8_lossy(&dbg_out.stderr);
    assert!(dbg_err.contains("vox-broker-debug:") && !dbg_err.contains("--broker-run-timeout"));
}
