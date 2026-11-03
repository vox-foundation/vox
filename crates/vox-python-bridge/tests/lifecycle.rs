//! Bridge truths: real-worker hello/print/sleep/crash, and fake-worker framing.

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use serde_json::json;
use vox_python_bridge::{Bridge, BridgeConfig};

fn python3_absent() -> bool {
    let present = Command::new("python3")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if !present {
        eprintln!("skip: python3 is absent; real vox_bridge worker test not run");
    }
    !present
}

fn fake_config(max_fetch_mb: u64, timeout: Duration) -> BridgeConfig {
    let mut config = BridgeConfig::default();
    config.worker_cmd = vec![env!("CARGO_BIN_EXE_fake_worker").into()];
    config.max_fetch_mb = max_fetch_mb;
    config.timeout = timeout;
    config
}

fn real_config(timeout: Duration, debug_ops: bool) -> BridgeConfig {
    let mut config = BridgeConfig::default();
    config.worker_cmd = vec!["python3".into(), "-m".into(), "vox_bridge".into()];
    let worker = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("worker");
    config.env.push((
        OsString::from("PYTHONPATH"),
        worker.as_os_str().to_os_string(),
    ));
    if debug_ops {
        config
            .env
            .push((OsString::from("VOX_BRIDGE_DEBUG_OPS"), OsString::from("1")));
    }
    config.timeout = timeout;
    config.max_fetch_mb = 256;
    config
}

#[tokio::test]
async fn fake_worker_hello_round_trips() {
    let bridge = Bridge::new(fake_config(256, Duration::from_secs(10)));
    let reply = bridge
        .call("hello", json!({}), None)
        .await
        .expect("hello round trip through the fake worker");
    assert_eq!(reply.value["method"], "hello");
    assert_eq!(reply.value["seen"][0], "hello");
    assert!(reply.body.is_none());
}

#[tokio::test]
async fn declared_body_over_cap_fails_before_the_body_is_read() {
    let bridge = Bridge::new(fake_config(1, Duration::from_secs(3)));
    let err = tokio::time::timeout(
        Duration::from_secs(12),
        bridge.call("test.oversize", json!({}), None),
    )
    .await
    .expect("cap check must return without waiting out a body read")
    .expect_err("an oversized declared body is an error");
    assert!(err.to_string().contains("py: fetch exceeds cap"), "{err}");
}

#[tokio::test]
async fn release_ids_are_batched_into_the_next_call() {
    let bridge = Bridge::new(fake_config(256, Duration::from_secs(10)));
    bridge.enqueue_release(7);
    bridge.enqueue_release(8);
    let reply = bridge
        .call("hello", json!({}), None)
        .await
        .expect("hello after batched release");
    assert_eq!(reply.value["seen"][0], "release");
    assert_eq!(reply.value["seen"][1], "hello");
    assert_eq!(reply.value["released"], json!([7, 8]));
}

#[tokio::test]
async fn real_worker_hello_returns_version_platform_and_pid() {
    if python3_absent() {
        return;
    }
    let bridge = Bridge::new(real_config(Duration::from_secs(30), false));
    let reply = bridge
        .call("hello", json!({}), None)
        .await
        .expect("hello through python3 -m vox_bridge");
    assert!(
        reply.value["python"]
            .as_str()
            .is_some_and(|s| !s.is_empty())
    );
    assert!(
        reply.value["platform"]
            .as_str()
            .is_some_and(|s| !s.is_empty())
    );
    assert!(reply.value["pid"].as_u64().is_some());
}

#[tokio::test]
async fn debug_print_reaches_stderr_and_the_reply_still_parses() {
    if python3_absent() {
        return;
    }
    let bridge = Bridge::new(real_config(Duration::from_secs(30), true));
    let reply = bridge
        .call(
            "debug.print",
            json!({"text": "vox-bridge-debug-print"}),
            None,
        )
        .await
        .expect("debug.print reply");
    assert_eq!(reply.value["ok"], true);
    assert!(
        bridge.stderr_text().contains("vox-bridge-debug-print"),
        "stderr={}",
        bridge.stderr_text()
    );
}

#[tokio::test]
async fn debug_sleep_bumps_generation_and_rejects_the_old_handle() {
    if python3_absent() {
        return;
    }
    let bridge = Bridge::new(real_config(Duration::from_secs(1), true));
    bridge
        .call("hello", json!({}), None)
        .await
        .expect("worker is up before the timeout");
    let generation = bridge.generation();
    assert!(generation >= 1, "generation={generation}");
    let err = tokio::time::timeout(
        Duration::from_secs(15),
        bridge.call("debug.sleep", json!({"secs": 30}), None),
    )
    .await
    .expect("sleep timeout must finish inside the grace window")
    .expect_err("a sleep past the deadline is an error");
    assert!(
        bridge.generation() > generation,
        "generation should bump after timeout, err={err}, generation={}",
        bridge.generation()
    );
    let stale = bridge
        .call(
            "hello",
            json!({
                "handle": {
                    "generation": generation,
                    "id": 1,
                    "kind": "array",
                    "dtype": "f64"
                }
            }),
            None,
        )
        .await
        .expect_err("a handle from the previous generation is stale");
    assert!(stale.to_string().contains("py: stale handle"), "{stale}");
}

#[tokio::test]
async fn debug_crash_is_survived_by_the_next_request() {
    if python3_absent() {
        return;
    }
    let bridge = Bridge::new(real_config(Duration::from_secs(30), true));
    let first = bridge
        .call("hello", json!({}), None)
        .await
        .expect("hello before crash");
    let generation = bridge.generation();
    let err = bridge
        .call("debug.crash", json!({}), None)
        .await
        .expect_err("crash is an error");
    assert!(
        bridge.generation() > generation,
        "generation should bump after crash, err={err}"
    );
    let second = bridge
        .call("hello", json!({}), None)
        .await
        .expect("hello on the restarted worker");
    assert!(second.value["pid"].as_u64().is_some());
    assert_ne!(first.value["pid"], second.value["pid"]);
}
