//! One lazy worker. Requests are serialized. A crash, timeout, or kill bumps
//! the generation so handles from the previous process are rejected.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use serde_json::Value;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout};
use tokio::time::Instant;
use vox_foundation::protocol::{DispatchPayload, DispatchRequest, DispatchResponse};

use crate::frame::{read_frame, write_frame};
use crate::{BridgeConfig, BridgeError, HandleRef, Reply};

const INTERRUPT_GRACE: Duration = Duration::from_secs(5);

struct LiveWorker {
    child: Child,
    stdin: ChildStdin,
    stdout: ChildStdout,
    stderr: ChildStderr,
}

struct Inner {
    config: BridgeConfig,
    live: Option<LiveWorker>,
    next_id: u64,
}

/// One lazy worker. Requests are serialized and handles carry a generation.
pub struct Bridge {
    inner: tokio::sync::Mutex<Inner>,
    generation: AtomicU32,
    pending_release: Mutex<Vec<u64>>,
    stderr: Mutex<String>,
}

impl Bridge {
    pub fn new(config: BridgeConfig) -> Self {
        Self {
            inner: tokio::sync::Mutex::new(Inner {
                config,
                live: None,
                next_id: 1,
            }),
            generation: AtomicU32::new(0),
            pending_release: Mutex::new(Vec::new()),
            stderr: Mutex::new(String::new()),
        }
    }

    pub fn generation(&self) -> u32 {
        self.generation.load(Ordering::SeqCst)
    }

    /// Queue handle ids. The next [`Self::call`] sends them as one `release`.
    pub fn enqueue_release(&self, id: u64) {
        self.pending_release.lock().expect("release lock").push(id);
    }

    /// Stderr captured from the worker since spawn. `debug.print` lands here.
    pub fn stderr_text(&self) -> String {
        self.stderr.lock().expect("stderr lock").clone()
    }

    pub async fn call(
        &self,
        method: &str,
        params: Value,
        body: Option<&[u8]>,
    ) -> Result<Reply, BridgeError> {
        let mut inner = self.inner.lock().await;
        self.reap_if_exited(&mut inner);
        if let Some(message) = stale_message(&params, self.generation()) {
            return Err(BridgeError::protocol(message));
        }
        self.ensure_spawned(&mut inner).await?;

        let pending = std::mem::take(&mut *self.pending_release.lock().expect("release lock"));
        let params = if pending.is_empty() {
            params
        } else if method == "release" {
            merge_release_ids(params, pending)
        } else {
            self.round_trip(
                &mut inner,
                "release",
                serde_json::json!({ "ids": pending }),
                None,
            )
            .await?;
            params
        };

        let reply = self.round_trip(&mut inner, method, params, body).await?;
        self.drain_stderr(&mut inner).await;
        Ok(reply)
    }

    fn reap_if_exited(&self, inner: &mut Inner) {
        let exited = inner
            .live
            .as_mut()
            .and_then(|live| live.child.try_wait().ok().flatten())
            .is_some();
        if exited {
            self.retire(inner);
        }
    }

    async fn ensure_spawned(&self, inner: &mut Inner) -> Result<(), BridgeError> {
        if inner.live.is_some() {
            return Ok(());
        }
        if self.generation.load(Ordering::SeqCst) == 0 {
            self.generation.store(1, Ordering::SeqCst);
        }
        let program = inner
            .config
            .worker_cmd
            .first()
            .cloned()
            .ok_or_else(|| BridgeError::protocol("py: worker command is empty"))?;
        let mut command = tokio::process::Command::new(program);
        command.args(inner.config.worker_cmd.iter().skip(1));
        command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
        for (key, value) in &inner.config.env {
            command.env(key, value);
        }
        #[cfg(windows)]
        {
            const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
            command.creation_flags(CREATE_NEW_PROCESS_GROUP);
        }
        let mut child = command.spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| std::io::Error::other("worker stdin was not piped"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| std::io::Error::other("worker stdout was not piped"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| std::io::Error::other("worker stderr was not piped"))?;
        inner.live = Some(LiveWorker {
            child,
            stdin,
            stdout,
            stderr,
        });
        Ok(())
    }

    async fn round_trip(
        &self,
        inner: &mut Inner,
        method: &str,
        params: Value,
        body: Option<&[u8]>,
    ) -> Result<Reply, BridgeError> {
        let id = inner.next_id.to_string();
        inner.next_id = inner.next_id.saturating_add(1);
        let request = DispatchRequest {
            id: id.clone(),
            method: method.to_string(),
            params,
            auth_token: None,
            permission_mode: None,
        };
        let header = serde_json::to_vec(&request)
            .map_err(|err| BridgeError::protocol(format!("py: request encode: {err}")))?;
        let timeout = inner.config.timeout;
        let max_body = inner
            .config
            .max_fetch_mb
            .saturating_mul(1024)
            .saturating_mul(1024);
        {
            let live = inner
                .live
                .as_mut()
                .ok_or_else(|| BridgeError::protocol("py: worker is not running"))?;
            write_frame(&mut live.stdin, &header, body).await?;
        }
        let read_result = {
            let live = inner
                .live
                .as_mut()
                .ok_or_else(|| BridgeError::protocol("py: worker is not running"))?;
            tokio::time::timeout(timeout, read_frame(&mut live.stdout, max_body)).await
        };
        match read_result {
            Ok(Ok(Some((header, body)))) => parse_reply(&id, &header, body),
            Ok(Ok(None)) => {
                self.retire(inner);
                Err(BridgeError::protocol("py: worker exited"))
            }
            Ok(Err(err)) if is_fetch_cap(&err) => Err(err),
            Ok(Err(err)) => {
                self.retire(inner);
                Err(err)
            }
            Err(_elapsed) => {
                if let Some(live) = inner.live.as_mut() {
                    interrupt_and_reap(live).await;
                }
                self.retire(inner);
                Err(BridgeError::protocol("py: timeout"))
            }
        }
    }

    async fn drain_stderr(&self, inner: &mut Inner) {
        let Some(live) = inner.live.as_mut() else {
            return;
        };
        let mut buf = [0u8; 4096];
        loop {
            match tokio::time::timeout(Duration::from_millis(50), live.stderr.read(&mut buf)).await
            {
                Ok(Ok(0)) | Err(_) => break,
                Ok(Err(_)) => break,
                Ok(Ok(n)) => {
                    let chunk = String::from_utf8_lossy(&buf[..n]);
                    self.stderr.lock().expect("stderr lock").push_str(&chunk);
                }
            }
        }
    }

    fn retire(&self, inner: &mut Inner) {
        if inner.live.take().is_some() {
            self.generation.fetch_add(1, Ordering::SeqCst);
        }
    }
}

fn parse_reply(id: &str, header: &[u8], body: Option<Vec<u8>>) -> Result<Reply, BridgeError> {
    let response: DispatchResponse = serde_json::from_slice(header)
        .map_err(|err| BridgeError::protocol(format!("py: bad response: {err}")))?;
    if response.id != id {
        return Err(BridgeError::protocol("py: response id mismatch"));
    }
    match response.payload {
        DispatchPayload::Result { value } => Ok(Reply { value, body }),
        DispatchPayload::Error { message, .. } => Err(BridgeError::protocol(message)),
        other => Err(BridgeError::protocol(format!(
            "py: unexpected payload {other:?}"
        ))),
    }
}

fn is_fetch_cap(err: &BridgeError) -> bool {
    err.to_string().contains("py: fetch exceeds cap")
}

fn stale_message(params: &Value, generation: u32) -> Option<String> {
    let mut handles = Vec::new();
    collect_handles(params, &mut handles);
    if handles.iter().any(|handle| handle.generation != generation) {
        Some("py: stale handle (worker restarted)".to_string())
    } else {
        None
    }
}

fn collect_handles(value: &Value, out: &mut Vec<HandleRef>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_handles(item, out);
            }
        }
        Value::Object(map) => {
            if let (Some(generation), Some(id), Some(kind), Some(dtype)) = (
                map.get("generation").and_then(Value::as_u64),
                map.get("id").and_then(Value::as_u64),
                map.get("kind").and_then(Value::as_str),
                map.get("dtype").and_then(Value::as_str),
            ) && let Ok(generation) = u32::try_from(generation)
            {
                out.push(HandleRef {
                    generation,
                    id,
                    kind: kind.to_string(),
                    dtype: dtype.to_string(),
                });
            }
            for child in map.values() {
                collect_handles(child, out);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

async fn interrupt_and_reap(live: &mut LiveWorker) {
    interrupt(&live.child);
    let deadline = Instant::now() + INTERRUPT_GRACE;
    loop {
        match live.child.try_wait() {
            Ok(Some(_)) | Err(_) => return,
            Ok(None) if Instant::now() >= deadline => break,
            Ok(None) => tokio::time::sleep(Duration::from_millis(50)).await,
        }
    }
    let _ = live.child.start_kill();
    let _ = tokio::time::timeout(Duration::from_secs(2), live.child.wait()).await;
}

#[cfg(unix)]
#[allow(unsafe_code)]
fn interrupt(child: &Child) {
    let Some(pid) = child.id() else {
        return;
    };
    // SAFETY: `kill` targets only the child pid with SIGINT. It does not run
    // in a signal handler and does not touch process-shared memory.
    let _ = unsafe { libc::kill(pid as libc::pid_t, libc::SIGINT) };
}

fn merge_release_ids(mut params: Value, pending: Vec<u64>) -> Value {
    let queued = pending.into_iter().map(Value::from).collect::<Vec<_>>();
    match params.get_mut("ids").and_then(Value::as_array_mut) {
        Some(ids) => ids.extend(queued),
        None => {
            params["ids"] = Value::Array(queued);
        }
    }
    params
}

#[cfg(windows)]
#[allow(unsafe_code)]
fn interrupt(child: &Child) {
    let Some(pid) = child.id() else {
        return;
    };
    // SAFETY: the child was spawned in its own process group, so CTRL_BREAK
    // reaches that group and not this host.
    let _ = unsafe {
        windows_sys::Win32::System::Console::GenerateConsoleCtrlEvent(
            windows_sys::Win32::System::Console::CTRL_BREAK_EVENT,
            pid,
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_bridge_has_no_generation_and_empty_stderr() {
        let bridge = Bridge::new(crate::BridgeConfig::default());
        assert_eq!(bridge.generation(), 0);
        bridge.enqueue_release(4);
        assert!(bridge.stderr_text().is_empty());
    }
}
