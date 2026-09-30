use anyhow::Result;
use vox_cli_core::daemon_ipc::orchestrator_daemon_ensure::ping_existing_daemon;
use vox_foundation::protocol::orch_daemon_method;

/// `vox rollback` undoes an operation recorded in the **live**
/// `vox-orchestrator-d`'s in-memory operation log (`orch.undo_operation`),
/// not a `vox-bounded-fs` ledger (`vox-bounded-fs` is a size-capped read
/// helper with no undo/ledger concept).
///
/// Deliberately does **not** auto-spawn a daemon (unlike most other
/// daemon-routed CLI commands): a freshly spawned `vox-orchestrator-d` starts
/// with an empty operation log, so it can never contain the operation the
/// caller wants undone. Spawning one anyway used to (a) do nothing useful for
/// this command and (b) unconditionally rewrite the shared
/// `~/.vox/run/orchestrator-daemon.token` file at startup, which could rotate
/// the token out from under an unrelated long-lived daemon a GUI session was
/// using concurrently. See `crates/vox-orchestrator-d/src/bin/vox_orchestrator_d.rs`.
pub async fn run(id: Option<String>) -> Result<()> {
    let target = match id {
        Some(t) => t,
        None => {
            anyhow::bail!("No target ID specified. Please specify an operation ID to rollback.");
        }
    };

    let client = ping_existing_daemon().await.ok_or_else(|| {
        anyhow::anyhow!(
            "no daemon running: `vox rollback` needs the live vox-orchestrator-d that recorded \
             operation `{target}` — a freshly spawned daemon would have an empty operation log \
             and nothing to undo. Start it first (e.g. `vox mcp`, or open the GUI), then retry."
        )
    })?;

    println!("Rolling back operation {}...", target);

    let res = client
        .call(
            orch_daemon_method::UNDO_OPERATION,
            serde_json::json!({ "op_id": target }),
        )
        .await?;

    println!("Rollback complete: {:?}", res);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RED test: with no daemon reachable, `run` must fail fast with a clear
    /// "no daemon running" error — and, implicitly, without spawning one
    /// (there is nothing at `127.0.0.1:1`, a privileged port, for a spawn
    /// attempt to succeed against). Before this fix `run` always spawned a
    /// fresh, empty `vox-orchestrator-d` here instead.
    #[tokio::test]
    #[serial_test::serial(orchestrator_daemon_socket_env)]
    async fn no_daemon_running_fails_without_spawning() {
        // SAFETY: serialized via `#[serial_test::serial]` on every test that
        // mutates this env var.
        #[allow(unsafe_code)]
        unsafe {
            std::env::set_var("VOX_ORCHESTRATOR_DAEMON_SOCKET", "127.0.0.1:1");
        }

        let err = run(Some("op-123".to_string()))
            .await
            .expect_err("run must fail when no daemon is reachable");

        #[allow(unsafe_code)]
        unsafe {
            std::env::remove_var("VOX_ORCHESTRATOR_DAEMON_SOCKET");
        }

        assert!(
            err.to_string().contains("no daemon running"),
            "expected a clear 'no daemon running' error, got: {err}"
        );
    }

    #[tokio::test]
    async fn missing_id_bails_before_touching_the_daemon() {
        let err = run(None)
            .await
            .expect_err("run must fail when no id is given");
        assert!(err.to_string().contains("No target ID specified"));
    }
}
