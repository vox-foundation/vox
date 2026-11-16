//! In-process [`TerminalBridge`] adapter over [`vox_terminal_core::LocalHost`] with 3-stage
//! output scrubbing (`strip_ansi_and_normalize` → `vox_secrets::scrub_resolved_secrets` → `vox-redact`).

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use anyhow::{Result, anyhow};
use async_trait::async_trait;
use serde_json::json;
use vox_orchestrator::oversight::exec_policy::{
    ExecPolicyV1, default_exec_policy_v1, evaluate_command_ast,
};
use vox_orchestrator::oversight::floor::{FloorInput, classify_floor};
use vox_orchestrator::terminal_bridge::{
    TermBlockDto, TermExecRequest, TermExecResponse, TermReadRequest, TermReadResponse,
    TermSendInputRequest, TermSessionSummary, TerminalBridge,
};
use vox_terminal_core::agent_exec::{AgentExecRequest, AgentExecStatus};
use vox_terminal_core::block::{BlockId, BlockStatus, strip_ansi_and_normalize};
use vox_terminal_core::corpus::redact::{
    DEFAULT_BLOCK_HEAD_LINES, DEFAULT_BLOCK_MAX_BYTES, DEFAULT_BLOCK_TAIL_LINES, redact_owned,
    slice_lines_1based, truncate_terminal_block,
};
use vox_terminal_core::{LocalHost, OpenSpec, PromptKind, TermControlMode, TermSecurityLevel};

/// Run all 3 terminal output scrubbing stages (`ECMA-48 + CR` → Clavis exact needles → `vox-redact`)
/// before any text reaches MCP/chat/RPC consumers.
#[must_use]
pub fn scrub_three_stage(raw: &str) -> String {
    let stage1 = strip_ansi_and_normalize(raw);
    let stage2 = vox_secrets::scrub_resolved_secrets(&stage1);
    redact_owned(&stage2)
}

/// Defense-in-depth daemon boundary AST + safety-floor validation (`WR-08`, `CR-05`).
pub fn validate_daemon_agent_exec(req: &TermExecRequest, cwd: &Path) -> Result<()> {
    static POLICY: OnceLock<ExecPolicyV1> = OnceLock::new();
    let trimmed = req.command.trim();
    if trimmed.is_empty() || trimmed.contains(['\n', '\r']) {
        anyhow::bail!("daemon exec policy rejected empty or multi-line command");
    }
    let policy = POLICY.get_or_init(default_exec_policy_v1);
    for v in evaluate_command_ast(trimmed, policy) {
        if v.rule != "command-not-allowed" {
            anyhow::bail!(
                "daemon exec policy rejected command ({}): {}",
                v.rule,
                v.message
            );
        }
    }
    if !req.hitl_approved
        && let Some(cat) = classify_floor(&FloorInput::tool(
            "vox_terminal_exec",
            &json!({ "command": trimmed }),
            cwd,
        ))
    {
        anyhow::bail!(
            "daemon safety floor ({}) requires explicit HITL approval",
            cat.id()
        );
    }
    Ok(())
}

fn cap_utf8_bytes(s: String, max_bytes: usize) -> (String, bool) {
    if s.len() <= max_bytes {
        return (s, false);
    }
    let mut cut = max_bytes;
    while cut > 0 && !s.is_char_boundary(cut) {
        cut -= 1;
    }
    (s[..cut].to_string(), true)
}

fn security_level_label(val: TermSecurityLevel) -> String {
    serde_json::to_value(val)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn control_mode_label(val: TermControlMode) -> String {
    serde_json::to_value(val)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn prompt_kind_label(val: PromptKind) -> String {
    serde_json::to_value(val)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .unwrap_or_default()
}

fn block_status_label(st: BlockStatus) -> &'static str {
    match st {
        BlockStatus::Running => "running",
        BlockStatus::AwaitingInput => "awaiting_input",
        BlockStatus::Ok => "ok",
        BlockStatus::Failed => "failed",
        BlockStatus::Cancelled => "cancelled",
    }
}

/// Daemon-local [`TerminalBridge`] sharing the single live [`LocalHost`] with `TermDispatch`.
pub struct LocalHostTerminalBridge {
    host: Arc<LocalHost>,
    default_cwd: Option<PathBuf>,
}

impl LocalHostTerminalBridge {
    #[must_use]
    pub fn new(host: Arc<LocalHost>, default_cwd: Option<PathBuf>) -> Self {
        Self { host, default_cwd }
    }

    #[must_use]
    pub fn host(&self) -> &Arc<LocalHost> {
        &self.host
    }
}

#[async_trait]
impl TerminalBridge for LocalHostTerminalBridge {
    async fn exec(&self, req: TermExecRequest) -> Result<TermExecResponse> {
        let fallback_cwd = self
            .default_cwd
            .as_deref()
            .unwrap_or_else(|| Path::new("."));
        validate_daemon_agent_exec(&req, fallback_cwd)?;
        let head_lines = req.head_lines;
        let tail_lines = req.tail_lines;
        let max_bytes = req.max_bytes;
        let outcome = self
            .host
            .exec_agent_command(AgentExecRequest {
                session_id: req.session_id,
                command: req.command,
                agent_id: req.agent_id,
                wait_ms: req.wait_ms,
                stall_quiescence_ms: req.stall_quiescence_ms,
                head_lines,
                tail_lines,
                max_bytes,
                rationale: req.rationale,
                hitl_approved: req.hitl_approved,
            })
            .await?;

        let status = match outcome.status {
            AgentExecStatus::Completed => "completed",
            AgentExecStatus::Running => "running",
            AgentExecStatus::AwaitingInput => "awaiting_input",
        }
        .to_string();

        let (output, truncated, omitted_lines) = self
            .host
            .block(&outcome.session_id, BlockId(outcome.block_id))
            .map(|b| {
                let scrubbed = scrub_three_stage(&b.plain_output());
                truncate_terminal_block(
                    &scrubbed,
                    &outcome.session_id,
                    outcome.block_id,
                    head_lines,
                    tail_lines,
                    max_bytes,
                )
            })
            .unwrap_or_else(|| {
                (
                    vox_secrets::scrub_resolved_secrets(&outcome.output),
                    outcome.truncated,
                    outcome.omitted_lines,
                )
            });

        Ok(TermExecResponse {
            session_id: outcome.session_id,
            block_id: outcome.block_id,
            status,
            exit_code: outcome.exit_code,
            seq: outcome.seq,
            output,
            truncated,
            omitted_lines,
            cwd: outcome.cwd,
            git_branch: outcome.git_branch,
            credential_prompt: outcome.credential_prompt,
            prompt_kind: outcome.prompt_kind.map(prompt_kind_label),
            duration_ms: outcome.duration_ms,
            ephemeral_fallback: false,
        })
    }

    async fn send_input(&self, req: TermSendInputRequest) -> Result<()> {
        self.host.agent_send_input(
            &req.session_id,
            BlockId(req.block_id),
            &req.text,
            req.append_newline,
            req.hitl_approved,
        )
    }

    async fn read(&self, req: TermReadRequest) -> Result<TermReadResponse> {
        let attached = self.host.attach(&req.session_id)?;
        let security_level = security_level_label(self.host.security_level(&req.session_id)?);
        let control_mode = control_mode_label(self.host.control_mode(&req.session_id)?);

        let blocks = if let Some(bid) = req.block_id {
            let b = attached
                .blocks
                .iter()
                .find(|b| b.id == BlockId(bid))
                .ok_or_else(|| anyhow!("no such block {bid} in session {}", req.session_id))?;
            let scrubbed_in = scrub_three_stage(&b.input);
            let scrubbed_out = scrub_three_stage(&b.plain_output());
            let eff_range = match (req.line_start, req.line_end) {
                (Some(ls), Some(le)) => Some((ls, le)),
                (Some(ls), None) => Some((ls, ls)),
                (None, Some(le)) => Some((1, le)),
                (None, None) => None,
            };
            let (output, truncated, omitted_lines) = match eff_range {
                Some((ls, le)) => {
                    let sliced = slice_lines_1based(&scrubbed_out, ls, le);
                    let (capped, was_capped) = cap_utf8_bytes(sliced, DEFAULT_BLOCK_MAX_BYTES);
                    (capped, was_capped, 0)
                }
                None => truncate_terminal_block(
                    &scrubbed_out,
                    &req.session_id,
                    b.id.0,
                    DEFAULT_BLOCK_HEAD_LINES,
                    DEFAULT_BLOCK_TAIL_LINES,
                    DEFAULT_BLOCK_MAX_BYTES,
                ),
            };
            vec![TermBlockDto {
                id: b.id.0,
                input: scrubbed_in,
                output,
                status: block_status_label(b.status).to_string(),
                exit_code: b.exit_code,
                pinned: b.pinned,
                credential_prompt: b.credential_prompt,
                truncated,
                omitted_lines,
                cwd: b.cwd.clone(),
                git_branch: b.git_branch.clone(),
            }]
        } else {
            let max_b = req.max_blocks.unwrap_or(10);
            let start = attached.blocks.len().saturating_sub(max_b);
            attached.blocks[start..]
                .iter()
                .map(|b| {
                    let scrubbed_in = scrub_three_stage(&b.input);
                    let scrubbed_out = scrub_three_stage(&b.plain_output());
                    let (output, truncated, omitted_lines) = truncate_terminal_block(
                        &scrubbed_out,
                        &req.session_id,
                        b.id.0,
                        DEFAULT_BLOCK_HEAD_LINES,
                        DEFAULT_BLOCK_TAIL_LINES,
                        DEFAULT_BLOCK_MAX_BYTES,
                    );
                    TermBlockDto {
                        id: b.id.0,
                        input: scrubbed_in,
                        output,
                        status: block_status_label(b.status).to_string(),
                        exit_code: b.exit_code,
                        pinned: b.pinned,
                        credential_prompt: b.credential_prompt,
                        truncated,
                        omitted_lines,
                        cwd: b.cwd.clone(),
                        git_branch: b.git_branch.clone(),
                    }
                })
                .collect()
        };

        Ok(TermReadResponse {
            session_id: req.session_id,
            seq: attached.seq,
            cwd: attached.cwd,
            git_branch: attached.git_branch,
            alt_screen: attached.alt_screen,
            security_level,
            control_mode,
            blocks,
            ephemeral_fallback: false,
        })
    }

    async fn list_sessions(&self) -> Result<Vec<TermSessionSummary>> {
        Ok(self
            .host
            .list()
            .into_iter()
            .map(|s| TermSessionSummary {
                session_id: s.id,
                blocks: s.blocks,
                cwd: s.cwd,
                git_branch: s.git_branch,
                alt_screen: s.alt_screen,
                security_level: security_level_label(s.security_level),
                control_mode: control_mode_label(s.control_mode),
                ephemeral_fallback: false,
            })
            .collect())
    }

    async fn open_session(&self, cwd: Option<PathBuf>) -> Result<String> {
        let effective_cwd = cwd.or_else(|| self.default_cwd.clone());
        self.host
            .open_with_cwd(OpenSpec::default(), effective_cwd.as_deref())
            .await
    }

    async fn promote_bg(&self, session_id: &str, block_id: u64) -> Result<()> {
        self.host.promote_bg(session_id, BlockId(block_id))
    }

    async fn pin_block(&self, session_id: &str, block_id: u64, pinned: bool) -> Result<()> {
        self.host.pin_block(session_id, BlockId(block_id), pinned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrub_three_stage_strips_ansi_and_redacts_vendor_tokens() {
        let raw = "\x1b[31mBearer sk-abcdefghijklmnopqrstuvwxyz123456\x1b[0m\r\nok";
        let scrubbed = scrub_three_stage(raw);
        assert!(!scrubbed.contains("\x1b["));
        assert!(!scrubbed.contains("sk-abcdefghijklmnopqrstuvwxyz123456"));
        assert!(scrubbed.contains("[REDACTED_KEY]"));
        assert!(scrubbed.contains("ok"));
    }

    #[tokio::test]
    async fn local_host_terminal_bridge_exec_read_slice_and_pin() {
        let host = Arc::new(LocalHost::default());
        let sid = host.open_synthetic(120, 32);
        let bridge = LocalHostTerminalBridge::new(Arc::clone(&host), None);

        let res = bridge
            .exec(TermExecRequest {
                session_id: sid.clone(),
                command: "printf 'L1\\nBearer sk-abcdefghijklmnopqrstuvwxyz123456\\nL3\\n'"
                    .to_string(),
                agent_id: "unit-agent".to_string(),
                wait_ms: 2_000,
                head_lines: 40,
                tail_lines: 160,
                max_bytes: 16_384,
                stall_quiescence_ms: 1_500,
                rationale: None,
                hitl_approved: false,
            })
            .await
            .expect("synthetic exec");
        assert_eq!(res.status, "completed");
        assert!(!res.ephemeral_fallback);
        assert!(!res.output.contains("sk-abcdefghijklmnopqrstuvwxyz123456"));

        bridge
            .pin_block(&sid, res.block_id, true)
            .await
            .expect("pin_block");

        let read_slice = bridge
            .read(TermReadRequest {
                session_id: sid.clone(),
                block_id: Some(res.block_id),
                line_start: Some(3),
                line_end: None,
                max_blocks: None,
            })
            .await
            .expect("read slice");
        assert_eq!(read_slice.blocks.len(), 1);
        assert!(read_slice.blocks[0].pinned);
        assert_eq!(read_slice.blocks[0].output.trim(), "L3");

        let floor_err = validate_daemon_agent_exec(
            &TermExecRequest {
                session_id: sid,
                command: "git push --force origin main".to_string(),
                agent_id: "unit-agent".to_string(),
                wait_ms: 1_000,
                head_lines: 40,
                tail_lines: 160,
                max_bytes: 16_384,
                stall_quiescence_ms: 1_500,
                rationale: None,
                hitl_approved: false,
            },
            Path::new("."),
        );
        assert!(floor_err.is_err());
    }
}
