#![allow(dead_code)]
// Each integration test binary (`tool_dispatch_tests`, `tool_dispatch_phase_b`) uses a subset.

use std::sync::Mutex;

use vox_mcp::llm_bridge::infer_test_stub::{INFER_STUB_ACK_ENV, INFER_STUB_BODY_ENV};

/// Serializes env mutations for synthetic infer responses (`set_var` / `remove_var` are `unsafe` in Rust 2024).
static INFER_FIXTURE_TEST_LOCK: Mutex<()> = Mutex::new(());
static ORCH_DAEMON_ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

pub struct InferFixtureEnvGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl InferFixtureEnvGuard {
    pub fn enter(body_json: &str) -> Self {
        let lock = INFER_FIXTURE_TEST_LOCK.lock().expect("infer fixture test lock");
        // SAFETY: tests hold `INFER_FIXTURE_TEST_LOCK`; no concurrent access to these env keys.
        unsafe {
            std::env::set_var(INFER_STUB_BODY_ENV, body_json);
            std::env::set_var(INFER_STUB_ACK_ENV, "1");
        }
        Self { _lock: lock }
    }
}

impl Drop for InferFixtureEnvGuard {
    fn drop(&mut self) {
        unsafe {
            std::env::remove_var(INFER_STUB_BODY_ENV);
            std::env::remove_var(INFER_STUB_ACK_ENV);
        }
    }
}

pub struct OrchDaemonEnvGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl OrchDaemonEnvGuard {
    pub fn enter(socket: &str, writes_enabled: bool) -> Self {
        let lock = ORCH_DAEMON_ENV_TEST_LOCK
            .lock()
            .expect("orch daemon env lock");
        // SAFETY: tests hold `ORCH_DAEMON_ENV_TEST_LOCK`; no concurrent access to these env keys.
        unsafe {
            std::env::set_var("VOX_ORCHESTRATOR_DAEMON_SOCKET", socket);
            std::env::set_var(
                "VOX_MCP_ORCHESTRATOR_RPC_WRITES",
                if writes_enabled { "1" } else { "0" },
            );
        }
        Self { _lock: lock }
    }
}

impl Drop for OrchDaemonEnvGuard {
    fn drop(&mut self) {
        // SAFETY: tests hold `ORCH_DAEMON_ENV_TEST_LOCK`; no concurrent access to these env keys.
        unsafe {
            std::env::remove_var("VOX_ORCHESTRATOR_DAEMON_SOCKET");
            std::env::remove_var("VOX_MCP_ORCHESTRATOR_RPC_WRITES");
        }
    }
}
