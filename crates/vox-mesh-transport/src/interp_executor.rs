//! Bounded VoxScript child: interpreter isolation plus a process boundary.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::Result;
use iroh::EndpointId;
use tokio::io::AsyncReadExt;
use tokio::sync::{Semaphore, oneshot};
use vox_mesh_types::TaskKind;

use crate::caps_spec::CapsSpec;
use crate::endpoint::{JobExecutor, ReceivedJob};
use crate::protocol::{JobId, JobLimits, JobRequest, JobResponse};
use crate::trust::{MeshTrust, TrustLevel};

/// Curated host env preserved across `env_clear()` so `vox`/the runtime still
/// works; everything else (including this process's own secrets) is stripped.
///
/// // vox:defactored-from vox-orchestrator 2026-09-07
fn baseline_passthrough_env() -> Vec<(String, String)> {
    const KEYS: &[&str] = &[
        "PATH",
        "HOME",
        "TMPDIR",
        "TMP",
        "TEMP",
        "USER",
        "LOGNAME",
        "LANG",
        "LC_ALL",
        "LD_LIBRARY_PATH",
        "DYLD_LIBRARY_PATH",
        // Windows essentials:
        "SystemRoot",
        "windir",
        "SystemDrive",
        "USERPROFILE",
        "APPDATA",
        "LOCALAPPDATA",
        "PATHEXT",
        "ComSpec",
        "NUMBER_OF_PROCESSORS",
        "PROCESSOR_ARCHITECTURE",
        "OS",
    ];
    KEYS.iter()
        .filter_map(|k| std::env::var(k).ok().map(|v| ((*k).to_string(), v)))
        .collect()
}

enum Done {
    Exit(std::process::ExitStatus),
    Timeout,
    Cancel,
}

/// Runs mesh-received VoxScript as `vox run --mode interp` under trust-mapped caps.
pub struct InterpExecutor {
    trust: Arc<MeshTrust>,
    vox: PathBuf,
    limits: JobLimits,
    global: Arc<Semaphore>,
    peer_slots: Mutex<HashMap<EndpointId, Arc<Semaphore>>>,
    running: Mutex<HashMap<(EndpointId, JobId), oneshot::Sender<()>>>,
}

impl InterpExecutor {
    pub fn new(trust: Arc<MeshTrust>, vox: PathBuf, limits: JobLimits) -> Self {
        let n = limits.max_concurrent.max(1) as usize;
        Self {
            trust,
            vox,
            limits,
            global: Arc::new(Semaphore::new(n)),
            peer_slots: Mutex::new(HashMap::new()),
            running: Mutex::new(HashMap::new()),
        }
    }

    /// Trust-mapped `--caps` tokens for a job directory. Never grants `secrets`.
    pub fn caps_for(level: TrustLevel, job_dir: &Path) -> anyhow::Result<Vec<String>> {
        let extra: &[&str] = match level {
            TrustLevel::Sandboxed => &["time:real"],
            TrustLevel::Native => &["time:real", "net:allow", "process:allow", "env:ro"],
        };
        Ok(CapsSpec::from_roots(vec![], vec![job_dir.to_path_buf()], extra)?.to_tokens())
    }

    pub async fn read_capped<R: tokio::io::AsyncRead + Unpin>(
        mut r: R,
        max: usize,
    ) -> (Vec<u8>, bool) {
        let mut buf = Vec::new();
        let mut truncated = false;
        let mut chunk = [0u8; 8192];
        loop {
            match r.read(&mut chunk).await {
                Ok(0) => break,
                Ok(n) => {
                    let would = buf.len().saturating_add(n);
                    if would > max {
                        truncated = true;
                        if buf.len() < max {
                            buf.extend_from_slice(&chunk[..max - buf.len()]);
                        }
                    } else {
                        buf.extend_from_slice(&chunk[..n]);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(_) => break,
            }
        }
        (buf, truncated)
    }

    pub async fn read_capped_for_test<R: tokio::io::AsyncRead + Unpin>(
        r: R,
        max: usize,
    ) -> (Vec<u8>, bool) {
        Self::read_capped(r, max).await
    }

    fn peer_sem(&self, peer: EndpointId) -> Arc<Semaphore> {
        self.peer_slots
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entry(peer)
            .or_insert_with(|| Arc::new(Semaphore::new(1)))
            .clone()
    }

    fn pending_count(&self) -> u64 {
        self.running.lock().unwrap_or_else(|e| e.into_inner()).len() as u64
    }

    #[cfg(test)]
    fn insert_running_for_test(&self, peer: EndpointId, job_id: JobId) {
        let (tx, _rx) = oneshot::channel();
        self.running
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert((peer, job_id), tx);
    }

    async fn run_script(&self, peer: EndpointId, job_id: JobId, payload: &[u8]) -> JobResponse {
        let start = Instant::now();
        let (cancel_tx, cancel_rx) = oneshot::channel();
        {
            let mut running = self.running.lock().unwrap_or_else(|e| e.into_inner());
            if running.contains_key(&(peer, job_id)) {
                return JobResponse::Failed("already running".to_string());
            }
            running.insert((peer, job_id), cancel_tx);
        }

        let peer_sem = self.peer_sem(peer);
        let Ok(_peer_permit) = peer_sem.try_acquire() else {
            self.running
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&(peer, job_id));
            return JobResponse::Failed("retry later: at the per-peer concurrency cap".to_string());
        };
        let Ok(_global_permit) = self.global.try_acquire() else {
            self.running
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&(peer, job_id));
            return JobResponse::Failed("retry later: at the node concurrency cap".to_string());
        };

        let result = self.spawn_and_wait(peer, job_id, payload, cancel_rx).await;
        self.running
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&(peer, job_id));
        let exit = match &result {
            JobResponse::Output(_) => 0,
            JobResponse::Failed(m) if m.contains("capability denied") => 77,
            JobResponse::Failed(m) if m.contains("execution budget") => 78,
            JobResponse::Failed(m) if m.contains("memory limit") => 79,
            JobResponse::Failed(m) if m.contains("interpreter bug") => 101,
            _ => -1,
        };
        tracing::info!(
            peer = %peer,
            job_id = job_id.0,
            exit,
            elapsed = ?start.elapsed(),
            "mesh job finished"
        );
        result
    }

    async fn spawn_and_wait(
        &self,
        peer: EndpointId,
        _job_id: JobId,
        payload: &[u8],
        cancel_rx: oneshot::Receiver<()>,
    ) -> JobResponse {
        let job_dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(e) => return JobResponse::Failed(format!("job dir: {e}")),
        };
        let home_dir = match tempfile::tempdir() {
            Ok(d) => d,
            Err(e) => return JobResponse::Failed(format!("home dir: {e}")),
        };
        let script = job_dir.path().join("job.vox");
        if let Err(e) = std::fs::write(&script, payload) {
            return JobResponse::Failed(format!("write script: {e}"));
        }

        let level = self.trust.level(&peer).unwrap_or(TrustLevel::Sandboxed);
        let tokens = match Self::caps_for(level, job_dir.path()) {
            Ok(t) => t,
            Err(e) => return JobResponse::Failed(e.to_string()),
        };

        let mut cmd = tokio::process::Command::new(&self.vox);
        cmd.arg("run")
            .arg("--mode")
            .arg("interp")
            .arg("--max-steps")
            .arg(self.limits.max_steps.to_string())
            .arg("--max-memory")
            .arg(self.limits.max_memory_bytes.to_string())
            .arg("--max-disk")
            .arg(self.limits.max_disk_bytes.to_string())
            .arg("--max-files")
            .arg(self.limits.max_files.to_string())
            .arg("--max-depth")
            .arg(self.limits.max_depth.to_string());
        for tok in &tokens {
            cmd.arg("--caps").arg(tok);
        }
        cmd.arg(&script)
            .current_dir(job_dir.path())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .env_clear();
        for (k, v) in baseline_passthrough_env() {
            cmd.env(k, v);
        }
        #[cfg(unix)]
        {
            cmd.env("HOME", home_dir.path());
            cmd.env("TMPDIR", job_dir.path());
            cmd.process_group(0);
        }
        #[cfg(windows)]
        {
            cmd.env("USERPROFILE", home_dir.path());
            cmd.env("TEMP", job_dir.path());
            cmd.env("TMP", job_dir.path());
        }

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => return JobResponse::Failed(format!("spawn vox: {e}")),
        };

        #[cfg(windows)]
        let win_job = assign_win32_job(&child);

        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let out_max = self.limits.max_output_bytes;
        let out_task = tokio::spawn(async move {
            match stdout {
                Some(r) => InterpExecutor::read_capped(r, out_max).await,
                None => (Vec::new(), false),
            }
        });
        let err_task = tokio::spawn(async move {
            match stderr {
                Some(r) => InterpExecutor::read_capped(r, 64 * 1024).await,
                None => (Vec::new(), false),
            }
        });

        let done = tokio::select! {
            s = child.wait() => match s {
                Ok(st) => Done::Exit(st),
                Err(e) => {
                    out_task.abort();
                    err_task.abort();
                    return JobResponse::Failed(format!("wait: {e}"));
                }
            },
            _ = tokio::time::sleep(self.limits.wall_clock) => Done::Timeout,
            Ok(()) = cancel_rx => Done::Cancel,
        };

        if matches!(done, Done::Timeout | Done::Cancel) {
            out_task.abort();
            err_task.abort();
            #[cfg(unix)]
            kill_process_group(&child);
            #[cfg(windows)]
            {
                drop(win_job);
            }
            let _ = child.start_kill();
            let _ = child.wait().await;
            return JobResponse::Failed(match done {
                Done::Timeout => "wall clock exceeded".to_string(),
                Done::Cancel => "cancelled".to_string(),
                Done::Exit(_) => unreachable!(),
            });
        }

        #[cfg(windows)]
        let _ = win_job;

        let (mut stdout, truncated) = out_task.await.unwrap_or((Vec::new(), false));
        let (stderr, _) = err_task.await.unwrap_or((Vec::new(), false));
        if truncated {
            stdout.extend_from_slice(b"\n[vox: output truncated]\n");
        }

        let Done::Exit(status) = done else {
            unreachable!()
        };
        map_exit(status, stdout, stderr)
    }
}

#[cfg(unix)]
#[allow(unsafe_code)]
fn kill_process_group(child: &tokio::process::Child) {
    if let Some(pid) = child.id() {
        // SAFETY: the child was spawned with process_group(0), so pid is the pgid.
        unsafe {
            libc::killpg(pid as i32, libc::SIGKILL);
        }
    }
}

#[cfg(windows)]
fn assign_win32_job(child: &tokio::process::Child) -> Option<win32job::Job> {
    use std::os::windows::io::AsRawHandle;
    let mut info = win32job::ExtendedLimitInfo::new();
    info.limit_kill_on_job_close();
    let job = win32job::Job::create_with_limit_info(&info).ok()?;
    let handle = child.as_raw_handle() as isize;
    job.assign_process(handle).ok()?;
    Some(job)
}

fn map_exit(status: std::process::ExitStatus, stdout: Vec<u8>, stderr: Vec<u8>) -> JobResponse {
    let err = String::from_utf8_lossy(&stderr);
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if status.signal().is_some() {
            return JobResponse::Failed("killed by signal (stack overflow or OOM)".to_string());
        }
    }
    #[cfg(windows)]
    {
        if let Some(code) = status.code() {
            let u = code as u32;
            const STATUS_STACK_OVERFLOW: u32 = 0xC00000FD;
            if u == STATUS_STACK_OVERFLOW {
                return JobResponse::Failed("STATUS_STACK_OVERFLOW".to_string());
            }
            if u & 0x8000_0000 != 0 {
                return JobResponse::Failed(format!("NTSTATUS {u:#010x}"));
            }
        }
    }
    match status.code() {
        Some(0) => JobResponse::Output(stdout),
        Some(77) if err.contains("capability denied") => JobResponse::Failed(err.into_owned()),
        Some(77) => JobResponse::Failed(format!("process exited 77: {err}")),
        Some(78) => JobResponse::Failed(format!("execution budget exceeded: {err}")),
        Some(79) => JobResponse::Failed(format!("memory limit exceeded: {err}")),
        Some(101) => JobResponse::Failed("interpreter bug".to_string()),
        Some(code) => JobResponse::Failed(format!("exit {code}: {err}")),
        None => JobResponse::Failed("process exited without a status".to_string()),
    }
}

impl JobExecutor for InterpExecutor {
    fn execute<'a>(
        &'a self,
        job: ReceivedJob,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<JobResponse>> + Send + 'a>> {
        Box::pin(async move {
            Ok(match job.request {
                JobRequest::Probe => JobResponse::Probed {
                    host_triple: format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS),
                    vox: env!("CARGO_PKG_VERSION").to_string(),
                    task_kinds: vec![TaskKind::VoxScript],
                    engines: Vec::new(),
                },
                JobRequest::QueueStats => JobResponse::QueueStats(crate::protocol::QueueStats {
                    pending_count: self.pending_count(),
                    pending_by_kind: Vec::new(),
                    pending_by_priority: Vec::new(),
                    max_concurrent: u64::from(self.limits.max_concurrent),
                }),
                JobRequest::Cancel { job_id } => {
                    let tx = self
                        .running
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .remove(&(job.peer, job_id));
                    match tx {
                        Some(tx) => {
                            let _ = tx.send(());
                            JobResponse::Failed("cancelled".to_string())
                        }
                        None => JobResponse::Failed("no such running job".to_string()),
                    }
                }
                JobRequest::Run { job_id, kind, .. } => {
                    if kind != TaskKind::VoxScript {
                        JobResponse::Failed(format!("no engine installed for {}", kind.as_str()))
                    } else {
                        self.run_script(job.peer, job_id, &job.payload).await
                    }
                }
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::endpoint::ReceivedJob;

    #[test]
    fn new_constructs_with_the_given_binary() {
        let d = tempfile::tempdir().unwrap();
        let trust = Arc::new(MeshTrust::at(&d.path().join("mesh_trust.json")));
        let exec = InterpExecutor::new(trust, PathBuf::from("vox"), JobLimits::default());
        assert_eq!(exec.vox, PathBuf::from("vox"));
        assert!(exec.limits.max_concurrent >= 2);
    }

    #[test]
    fn caps_for_sandboxed_does_not_grant_native_extras() {
        let d = tempfile::tempdir().unwrap();
        let tokens = InterpExecutor::caps_for(TrustLevel::Sandboxed, d.path()).unwrap();
        let s = tokens.join(",");
        assert!(s.contains("fs:rw=") && s.contains("time:real"), "{s}");
        for forbidden in ["net:allow", "process:allow", "env:", "secrets"] {
            assert!(!s.contains(forbidden), "{s}");
        }
    }

    #[test]
    fn caps_for_native_adds_net_process_env_but_not_secrets() {
        let d = tempfile::tempdir().unwrap();
        let n = InterpExecutor::caps_for(TrustLevel::Native, d.path())
            .unwrap()
            .join(",");
        assert!(n.contains("net:allow") && n.contains("process:allow") && n.contains("env:ro"));
        assert!(!n.contains("secrets"));
    }

    #[test]
    fn exit_79_with_stdout_is_still_a_memory_failure() {
        #[cfg(unix)]
        let status = {
            use std::os::unix::process::ExitStatusExt;
            std::process::ExitStatus::from_raw(79 << 8)
        };
        #[cfg(windows)]
        let status = {
            use std::os::windows::process::ExitStatusExt;
            std::process::ExitStatus::from_raw(79)
        };
        let response = map_exit(
            status,
            b"completed output\n".to_vec(),
            b"vox: memory limit exceeded\n".to_vec(),
        );
        assert!(
            matches!(response, JobResponse::Failed(ref m) if m.contains("memory limit exceeded")),
            "{response:?}"
        );
    }

    #[tokio::test]
    async fn read_capped_exact_max_is_not_truncated() {
        let exact = [b'x'; 64];
        let (buf, truncated) = InterpExecutor::read_capped(&exact[..], 64).await;
        assert_eq!(buf.len(), 64);
        assert!(!truncated);
    }

    #[tokio::test]
    async fn cancel_is_keyed_by_peer_not_just_job_id() {
        let d = tempfile::tempdir().unwrap();
        let trust = Arc::new(MeshTrust::at(&d.path().join("mesh_trust.json")));
        let exec = InterpExecutor::new(trust, PathBuf::from("vox"), JobLimits::default());
        let a = iroh::SecretKey::from_bytes(&[1u8; 32]).public();
        let b = iroh::SecretKey::from_bytes(&[2u8; 32]).public();
        exec.insert_running_for_test(a, JobId(9));
        let resp = exec
            .execute(ReceivedJob {
                peer: b,
                request: JobRequest::Cancel { job_id: JobId(9) },
                limits: JobLimits::default(),
                payload: Vec::new(),
            })
            .await
            .unwrap();
        assert!(
            matches!(resp, JobResponse::Failed(ref m) if m.contains("no such running job")),
            "{resp:?}"
        );
    }

    #[tokio::test]
    async fn read_capped_over_max_is_truncated() {
        let over = [b'x'; 65];
        let (buf, truncated) = InterpExecutor::read_capped(&over[..], 64).await;
        assert_eq!(buf.len(), 64);
        assert!(truncated);
    }
}
