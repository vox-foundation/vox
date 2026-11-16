//! Standalone sticky-cwd [`EphemeralTerminalBridge`] fallback with full ECMA-48 + CR
//! normalization, Clavis exact-needle scrubbing, and conservative vendor/PEM/path redaction.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use parking_lot::Mutex;
use serde_json::json;

use crate::oversight::exec_policy::{
    ExecPolicyV1, SecurityTier, default_exec_policy_v1, evaluate_command_ast,
    evaluate_security_tier,
};
use crate::oversight::floor::{FloorInput, classify_floor};

use super::{
    DEFAULT_HEAD_LINES, DEFAULT_MAX_BYTES, DEFAULT_TAIL_LINES, EPHEMERAL_PWD_MARKER,
    MAX_DAEMON_WAIT_MS, TermBlockDto, TermExecRequest, TermExecResponse, TermReadRequest,
    TermReadResponse, TermSendInputRequest, TermSessionSummary, TerminalBridge,
    bound_ephemeral_output, scrub_ephemeral_text,
};

struct EphemeralBlock {
    id: u64,
    input: String,
    full_scrubbed_output: String,
    exit_code: i32,
    pinned: bool,
    cwd: String,
}

struct EphemeralSession {
    cwd: PathBuf,
    next_block_id: u64,
    seq: u64,
    blocks: Vec<EphemeralBlock>,
}

/// Standalone fallback [`TerminalBridge`] used when `vox-orchestrator-d` is unreachable.
pub struct EphemeralTerminalBridge {
    default_cwd: PathBuf,
    next_session: Mutex<u64>,
    sessions: Mutex<HashMap<String, EphemeralSession>>,
}

impl Default for EphemeralTerminalBridge {
    fn default() -> Self {
        Self::new(None)
    }
}

impl EphemeralTerminalBridge {
    #[must_use]
    pub fn new(default_cwd: Option<PathBuf>) -> Self {
        let cwd = default_cwd
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        Self {
            default_cwd: cwd,
            next_session: Mutex::new(0),
            sessions: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl TerminalBridge for EphemeralTerminalBridge {
    async fn exec(&self, req: TermExecRequest) -> Result<TermExecResponse> {
        static POLICY: OnceLock<ExecPolicyV1> = OnceLock::new();
        let trimmed = req.command.trim();
        if trimmed.is_empty() || trimmed.contains(['\n', '\r']) {
            anyhow::bail!("ephemeral terminal rejected empty or multi-line command");
        }
        if req.wait_ms == 0 {
            anyhow::bail!(
                "ephemeral terminal fallback cannot run background commands (wait_ms=0) without a persistent daemon PTY session"
            );
        }
        let policy = POLICY.get_or_init(default_exec_policy_v1);
        for v in evaluate_command_ast(trimmed, policy) {
            if v.rule != "command-not-allowed" {
                anyhow::bail!("exec policy rejected command ({}): {}", v.rule, v.message);
            }
        }
        let cur_cwd = {
            let mut map = self.sessions.lock();
            let sess = map
                .entry(req.session_id.clone())
                .or_insert_with(|| EphemeralSession {
                    cwd: self.default_cwd.clone(),
                    next_block_id: 1,
                    seq: 0,
                    blocks: Vec::new(),
                });
            sess.cwd.clone()
        };
        if !req.hitl_approved
            && let Some(cat) = classify_floor(&FloorInput::tool(
                "vox_terminal_exec",
                &json!({ "command": trimmed }),
                &cur_cwd,
            ))
        {
            anyhow::bail!(
                "safety floor ({}) requires explicit HITL approval",
                cat.id()
            );
        }
        let allowed_tier = if req.hitl_approved {
            SecurityTier::L5AutopilotFloor
        } else {
            SecurityTier::L3StructuredDefault
        };
        evaluate_security_tier(trimmed, allowed_tier)
            .map_err(|v| anyhow!("security tier rejected command ({}): {}", v.rule, v.message))?;

        let started = Instant::now();
        #[cfg(windows)]
        let mut cmd = {
            let pwsh_bin = std::env::var_os("PATH")
                .and_then(|paths| {
                    std::env::split_paths(&paths)
                        .map(|dir| dir.join("pwsh.exe"))
                        .find(|p| p.is_file())
                })
                .unwrap_or_else(|| PathBuf::from("powershell.exe"));
            let mut c = crate::process_util::quiet_tokio_command(pwsh_bin);
            let script = format!(
                "{trimmed}; $__ec = $LASTEXITCODE; Write-Output \"`n{EPHEMERAL_PWD_MARKER}$((Get-Location).Path)\"; exit $(if ($null -ne $__ec) {{ $__ec }} else {{ 0 }})"
            );
            c.args(["-NoLogo", "-NoProfile", "-Command", &script]);
            c
        };
        #[cfg(not(windows))]
        let mut cmd = {
            let sh_bin = std::env::var_os("PATH")
                .and_then(|paths| {
                    std::env::split_paths(&paths)
                        .map(|dir| dir.join("sh"))
                        .find(|p| p.is_file())
                })
                .unwrap_or_else(|| PathBuf::from("sh"));
            let mut c = crate::process_util::quiet_tokio_command(sh_bin);
            let script = format!(
                "{trimmed}\n__vox_ec=$?\nprintf '\\n{EPHEMERAL_PWD_MARKER}%s\\n' \"$PWD\"\nexit $__vox_ec"
            );
            c.args(["-c", &script]);
            c
        };
        cmd.current_dir(&cur_cwd)
            .kill_on_drop(true)
            .stdin(std::process::Stdio::null());
        for pager in [
            "PAGER",
            "GIT_PAGER",
            "SYSTEMD_PAGER",
            "BAT_PAGER",
            "MANPAGER",
            "GH_PAGER",
        ] {
            cmd.env(pager, "cat");
        }

        let wait_dur = Duration::from_millis(req.wait_ms.clamp(50, MAX_DAEMON_WAIT_MS));
        let out = tokio::time::timeout(wait_dur, cmd.output())
            .await
            .context("ephemeral terminal command timed out")?
            .context("spawning ephemeral shell command")?;

        let mut raw_stdout = String::from_utf8_lossy(&out.stdout).into_owned();
        let mut new_cwd = cur_cwd.clone();
        if let Some(pos) = raw_stdout.rfind(EPHEMERAL_PWD_MARKER) {
            let after = &raw_stdout[pos + EPHEMERAL_PWD_MARKER.len()..];
            let pwd_str = after.lines().next().unwrap_or("").trim();
            if !pwd_str.is_empty() {
                new_cwd = PathBuf::from(pwd_str);
            }
            let trim_pos = if pos > 0 && raw_stdout.as_bytes()[pos - 1] == b'\n' {
                pos - 1
            } else {
                pos
            };
            raw_stdout.truncate(trim_pos);
        }
        let raw_stderr = String::from_utf8_lossy(&out.stderr);
        let combined = if raw_stderr.is_empty() {
            raw_stdout
        } else if raw_stdout.is_empty() {
            raw_stderr.into_owned()
        } else {
            format!("{raw_stdout}\n{raw_stderr}")
        };

        let scrubbed_full = scrub_ephemeral_text(&combined);
        let scrubbed_input = scrub_ephemeral_text(trimmed);
        let exit_code = out.status.code().unwrap_or(-1);
        let cwd_display = new_cwd.to_string_lossy().into_owned();

        let (block_id, seq) = {
            let mut map = self.sessions.lock();
            let sess = map
                .entry(req.session_id.clone())
                .or_insert_with(|| EphemeralSession {
                    cwd: new_cwd.clone(),
                    next_block_id: 1,
                    seq: 0,
                    blocks: Vec::new(),
                });
            sess.cwd = new_cwd;
            let bid = sess.next_block_id;
            sess.next_block_id += 1;
            sess.seq += 1;
            sess.blocks.push(EphemeralBlock {
                id: bid,
                input: scrubbed_input,
                full_scrubbed_output: scrubbed_full.clone(),
                exit_code,
                pinned: false,
                cwd: cwd_display.clone(),
            });
            (bid, sess.seq)
        };

        let (bounded, truncated, omitted_lines) = bound_ephemeral_output(
            &scrubbed_full,
            &req.session_id,
            block_id,
            req.head_lines,
            req.tail_lines,
            req.max_bytes,
        );

        Ok(TermExecResponse {
            session_id: req.session_id,
            block_id,
            status: "completed".to_string(),
            exit_code: Some(exit_code),
            seq,
            output: bounded,
            truncated,
            omitted_lines,
            cwd: Some(cwd_display),
            git_branch: None,
            credential_prompt: false,
            prompt_kind: None,
            duration_ms: started.elapsed().as_millis() as u64,
            ephemeral_fallback: true,
        })
    }

    async fn send_input(&self, _req: TermSendInputRequest) -> Result<()> {
        anyhow::bail!("ephemeral terminal fallback does not support interactive stdin handoff")
    }

    async fn read(&self, req: TermReadRequest) -> Result<TermReadResponse> {
        let map = self.sessions.lock();
        let sess = map
            .get(&req.session_id)
            .ok_or_else(|| anyhow!("no such ephemeral session: {}", req.session_id))?;

        let blocks = if let Some(bid) = req.block_id {
            let b = sess
                .blocks
                .iter()
                .find(|b| b.id == bid)
                .ok_or_else(|| anyhow!("no such block {bid} in session {}", req.session_id))?;
            let eff_range = match (req.line_start, req.line_end) {
                (Some(ls), Some(le)) => Some((ls, le)),
                (Some(ls), None) => Some((ls, ls)),
                (None, Some(le)) => Some((1, le)),
                (None, None) => None,
            };
            let output = match eff_range {
                Some((ls, le)) => {
                    let lines: Vec<&str> = b.full_scrubbed_output.lines().collect();
                    let s = ls.max(1) - 1;
                    let e = le.max(ls).min(lines.len());
                    if s >= lines.len() {
                        String::new()
                    } else {
                        lines[s..e].join("\n")
                    }
                }
                None => b.full_scrubbed_output.clone(),
            };
            vec![TermBlockDto {
                id: b.id,
                input: b.input.clone(),
                output,
                status: if b.exit_code == 0 {
                    "ok".into()
                } else {
                    "failed".into()
                },
                exit_code: Some(b.exit_code),
                pinned: b.pinned,
                credential_prompt: false,
                truncated: false,
                omitted_lines: 0,
                cwd: Some(b.cwd.clone()),
                git_branch: None,
            }]
        } else {
            let max_b = req.max_blocks.unwrap_or(10);
            let start = sess.blocks.len().saturating_sub(max_b);
            sess.blocks[start..]
                .iter()
                .map(|b| {
                    let (bounded, truncated, omitted) = bound_ephemeral_output(
                        &b.full_scrubbed_output,
                        &req.session_id,
                        b.id,
                        DEFAULT_HEAD_LINES,
                        DEFAULT_TAIL_LINES,
                        DEFAULT_MAX_BYTES,
                    );
                    TermBlockDto {
                        id: b.id,
                        input: b.input.clone(),
                        output: bounded,
                        status: if b.exit_code == 0 {
                            "ok".into()
                        } else {
                            "failed".into()
                        },
                        exit_code: Some(b.exit_code),
                        pinned: b.pinned,
                        credential_prompt: false,
                        truncated,
                        omitted_lines: omitted,
                        cwd: Some(b.cwd.clone()),
                        git_branch: None,
                    }
                })
                .collect()
        };

        Ok(TermReadResponse {
            session_id: req.session_id,
            seq: sess.seq,
            cwd: Some(sess.cwd.to_string_lossy().into_owned()),
            git_branch: None,
            alt_screen: false,
            security_level: "l3_structured_default".to_string(),
            control_mode: "co_pilot".to_string(),
            blocks,
            ephemeral_fallback: true,
        })
    }

    async fn list_sessions(&self) -> Result<Vec<TermSessionSummary>> {
        let map = self.sessions.lock();
        let mut out: Vec<TermSessionSummary> = map
            .iter()
            .map(|(id, s)| TermSessionSummary {
                session_id: id.clone(),
                blocks: s.blocks.len(),
                cwd: Some(s.cwd.to_string_lossy().into_owned()),
                git_branch: None,
                alt_screen: false,
                security_level: "l3_structured_default".to_string(),
                control_mode: "co_pilot".to_string(),
                ephemeral_fallback: true,
            })
            .collect();
        out.sort_by(|a, b| a.session_id.cmp(&b.session_id));
        Ok(out)
    }

    async fn open_session(&self, cwd: Option<PathBuf>) -> Result<String> {
        let id = {
            let mut n = self.next_session.lock();
            *n += 1;
            format!("ephem-term-{}", *n)
        };
        let init_cwd = cwd.unwrap_or_else(|| self.default_cwd.clone());
        self.sessions.lock().insert(
            id.clone(),
            EphemeralSession {
                cwd: init_cwd,
                next_block_id: 1,
                seq: 0,
                blocks: Vec::new(),
            },
        );
        Ok(id)
    }

    async fn promote_bg(&self, _session_id: &str, _block_id: u64) -> Result<()> {
        anyhow::bail!("ephemeral terminal fallback does not support background promotion")
    }

    async fn pin_block(&self, session_id: &str, block_id: u64, pinned: bool) -> Result<()> {
        let mut map = self.sessions.lock();
        let sess = map
            .get_mut(session_id)
            .ok_or_else(|| anyhow!("no such ephemeral session: {session_id}"))?;
        let block = sess
            .blocks
            .iter_mut()
            .find(|b| b.id == block_id)
            .ok_or_else(|| anyhow!("no such block {block_id} in session {session_id}"))?;
        block.pinned = pinned;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrub_ephemeral_text_normalizes_cr_ecma48_and_redacts_vendor_pem_paths() {
        let home = format!("/{}/alice", "Users");
        let raw = format!(
            "10%\r100%\n\
            tok=sk-\x1b(Bproj-abcdefghijklmnopqrstuvwxyz012345\n\
            pat=ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789\n\
            url=https://admin:s3cr3tPass@db.internal.org/app\n\
            dir={home}/.ssh\n\
            -----BEGIN RSA PRIVATE KEY-----\nabc\n-----END RSA PRIVATE KEY-----\n\
            \x1bP1$r0m\x1b\\🦀"
        );
        let scrubbed = scrub_ephemeral_text(&raw);
        assert!(scrubbed.starts_with("100%\n"), "got: {scrubbed:?}");
        assert!(!scrubbed.contains("10%"));
        assert!(!scrubbed.contains("sk-"));
        assert!(!scrubbed.contains("ghp_"));
        assert!(!scrubbed.contains("s3cr3tPass"));
        assert!(!scrubbed.contains("alice"));
        assert!(!scrubbed.contains("BEGIN RSA PRIVATE KEY"));
        assert!(scrubbed.contains("tok=[REDACTED_KEY]"));
        assert!(scrubbed.contains("pat=[REDACTED_KEY]"));
        assert!(scrubbed.contains("https://admin:[REDACTED_CRED]@db.internal.org/app"));
        assert!(scrubbed.contains("~[REDACTED_PATH]/.ssh"));
        assert!(scrubbed.contains("[REDACTED_PEM]"));
        assert!(scrubbed.ends_with("🦀"));
    }

    #[tokio::test]
    async fn ephemeral_bridge_persists_cwd_reads_slices_and_enforces_policy() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let sub = tmp.path().join("ephem_sub");
        std::fs::create_dir_all(&sub).expect("create_dir_all");

        let bridge = EphemeralTerminalBridge::new(Some(tmp.path().to_path_buf()));
        let sid = bridge.open_session(None).await.expect("open_session");

        let bg_err = bridge
            .exec(TermExecRequest {
                session_id: sid.clone(),
                command: "echo bg".to_string(),
                agent_id: "test-agent".to_string(),
                wait_ms: 0,
                head_lines: 40,
                tail_lines: 160,
                max_bytes: 16_384,
                stall_quiescence_ms: 1_500,
                rationale: None,
                hitl_approved: false,
            })
            .await;
        assert!(bg_err.is_err());

        let r1 = bridge
            .exec(TermExecRequest {
                session_id: sid.clone(),
                command: "cd ephem_sub".to_string(),
                agent_id: "test-agent".to_string(),
                wait_ms: 5_000,
                head_lines: 40,
                tail_lines: 160,
                max_bytes: 16_384,
                stall_quiescence_ms: 1_500,
                rationale: None,
                hitl_approved: false,
            })
            .await
            .expect("cd exec");
        assert!(r1.ephemeral_fallback);
        assert!(r1.cwd.as_deref().unwrap_or("").ends_with("ephem_sub"));

        let r2 = bridge
            .exec(TermExecRequest {
                session_id: sid.clone(),
                command: "echo line1; echo line2; echo line3".to_string(),
                agent_id: "test-agent".to_string(),
                wait_ms: 5_000,
                head_lines: 40,
                tail_lines: 160,
                max_bytes: 16_384,
                stall_quiescence_ms: 1_500,
                rationale: None,
                hitl_approved: false,
            })
            .await
            .expect("echo exec");
        assert!(r2.cwd.as_deref().unwrap_or("").ends_with("ephem_sub"));

        let read_slice = bridge
            .read(TermReadRequest {
                session_id: sid.clone(),
                block_id: Some(r2.block_id),
                line_start: Some(2),
                line_end: None,
                max_blocks: None,
            })
            .await
            .expect("read slice");
        assert_eq!(read_slice.blocks[0].output.trim(), "line2");

        bridge
            .pin_block(&sid, r2.block_id, true)
            .await
            .expect("pin");
        let read_all = bridge
            .read(TermReadRequest {
                session_id: sid.clone(),
                block_id: None,
                line_start: None,
                line_end: None,
                max_blocks: Some(5),
            })
            .await
            .expect("read_all");
        assert!(
            read_all
                .blocks
                .iter()
                .any(|b| b.id == r2.block_id && b.pinned)
        );

        let denied = bridge
            .exec(TermExecRequest {
                session_id: sid,
                command: "sudo whoami".to_string(),
                agent_id: "test-agent".to_string(),
                wait_ms: 5_000,
                head_lines: 40,
                tail_lines: 160,
                max_bytes: 16_384,
                stall_quiescence_ms: 1_500,
                rationale: None,
                hitl_approved: false,
            })
            .await;
        assert!(denied.is_err());
    }
}
