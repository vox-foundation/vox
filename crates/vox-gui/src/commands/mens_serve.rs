//! `vox mens serve` supervisor: a detached-child-process pattern for
//! `vox-ml-cli mens serve`, modeled on [`super::daemon::PersistentDaemon`].
//!
//! `vox mens serve` never exits (it's a long-lived HTTP server), so it cannot
//! go through `execute_command` (`.output().await` waits for process exit —
//! see `CommandCardsView`'s doc comment for why that component is the wrong
//! place for this). This module instead: resolves the `vox-ml-cli` sidecar via
//! [`resolve_managed_binary_path`] (the same helper Task C1 fixed
//! `crates/vox-cli/src/main.rs` to use), spawns it detached with stderr
//! redirected to `~/.vox/run/mens-serve.stderr.log`, holds the [`Child`] in a
//! slot, polls readiness by reusing [`llm_settings::probe_vox_local_bases`]
//! (the exact same health-check logic `inference_provider_status` already
//! uses for the VoxLocal row — not a second probe), and refuses to start a
//! second instance while one is already held (an unsafe restart would either
//! fail to bind the port or double-load a model into memory).

use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

use serde::Serialize;
use vox_cli_core::daemon_ipc::process_supervision::resolve_managed_binary_path;
use vox_config::timeouts::{D_60S, D_200MS};

use super::process_util::quiet_command;

/// Port `vox mens serve` binds when the caller doesn't specify one — mirrors
/// `vox_config::inference::VOX_LOCAL_ENDPOINT_OLLAMA_CONFLICT_ALT`
/// (`http://127.0.0.1:11435`), the alternate loopback port used when Ollama
/// already holds `:11434`. Kept as a parsed `u16` here (rather than adding a
/// bare-port constant to `vox-config`, which is out of scope for this task —
/// see Task E3) so the frontend never hardcodes the literal itself: it reads
/// this via [`MensServeStatusDto::default_port`] from [`mens_serve_status`].
fn default_port() -> u16 {
    vox_config::inference::VOX_LOCAL_ENDPOINT_OLLAMA_CONFLICT_ALT
        .rsplit(':')
        .next()
        .and_then(|p| p.parse().ok())
        .unwrap_or(11435)
}

/// Well-known file `vox mens serve`'s stderr is redirected to, mirroring
/// `daemon.rs`'s `~/.vox/run/orchestrator-daemon.stderr.log` convention.
fn stderr_log_path() -> std::path::PathBuf {
    vox_config::paths::user_home_dir()
        .join(".vox")
        .join("run")
        .join("mens-serve.stderr.log")
}

/// Tauri-managed holder for the (at most one) `vox-ml-cli mens serve` child
/// this GUI has spawned.
#[derive(Default)]
pub struct MensServeState {
    child: Mutex<Option<Child>>,
    port: Mutex<Option<u16>>,
}

impl MensServeState {
    /// Whether a spawned child is still alive, self-healing the slot if the
    /// process has exited on its own (crash, model load failure, `kill -9`
    /// from outside the GUI) so a stale `running: true` never lingers.
    fn is_running(&self) -> bool {
        let Ok(mut slot) = self.child.lock() else {
            return false;
        };
        match slot.as_mut() {
            None => false,
            Some(child) => match child.try_wait() {
                Ok(Some(_exit_status)) => {
                    *slot = None;
                    if let Ok(mut port) = self.port.lock() {
                        *port = None;
                    }
                    false
                }
                Ok(None) => true,
                // Can't determine status; assume still running rather than
                // silently dropping a live handle.
                Err(_) => true,
            },
        }
    }

    fn current_port(&self) -> Option<u16> {
        self.port.lock().ok().and_then(|p| *p)
    }

    fn status(&self) -> MensServeStatusDto {
        MensServeStatusDto {
            running: self.is_running(),
            port: self.current_port(),
            default_port: default_port(),
        }
    }

    /// Core start logic: refuses an unsafe restart (already running), spawns
    /// via `spawn`, holds the child, then polls readiness against `port`
    /// until [`llm_settings::probe_vox_local_bases`] reports the server
    /// reachable or `D_60S` elapses (model load can be slow). `spawn` is
    /// injected so tests can stand in a lightweight process instead of the
    /// real `vox-ml-cli` binary (mirrors `daemon.rs`'s tests binding a real
    /// in-process listener instead of spawning `vox-orchestrator-d`).
    async fn start_with(
        &self,
        port: u16,
        spawn: impl FnOnce() -> std::io::Result<Child>,
    ) -> Result<(), String> {
        if self.is_running() {
            return Err("mens serve is already running; stop it before restarting".to_string());
        }
        let child = spawn().map_err(|e| format!("failed to spawn vox-ml-cli: {e}"))?;
        if let Ok(mut slot) = self.child.lock() {
            *slot = Some(child);
        }
        if let Ok(mut p) = self.port.lock() {
            *p = Some(port);
        }

        let base = format!("http://127.0.0.1:{port}");
        let deadline = std::time::Instant::now() + D_60S;
        loop {
            let (reachable, _models) =
                super::llm_settings::probe_vox_local_bases(std::slice::from_ref(&base)).await;
            if reachable == Some(true) {
                return Ok(());
            }
            if std::time::Instant::now() >= deadline {
                self.stop();
                return Err(format!(
                    "vox-ml-cli mens serve did not become ready on port {port} within 60s"
                ));
            }
            tokio::time::sleep(D_200MS).await;
        }
    }

    /// Kill the held child, if any. Returns whether a child was actually
    /// stopped (idempotent: calling this with nothing running is a no-op).
    fn stop(&self) -> bool {
        let Ok(mut slot) = self.child.lock() else {
            return false;
        };
        let Some(mut child) = slot.take() else {
            return false;
        };
        let _ = child.kill();
        let _ = child.wait();
        if let Ok(mut port) = self.port.lock() {
            *port = None;
        }
        true
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct MensServeStatusDto {
    pub running: bool,
    pub port: Option<u16>,
    /// SSOT default port for the frontend's port field — see [`default_port`].
    pub default_port: u16,
}

fn build_serve_command(model: &str, port: u16) -> Command {
    let binary = resolve_managed_binary_path("vox-ml-cli");
    let mut cmd = quiet_command(binary);
    cmd.args(["mens", "serve", "--model", model, "--port"])
        .arg(port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null());
    let stderr_path = stderr_log_path();
    if let Some(parent) = stderr_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&stderr_path)
    {
        Ok(file) => {
            cmd.stderr(Stdio::from(file));
        }
        Err(_) => {
            cmd.stderr(Stdio::null());
        }
    }
    cmd
}

#[tauri::command]
pub async fn mens_serve_status(
    state: tauri::State<'_, std::sync::Arc<MensServeState>>,
) -> Result<MensServeStatusDto, String> {
    Ok(state.status())
}

#[tauri::command]
pub async fn mens_serve_start(
    state: tauri::State<'_, std::sync::Arc<MensServeState>>,
    model: String,
    port: Option<u16>,
) -> Result<MensServeStatusDto, String> {
    let port = port.unwrap_or_else(default_port);
    state
        .start_with(port, move || build_serve_command(&model, port).spawn())
        .await?;
    Ok(state.status())
}

#[tauri::command]
pub async fn mens_serve_stop(
    state: tauri::State<'_, std::sync::Arc<MensServeState>>,
) -> Result<MensServeStatusDto, String> {
    state.stop();
    Ok(state.status())
}

#[cfg(test)]
mod tests {
    //! RED tests for the mens-serve supervisor. Mirrors `daemon.rs`'s test
    //! style: exercise `MensServeState`'s real async logic directly (a real
    //! spawned OS process + a real bound TCP listener answering the exact
    //! health contract `vox_local_health_identifies_serve` checks), rather
    //! than the `#[tauri::command]` wrappers (which need a live Tauri
    //! `AppHandle` to construct `State<'_, _>` from) or the real `vox-ml-cli`
    //! binary (which needs a trained checkpoint + GPU plugin — infeasible in
    //! a unit test sandbox).
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// A long-running, harmless child process any dev machine has, standing
    /// in for the real `vox-ml-cli mens serve` process so `start_with` has a
    /// genuine OS process to hold and later kill.
    fn spawn_dummy_child() -> std::io::Result<Child> {
        #[cfg(unix)]
        {
            Command::new("sleep")
                .arg("60")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
        }
        #[cfg(windows)]
        {
            Command::new("cmd")
                .args(["/C", "timeout", "/T", "60", "/NOBREAK"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
        }
    }

    /// Binds a real TCP listener on an ephemeral port and answers every
    /// request on a background thread with the exact `/health` body
    /// `vox_local_health_identifies_serve` requires
    /// (`{"service":"vox-ml-cli"}`) — the real contract `probe_vox_local_bases`
    /// checks, not a mocked function.
    fn spawn_fake_health_server() -> u16 {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let port = listener.local_addr().expect("local addr").port();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { break };
                let mut buf = [0u8; 512];
                let _ = stream.read(&mut buf);
                let body = r#"{"status":"ok","service":"vox-ml-cli"}"#;
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(resp.as_bytes());
            }
        });
        port
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn start_spawns_a_real_child_and_reaches_ready() {
        let state = MensServeState::default();
        let port = spawn_fake_health_server();

        state
            .start_with(port, spawn_dummy_child)
            .await
            .expect("start_with should report ready once the health server answers");

        assert!(state.is_running(), "a spawned child must be held");
        assert_eq!(state.current_port(), Some(port));

        // Cleanup: stop, don't leak the dummy child past this test.
        assert!(state.stop());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn start_refuses_a_second_spawn_while_one_is_running() {
        let state = MensServeState::default();
        let port = spawn_fake_health_server();
        state
            .start_with(port, spawn_dummy_child)
            .await
            .expect("first start should succeed");

        let second_port = spawn_fake_health_server();
        let result = state.start_with(second_port, spawn_dummy_child).await;
        assert!(
            result.is_err(),
            "starting again while already running must be refused, not spawn a second child"
        );

        assert!(state.stop());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn stop_kills_the_held_child() {
        let state = MensServeState::default();
        let port = spawn_fake_health_server();
        state
            .start_with(port, spawn_dummy_child)
            .await
            .expect("start should succeed");
        assert!(state.is_running());

        let stopped = state.stop();
        assert!(
            stopped,
            "stop() must report that a child was actually stopped"
        );
        assert!(!state.is_running(), "the slot must be empty after stop()");

        // stop() is idempotent: nothing to stop the second time.
        assert!(!state.stop());
    }

    #[test]
    fn build_serve_command_shape_matches_the_real_cli() {
        // Regression for the exact argv `vox-ml-cli mens serve` expects
        // (crates/vox-ml-cli/src/commands/mens/populi/action_populi_enum.rs's
        // `Serve` variant: `--model <path>` then `--port <n>`).
        let cmd = build_serve_command("/tmp/some-run-dir", 11435);
        let args: Vec<String> = cmd
            .get_args()
            .map(|a| a.to_string_lossy().to_string())
            .collect();
        assert_eq!(
            args,
            vec![
                "mens",
                "serve",
                "--model",
                "/tmp/some-run-dir",
                "--port",
                "11435"
            ]
        );
    }

    #[test]
    fn default_port_matches_the_ollama_conflict_alt_constant() {
        assert_eq!(default_port(), 11435);
    }
}
