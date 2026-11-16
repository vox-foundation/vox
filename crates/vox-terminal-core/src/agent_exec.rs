//! Stateful PTY agent command execution, AST policy gating, and interactive input handoff.

use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use vox_orchestrator::oversight::exec_policy::{
    ExecPolicyV1, SecurityTier, default_exec_policy_v1, evaluate_command_ast,
    evaluate_security_tier, extract_effective_commands,
};

use crate::block::{BlockId, BlockOrigin, BlockStatus};
use crate::corpus::redact::{
    DEFAULT_BLOCK_HEAD_LINES, DEFAULT_BLOCK_MAX_BYTES, DEFAULT_BLOCK_TAIL_LINES,
    scrub_ansi_and_redact, truncate_terminal_block,
};
use crate::host::{
    Entry, LocalHost, OpenSpec, SessionId, TermControlMode, TermSecurityLevel, lock,
};
use crate::input::InputIntent;
use crate::pty::{PtyHandle, default_shell, spawn_pty_with_callback};
use crate::shell_init::normalize_shell_name;
use crate::stall_detector::{DEFAULT_STALL_QUIESCENCE_MS, PromptKind, classify_stall};

const POLL_STEP_MS: u64 = 20;
const PROMPT_SETTLE_GRACE_MS: u64 = 60;
const INITIAL_PROMPT_WAIT_MS: u64 = 2_500;
const DEFAULT_AGENT_WAIT_MS: u64 = 10_000;
const MIN_AGENT_WAIT_MS: u64 = 50;
const MAX_AGENT_WAIT_MS: u64 = 120_000;
const POSIX_PAGER_ENV_PREFIX: &str =
    "PAGER=cat GIT_PAGER=cat SYSTEMD_PAGER=cat BAT_PAGER=cat MANPAGER=cat GH_PAGER=cat";

fn default_wait_ms() -> u64 {
    DEFAULT_AGENT_WAIT_MS
}
fn default_quiescence_ms() -> u64 {
    DEFAULT_STALL_QUIESCENCE_MS
}
fn default_head_lines() -> usize {
    DEFAULT_BLOCK_HEAD_LINES
}
fn default_tail_lines() -> usize {
    DEFAULT_BLOCK_TAIL_LINES
}
fn default_max_bytes() -> usize {
    DEFAULT_BLOCK_MAX_BYTES
}

/// Request parameters for stateful PTY agent execution on [`LocalHost`].
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentExecRequest {
    pub session_id: String,
    pub command: String,
    pub agent_id: String,
    #[serde(default = "default_wait_ms")]
    pub wait_ms: u64,
    #[serde(default = "default_quiescence_ms")]
    pub stall_quiescence_ms: u64,
    #[serde(default = "default_head_lines")]
    pub head_lines: usize,
    #[serde(default = "default_tail_lines")]
    pub tail_lines: usize,
    #[serde(default = "default_max_bytes")]
    pub max_bytes: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    #[serde(default)]
    pub hitl_approved: bool,
}

/// Lifecycle state returned from a bounded `exec_agent_command` wait window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentExecStatus {
    Completed,
    Running,
    AwaitingInput,
}

/// Structured, scrubbed, and bounded outcome of [`LocalHost::exec_agent_command`].
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AgentExecOutcome {
    pub session_id: String,
    pub block_id: u64,
    pub status: AgentExecStatus,
    pub exit_code: Option<i32>,
    pub seq: u64,
    pub output: String,
    pub truncated: bool,
    pub omitted_lines: usize,
    pub cwd: Option<String>,
    pub git_branch: Option<String>,
    pub credential_prompt: bool,
    pub prompt_kind: Option<PromptKind>,
    pub duration_ms: u64,
}

fn strip_intent_prefix(raw: &str) -> &str {
    raw.strip_prefix("!nu ")
        .or_else(|| raw.strip_prefix('!'))
        .or_else(|| raw.strip_prefix("/sh "))
        .or_else(|| raw.strip_prefix("/nu "))
        .unwrap_or(raw)
        .trim()
}

/// Validate `cmd` against the unified `exec-policy.v1` AST evaluator and `level` tier (`T-442-05`).
pub fn allows_agent_command(level: TermSecurityLevel, cmd: &str) -> Result<(), String> {
    allows_agent_command_with_approval(level, cmd, false)
}

/// Validate `cmd` against `exec-policy.v1`, allowing human HITL-approved tier overrides up to
/// [`SecurityTier::L5AutopilotFloor`] while unconditionally rejecting hard AST hazards (`CR-05`).
pub fn allows_agent_command_with_approval(
    level: TermSecurityLevel,
    cmd: &str,
    hitl_approved: bool,
) -> Result<(), String> {
    static POLICY: OnceLock<ExecPolicyV1> = OnceLock::new();

    let raw = cmd.trim();
    if raw.contains(['\n', '\r']) {
        return Err(format!(
            "security level {level:?} rejected multi-line agent command"
        ));
    }

    let trimmed = strip_intent_prefix(raw);
    if trimmed.is_empty() {
        return Err(format!(
            "security level {level:?} rejected empty agent command"
        ));
    }

    let policy = POLICY.get_or_init(default_exec_policy_v1);
    for v in evaluate_command_ast(trimmed, policy) {
        if v.rule != "command-not-allowed" {
            return Err(format!(
                "security level {level:?} rejected command ({}): {}",
                v.rule, v.message
            ));
        }
    }

    if !hitl_approved
        && level == TermSecurityLevel::L1ReadOnly
        && extract_effective_commands(trimmed).len() > 1
    {
        return Err("security level L1ReadOnly rejects shell chaining and pipelines".to_string());
    }

    let tier = if hitl_approved {
        SecurityTier::L5AutopilotFloor
    } else {
        match level {
            TermSecurityLevel::L1ReadOnly => SecurityTier::L1ReadOnly,
            TermSecurityLevel::L2WorkspaceBuild => SecurityTier::L2WorkspaceBuild,
            TermSecurityLevel::L3StructuredDefault => SecurityTier::L3StructuredDefault,
            TermSecurityLevel::L4HostGated => SecurityTier::L4HostGated,
            TermSecurityLevel::L5AutopilotFloor => SecurityTier::L5AutopilotFloor,
        }
    };

    evaluate_security_tier(trimmed, tier)
        .map(|_| ())
        .map_err(|v| {
            format!(
                "security level {level:?} rejected command ({}): {}",
                v.rule, v.message
            )
        })
}

/// Wrap `cmd` with non-subshell pager neutralization for the target shell.
pub fn format_agent_command_for_shell(shell_name: &str, osc633_capable: bool, cmd: &str) -> String {
    let trimmed = cmd.trim();
    if osc633_capable {
        return format!("__vox_pager_enter; {trimmed}");
    }
    if normalize_shell_name(shell_name) == "cmd" {
        return format!(
            "set PAGER=cat&& set GIT_PAGER=cat&& set SYSTEMD_PAGER=cat&& set BAT_PAGER=cat&& set MANPAGER=cat&& set GH_PAGER=cat&& {trimmed}"
        );
    }
    format!("{POSIX_PAGER_ENV_PREFIX} {trimmed}")
}

impl LocalHost {
    /// Spawn a PTY session with an optional initial working directory override.
    pub async fn open_with_cwd(&self, spec: OpenSpec, cwd: Option<&Path>) -> Result<SessionId> {
        let def = default_shell();
        let shell = spec.shell.as_deref().unwrap_or(&def);
        let shell_name = normalize_shell_name(shell);
        let cols = if spec.cols == 0 { 120 } else { spec.cols };
        let rows = if spec.rows == 0 { 32 } else { spec.rows };
        let id = {
            let mut n = lock(&self.next);
            *n += 1;
            format!("term-{n}")
        };
        let entry =
            Entry::new_with_size_and_shell(&id, None, self.replay_cap, cols, rows, shell_name);
        let pump_chunk = Arc::clone(&entry);
        let pump_eof = Arc::clone(&entry);
        let handle = spawn_pty_with_callback(
            shell,
            cols,
            rows,
            cwd,
            move |bytes| pump_chunk.feed(bytes),
            move || {
                let needs_exit = lock(&pump_eof.session).open_block_id().is_some();
                let mut exit_code = 0;
                for _ in 0..if needs_exit { 15 } else { 3 } {
                    let code = lock(&pump_eof.pty)
                        .as_mut()
                        .and_then(PtyHandle::try_wait_exit_code);
                    if let Some(c) = code {
                        exit_code = c;
                        break;
                    }
                    // drift-allow(timeout-literal,duplicate-numeric-literal): child exit reap poll
                    std::thread::sleep(Duration::from_millis(2));
                }
                lock(&pump_eof.session).finish_open_block_on_eof(exit_code);
                pump_eof.pty_eof.store(true, Ordering::Release);
                pump_eof.settled_cv.notify_all();
            },
        )?;
        let osc633_capable = handle.supports_osc633();
        *lock(&entry.pty) = Some(handle);
        lock(&self.sessions).insert(id.clone(), Arc::clone(&entry));

        if osc633_capable {
            let wait_entry = Arc::clone(&entry);
            let _ = tokio::task::spawn_blocking(move || {
                wait_entry.wait_until(Duration::from_millis(INITIAL_PROMPT_WAIT_MS), |s| {
                    s.is_at_prompt()
                });
            })
            .await;
        }
        Ok(id)
    }

    /// Execute an agent shell command inside the stateful PTY session with stall & TUI detection.
    pub async fn exec_agent_command(&self, req: AgentExecRequest) -> Result<AgentExecOutcome> {
        let entry = self.entry(&req.session_id)?;
        if *lock(&entry.control_mode) == TermControlMode::Manual && !req.hitl_approved {
            anyhow::bail!(
                "terminal session {} is in Manual control mode; agent execution denied",
                req.session_id
            );
        }
        let level = *lock(&entry.security_level);
        allows_agent_command_with_approval(level, &req.command, req.hitl_approved)
            .map_err(|e| anyhow!(e))?;

        let _exec_guard = entry.agent_exec_lock.lock().await;
        {
            let sess = lock(&entry.session);
            if sess.is_alt_screen() {
                anyhow::bail!(
                    "terminal session {} is in alternate-screen TUI mode; exit the fullscreen TUI before running agent commands",
                    req.session_id
                );
            }
            if let Some(running) = sess.open_block_id() {
                anyhow::bail!(
                    "terminal session {} already has an active running block ({running:?})",
                    req.session_id
                );
            }
        }

        let clean_cmd = strip_intent_prefix(req.command.trim()).to_string();
        let origin = BlockOrigin::Agent {
            agent_id: req.agent_id.clone(),
        };
        let started = Instant::now();
        entry.promoted_bg.store(false, Ordering::Release);

        let (has_pty, osc633_capable, shell_name) = if entry.pty_eof.load(Ordering::Acquire) {
            (false, false, entry.shell_name.clone())
        } else {
            lock(&entry.pty)
                .as_ref()
                .map(|p| (true, p.supports_osc633(), p.shell_name().to_string()))
                .unwrap_or((false, false, entry.shell_name.clone()))
        };

        if !has_pty {
            let bid =
                lock(&entry.session).submit_with_origin(InputIntent::Shell(clean_cmd), origin);
            let seq = lock(&entry.replay).seq;
            let s = lock(&entry.session);
            let b = s
                .blocks()
                .iter()
                .find(|b| b.id == bid)
                .ok_or_else(|| anyhow!("missing completed synthetic block {}", bid.0))?;
            let scrubbed = scrub_ansi_and_redact(&b.plain_output());
            let (output, truncated, omitted_lines) = truncate_terminal_block(
                &scrubbed,
                &req.session_id,
                bid.0,
                req.head_lines,
                req.tail_lines,
                req.max_bytes,
            );
            return Ok(AgentExecOutcome {
                session_id: req.session_id,
                block_id: bid.0,
                status: AgentExecStatus::Completed,
                exit_code: b.exit_code,
                seq,
                output,
                truncated,
                omitted_lines,
                cwd: s.cwd().map(str::to_owned),
                git_branch: s.git_branch().map(str::to_owned),
                credential_prompt: false,
                prompt_kind: None,
                duration_ms: started.elapsed().as_millis() as u64,
            });
        }

        *lock(&entry.last_output_at) = started;
        let block_id = lock(&entry.session).begin_pty_submission(clean_cmd.clone(), origin);
        let wire_line = format_agent_command_for_shell(&shell_name, osc633_capable, &clean_cmd);
        let write_res = {
            let mut pty_guard = lock(&entry.pty);
            match pty_guard.as_mut() {
                Some(pty) => pty.write(format!("{wire_line}\n").as_bytes()),
                None => Err(anyhow!("terminal session {} is closed", req.session_id)),
            }
        };
        if let Err(err) = write_res {
            lock(&entry.session).finish_open_block_on_eof(-1);
            return Err(err);
        }

        let mut status = AgentExecStatus::Running;
        let mut prompt_kind = None;
        let mut credential_prompt = false;
        if req.wait_ms > 0 {
            let wait_ms = req.wait_ms.clamp(MIN_AGENT_WAIT_MS, MAX_AGENT_WAIT_MS);
            let mut completed_at: Option<Instant> = None;
            loop {
                let step_action = {
                    let mut sess = lock(&entry.session);
                    let block_state = sess
                        .blocks()
                        .iter()
                        .find(|b| b.id == block_id)
                        .map(|b| (b.status, b.plain_output()));
                    match block_state {
                        Some((
                            BlockStatus::Ok | BlockStatus::Failed | BlockStatus::Cancelled,
                            _,
                        )) => {
                            let ready = !osc633_capable
                                || entry.pty_eof.load(Ordering::Acquire)
                                || sess.is_at_prompt();
                            if ready {
                                status = AgentExecStatus::Completed;
                                true
                            } else {
                                let since = *completed_at.get_or_insert_with(Instant::now);
                                if since.elapsed() >= Duration::from_millis(PROMPT_SETTLE_GRACE_MS)
                                {
                                    status = AgentExecStatus::Completed;
                                    true
                                } else {
                                    false
                                }
                            }
                        }
                        Some((_, plain)) => {
                            if entry.promoted_bg.swap(false, Ordering::AcqRel) {
                                status = AgentExecStatus::Running;
                                true
                            } else {
                                let silence_ms =
                                    lock(&entry.last_output_at).elapsed().as_millis() as u64;
                                if let Some(sig) = classify_stall(
                                    &plain,
                                    sess.is_alt_screen(),
                                    silence_ms,
                                    req.stall_quiescence_ms,
                                ) {
                                    sess.mark_block_awaiting_input(block_id, &sig);
                                    status = AgentExecStatus::AwaitingInput;
                                    prompt_kind = Some(sig.prompt_kind);
                                    credential_prompt = sig.credential_prompt;
                                    true
                                } else {
                                    started.elapsed() >= Duration::from_millis(wait_ms)
                                }
                            }
                        }
                        None => true,
                    }
                };
                if step_action {
                    break;
                }
                // drift-allow(timeout-literal): async PTY completion/stall poll cadence
                tokio::time::sleep(Duration::from_millis(POLL_STEP_MS)).await;
            }
        }

        let seq = lock(&entry.replay).seq;
        let sess = lock(&entry.session);
        let (raw_out, exit_code, block_cred) = sess
            .blocks()
            .iter()
            .find(|b| b.id == block_id)
            .map(|b| (b.plain_output(), b.exit_code, b.credential_prompt))
            .unwrap_or_default();
        let scrubbed = scrub_ansi_and_redact(&raw_out);
        let (output, truncated, omitted_lines) = truncate_terminal_block(
            &scrubbed,
            &req.session_id,
            block_id.0,
            req.head_lines,
            req.tail_lines,
            req.max_bytes,
        );

        Ok(AgentExecOutcome {
            session_id: req.session_id,
            block_id: block_id.0,
            status,
            exit_code,
            seq,
            output,
            truncated,
            omitted_lines,
            cwd: sess.cwd().map(str::to_owned),
            git_branch: sess.git_branch().map(str::to_owned),
            credential_prompt: credential_prompt || block_cred,
            prompt_kind,
            duration_ms: started.elapsed().as_millis() as u64,
        })
    }

    /// Answer a non-credential `AwaitingInput` prompt on an active PTY block (`T-442-07`).
    pub fn agent_send_input(
        &self,
        id: &str,
        block_id: BlockId,
        text: &str,
        append_newline: bool,
        hitl_approved: bool,
    ) -> Result<()> {
        let entry = self.entry(id)?;
        if *lock(&entry.control_mode) == TermControlMode::Manual && !hitl_approved {
            anyhow::bail!("terminal session {id} is in Manual control mode; agent input denied");
        }
        lock(&entry.session).resume_awaiting_block(block_id)?;
        *lock(&entry.last_output_at) = Instant::now();
        let payload = if append_newline {
            format!("{text}\n")
        } else {
            text.to_string()
        };
        let mut pty_guard = lock(&entry.pty);
        pty_guard
            .as_mut()
            .ok_or_else(|| anyhow!("terminal session {id} is closed"))?
            .write(payload.as_bytes())
    }

    /// Pin or unpin a terminal block for sticky chat context attachment.
    pub fn pin_block(&self, id: &str, block_id: BlockId, pinned: bool) -> Result<()> {
        let entry = self.entry(id)?;
        lock(&entry.session).set_block_pinned(block_id, pinned)
    }

    /// Promote an in-flight `exec_agent_command` wait loop to return `Running` early.
    pub fn promote_bg(&self, id: &str, block_id: BlockId) -> Result<()> {
        let entry = self.entry(id)?;
        let active = lock(&entry.session).open_block_id();
        anyhow::ensure!(
            active == Some(block_id),
            "block {} is not the active running block in session {id}",
            block_id.0
        );
        entry.promoted_bg.store(true, Ordering::Release);
        entry.settled_cv.notify_all();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_agent_command_enforces_ast_hazards_and_tiers() {
        for level in [
            TermSecurityLevel::L1ReadOnly,
            TermSecurityLevel::L2WorkspaceBuild,
            TermSecurityLevel::L3StructuredDefault,
            TermSecurityLevel::L4HostGated,
            TermSecurityLevel::L5AutopilotFloor,
        ] {
            assert!(allows_agent_command(level, "pwsh -EncodedCommand ZQBjAGgAbwA=").is_err());
            assert!(allows_agent_command(level, "eval 'echo hi'").is_err());
            assert!(allows_agent_command(level, "echo $(whoami)").is_err());
        }

        assert!(
            allows_agent_command(TermSecurityLevel::L3StructuredDefault, "sh -c 'sudo id'")
                .is_err()
        );
        assert!(allows_agent_command(TermSecurityLevel::L4HostGated, "sudo id").is_ok());
        assert!(
            allows_agent_command_with_approval(TermSecurityLevel::L1ReadOnly, "sudo id", true)
                .is_ok()
        );
        assert!(
            allows_agent_command_with_approval(
                TermSecurityLevel::L5AutopilotFloor,
                "eval 'echo hi'",
                true,
            )
            .is_err()
        );
    }

    #[test]
    fn format_agent_command_wraps_five_shells_without_subshell() {
        for shell in ["zsh", "bash", "fish", "pwsh", "nu"] {
            let wrapped = format_agent_command_for_shell(shell, true, "git log -n 5");
            assert_eq!(wrapped, "__vox_pager_enter; git log -n 5");
        }
        let fallback = format_agent_command_for_shell("sh", false, "git status");
        assert!(fallback.starts_with("PAGER=cat GIT_PAGER=cat "));
        assert!(fallback.ends_with(" git status"));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn live_pty_exec_preserves_state_handles_prompts_and_refuses_credentials() {
        let host = LocalHost::default();
        let tmp = tempfile::tempdir().expect("tempdir");
        let sub = tmp.path().join("stateful_sub");
        std::fs::create_dir_all(&sub).expect("create_dir_all");

        let id = host
            .open_with_cwd(OpenSpec::default(), Some(tmp.path()))
            .await
            .expect("open_with_cwd");

        // 1. Stateful cwd + env persistence across turns without subshell loss
        let out1 = host
            .exec_agent_command(AgentExecRequest {
                session_id: id.clone(),
                command: "cd stateful_sub && export VOX_AGENT_KEEP=99".to_string(),
                agent_id: "agent-test".to_string(),
                wait_ms: 5_000,
                stall_quiescence_ms: 500,
                head_lines: 40,
                tail_lines: 160,
                max_bytes: 16_384,
                rationale: None,
                hitl_approved: false,
            })
            .await
            .expect("turn 1");
        assert_eq!(out1.status, AgentExecStatus::Completed);
        assert_eq!(out1.exit_code, Some(0));

        let out2 = host
            .exec_agent_command(AgentExecRequest {
                session_id: id.clone(),
                command: "pwd && printf 'KEEP=%s\\n' \"$VOX_AGENT_KEEP\"".to_string(),
                agent_id: "agent-test".to_string(),
                wait_ms: 5_000,
                stall_quiescence_ms: 500,
                head_lines: 40,
                tail_lines: 160,
                max_bytes: 16_384,
                rationale: None,
                hitl_approved: false,
            })
            .await
            .expect("turn 2");
        assert_eq!(out2.status, AgentExecStatus::Completed);
        assert!(
            out2.output.contains("stateful_sub") && out2.output.contains("KEEP=99"),
            "unexpected turn 2 output: {:?}",
            out2.output
        );

        // 2. Pinning a block emits BlockPinned and marks block.pinned
        let mut rx = host.attach(&id).expect("attach").events;
        host.pin_block(&id, BlockId(out2.block_id), true)
            .expect("pin_block");
        assert!(host.block(&id, BlockId(out2.block_id)).unwrap().pinned);
        let pinned_ev = rx.recv().await.expect("pinned event");
        assert!(matches!(
            pinned_ev,
            crate::session::SessionEvent::BlockPinned { pinned: true, .. }
        ));

        // 3. Yes/No interactive prompt pauses in AwaitingInput and resumes via agent_send_input
        let prompt_out = host
            .exec_agent_command(AgentExecRequest {
                session_id: id.clone(),
                command: "printf 'Proceed? [y/N] '; read ans; printf 'ANS=%s\\n' \"$ans\""
                    .to_string(),
                agent_id: "agent-test".to_string(),
                wait_ms: 4_000,
                stall_quiescence_ms: 250,
                head_lines: 40,
                tail_lines: 160,
                max_bytes: 16_384,
                rationale: None,
                hitl_approved: false,
            })
            .await
            .expect("yes/no prompt exec");
        assert_eq!(prompt_out.status, AgentExecStatus::AwaitingInput);
        assert_eq!(prompt_out.prompt_kind, Some(PromptKind::YesNo));
        assert!(!prompt_out.credential_prompt);

        host.agent_send_input(&id, BlockId(prompt_out.block_id), "y", true, false)
            .expect("agent_send_input for non-credential prompt");

        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        loop {
            let b = host.block(&id, BlockId(prompt_out.block_id)).unwrap();
            if b.status == BlockStatus::Ok {
                assert!(b.plain_output().contains("ANS=y"));
                break;
            }
            assert!(
                tokio::time::Instant::now() < deadline,
                "timed out waiting for resumed block to finish: {b:?}"
            );
            tokio::time::sleep(Duration::from_millis(25)).await;
        }

        // 4. Credential prompt sets credential_prompt=true and refuses agent_send_input
        let cred_out = host
            .exec_agent_command(AgentExecRequest {
                session_id: id.clone(),
                command: "printf '[sudo] challenge for user: '; read c; printf 'DONE\\n'"
                    .to_string(),
                agent_id: "agent-test".to_string(),
                wait_ms: 4_000,
                stall_quiescence_ms: 250,
                head_lines: 40,
                tail_lines: 160,
                max_bytes: 16_384,
                rationale: None,
                hitl_approved: false,
            })
            .await
            .expect("credential prompt exec");
        assert_eq!(cred_out.status, AgentExecStatus::AwaitingInput);
        assert_eq!(cred_out.prompt_kind, Some(PromptKind::Credential));
        assert!(cred_out.credential_prompt);

        let err = host
            .agent_send_input(&id, BlockId(cred_out.block_id), "hunter2", true, true)
            .expect_err("credential prompt must refuse agent_send_input");
        assert!(err.to_string().contains("credential"));

        host.close(&id).expect("close");
    }

    #[tokio::test]
    async fn manual_hitl_override_and_alt_screen_rejection_work() {
        let host = LocalHost::default();
        let id = host.open_synthetic(120, 32);
        host.set_control_mode(&id, TermControlMode::Manual)
            .expect("set_control_mode");

        let denied = host
            .exec_agent_command(AgentExecRequest {
                session_id: id.clone(),
                command: "pwd".to_string(),
                agent_id: "agent-test".to_string(),
                wait_ms: 500,
                stall_quiescence_ms: 250,
                head_lines: 40,
                tail_lines: 160,
                max_bytes: 16_384,
                rationale: None,
                hitl_approved: false,
            })
            .await;
        assert!(denied.is_err());

        let approved = host
            .exec_agent_command(AgentExecRequest {
                session_id: id.clone(),
                command: "pwd".to_string(),
                agent_id: "agent-test".to_string(),
                wait_ms: 500,
                stall_quiescence_ms: 250,
                head_lines: 40,
                tail_lines: 160,
                max_bytes: 16_384,
                rationale: None,
                hitl_approved: true,
            })
            .await
            .expect("hitl_approved overrides Manual mode");
        assert_eq!(approved.status, AgentExecStatus::Completed);

        host.entry(&id).expect("entry").feed(b"\x1b[?1049h");
        let alt_err = host
            .exec_agent_command(AgentExecRequest {
                session_id: id.clone(),
                command: "pwd".to_string(),
                agent_id: "agent-test".to_string(),
                wait_ms: 500,
                stall_quiescence_ms: 250,
                head_lines: 40,
                tail_lines: 160,
                max_bytes: 16_384,
                rationale: None,
                hitl_approved: true,
            })
            .await
            .expect_err("alt-screen TUI must reject agent commands");
        assert!(alt_err.to_string().contains("alternate-screen"));
    }
}
