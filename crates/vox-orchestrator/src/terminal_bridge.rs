//! Orchestrator-layer terminal session bridge (`TerminalBridge`), loopback daemon client
//! (`OrchDaemonTerminalBridge`), and standalone sticky-cwd fallback (`EphemeralTerminalBridge`).

use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use async_trait::async_trait;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::orch_daemon::OrchDaemonClient;

const DEFAULT_WAIT_MS: u64 = 10_000;
pub(super) const MAX_DAEMON_WAIT_MS: u64 = 120_000;
const DAEMON_DEADLINE_MARGIN_MS: u64 = 15_000;
const DEFAULT_QUIESCENCE_MS: u64 = 1_500;
pub(super) const DEFAULT_HEAD_LINES: usize = 40;
pub(super) const DEFAULT_TAIL_LINES: usize = 160;
pub(super) const DEFAULT_MAX_BYTES: usize = 16_384;
pub(super) const EPHEMERAL_PWD_MARKER: &str = "__VOX_EPHEM_PWD__:";

fn default_wait_ms() -> u64 {
    DEFAULT_WAIT_MS
}
fn default_quiescence_ms() -> u64 {
    DEFAULT_QUIESCENCE_MS
}
fn default_head_lines() -> usize {
    DEFAULT_HEAD_LINES
}
fn default_tail_lines() -> usize {
    DEFAULT_TAIL_LINES
}
fn default_max_bytes() -> usize {
    DEFAULT_MAX_BYTES
}
fn default_true() -> bool {
    true
}

/// Request payload for executing a command inside a terminal session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermExecRequest {
    pub session_id: String,
    pub command: String,
    pub agent_id: String,
    #[serde(default = "default_wait_ms")]
    pub wait_ms: u64,
    #[serde(default = "default_head_lines")]
    pub head_lines: usize,
    #[serde(default = "default_tail_lines")]
    pub tail_lines: usize,
    #[serde(default = "default_max_bytes")]
    pub max_bytes: usize,
    #[serde(default = "default_quiescence_ms")]
    pub stall_quiescence_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    #[serde(default)]
    pub hitl_approved: bool,
}

/// Scrubbed, bounded outcome of [`TerminalBridge::exec`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermExecResponse {
    pub session_id: String,
    pub block_id: u64,
    pub status: String,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub seq: u64,
    #[serde(default)]
    pub output: String,
    #[serde(default)]
    pub truncated: bool,
    #[serde(default)]
    pub omitted_lines: usize,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub git_branch: Option<String>,
    #[serde(default)]
    pub credential_prompt: bool,
    #[serde(default)]
    pub prompt_kind: Option<String>,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub ephemeral_fallback: bool,
}

/// Request payload for answering a non-credential `awaiting_input` terminal block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermSendInputRequest {
    pub session_id: String,
    pub block_id: u64,
    pub text: String,
    #[serde(default = "default_true")]
    pub append_newline: bool,
    #[serde(default)]
    pub hitl_approved: bool,
}

/// Request payload for reading block output or session block history.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermReadRequest {
    pub session_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_start: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_end: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_blocks: Option<usize>,
}

/// Scrubbed DTO representing one terminal command block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermBlockDto {
    pub id: u64,
    pub input: String,
    pub output: String,
    pub status: String,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default)]
    pub credential_prompt: bool,
    #[serde(default)]
    pub truncated: bool,
    #[serde(default)]
    pub omitted_lines: usize,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub git_branch: Option<String>,
}

/// Scrubbed snapshot of a terminal session and requested blocks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermReadResponse {
    pub session_id: String,
    #[serde(default)]
    pub seq: u64,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub git_branch: Option<String>,
    #[serde(default)]
    pub alt_screen: bool,
    #[serde(default)]
    pub security_level: String,
    #[serde(default)]
    pub control_mode: String,
    #[serde(default)]
    pub blocks: Vec<TermBlockDto>,
    #[serde(default)]
    pub ephemeral_fallback: bool,
}

/// Summary entry returned by [`TerminalBridge::list_sessions`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TermSessionSummary {
    pub session_id: String,
    #[serde(default)]
    pub blocks: usize,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub git_branch: Option<String>,
    #[serde(default)]
    pub alt_screen: bool,
    #[serde(default)]
    pub security_level: String,
    #[serde(default)]
    pub control_mode: String,
    #[serde(default)]
    pub ephemeral_fallback: bool,
}

/// Model-agnostic orchestrator seam over live PTY sessions or standalone ephemeral sessions.
#[async_trait]
pub trait TerminalBridge: Send + Sync {
    async fn exec(&self, req: TermExecRequest) -> Result<TermExecResponse>;
    async fn send_input(&self, req: TermSendInputRequest) -> Result<()>;
    async fn read(&self, req: TermReadRequest) -> Result<TermReadResponse>;
    async fn list_sessions(&self) -> Result<Vec<TermSessionSummary>>;
    async fn open_session(&self, cwd: Option<PathBuf>) -> Result<String>;
    async fn promote_bg(&self, session_id: &str, block_id: u64) -> Result<()>;
    async fn pin_block(&self, session_id: &str, block_id: u64, pinned: bool) -> Result<()>;
}

/// Remote [`TerminalBridge`] forwarding `term.*` RPCs to `vox-orchestrator-d`.
#[derive(Debug, Clone)]
pub struct OrchDaemonTerminalBridge {
    client: OrchDaemonClient,
}

impl OrchDaemonTerminalBridge {
    #[must_use]
    pub fn new(client: OrchDaemonClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl TerminalBridge for OrchDaemonTerminalBridge {
    async fn exec(&self, req: TermExecRequest) -> Result<TermExecResponse> {
        let deadline = Duration::from_millis(
            req.wait_ms
                .min(MAX_DAEMON_WAIT_MS)
                .saturating_add(DAEMON_DEADLINE_MARGIN_MS),
        );
        let params = serde_json::to_value(&req)?;
        let val = self
            .client
            .call_with_deadline("term.agent_exec", params, deadline)
            .await?;
        serde_json::from_value(val).context("decoding term.agent_exec response")
    }

    async fn send_input(&self, req: TermSendInputRequest) -> Result<()> {
        let params = serde_json::to_value(&req)?;
        self.client
            .call("term.agent_send_input", params)
            .await
            .map(drop)
    }

    async fn read(&self, req: TermReadRequest) -> Result<TermReadResponse> {
        let params = serde_json::to_value(&req)?;
        let val = self.client.call("term.read", params).await?;
        serde_json::from_value(val).context("decoding term.read response")
    }

    async fn list_sessions(&self) -> Result<Vec<TermSessionSummary>> {
        let val = self.client.call("term.list", json!({})).await?;
        let arr = val
            .as_array()
            .ok_or_else(|| anyhow!("term.list: expected JSON array"))?;
        arr.iter()
            .map(|v| serde_json::from_value(v.clone()).context("decoding term.list item"))
            .collect()
    }

    async fn open_session(&self, cwd: Option<PathBuf>) -> Result<String> {
        let cwd_str = cwd.map(|p| p.to_string_lossy().into_owned());
        let val = self
            .client
            .call("term.open", json!({ "cwd": cwd_str }))
            .await?;
        val.get("session_id")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| anyhow!("term.open response missing session_id"))
    }

    async fn promote_bg(&self, session_id: &str, block_id: u64) -> Result<()> {
        self.client
            .call(
                "term.promote_bg",
                json!({ "session_id": session_id, "block_id": block_id }),
            )
            .await
            .map(drop)
    }

    async fn pin_block(&self, session_id: &str, block_id: u64, pinned: bool) -> Result<()> {
        self.client
            .call(
                "term.pin",
                json!({ "session_id": session_id, "block_id": block_id, "pinned": pinned }),
            )
            .await
            .map(drop)
    }
}

fn strip_ecma48_and_normalize_cr(raw: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }
    let mut out = String::with_capacity(raw.len());
    let mut line: Vec<char> = Vec::new();
    let mut col = 0usize;
    let mut chars = raw.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '\r' if chars.peek() == Some(&'\n') => {
                chars.next();
                out.extend(&line);
                out.push('\n');
                line.clear();
                col = 0;
            }
            '\r' => col = 0,
            '\n' => {
                out.extend(&line);
                out.push('\n');
                line.clear();
                col = 0;
            }
            '\x1b' => match chars.peek().copied() {
                Some('[') => {
                    chars.next();
                    let mut params = String::new();
                    let mut fin = None;
                    for ch in chars.by_ref() {
                        if ('\x40'..='\x7e').contains(&ch) {
                            fin = Some(ch);
                            break;
                        }
                        params.push(ch);
                    }
                    if fin == Some('K') {
                        match params.trim() {
                            "" | "0" => line.truncate(col),
                            "1" => {
                                let take_n = (col + 1).min(line.len());
                                line.iter_mut().take(take_n).for_each(|x| *x = ' ');
                            }
                            "2" => {
                                line.clear();
                                col = 0;
                            }
                            _ => {}
                        }
                    }
                }
                Some(']') | Some('P') | Some('X') | Some('^') | Some('_') => {
                    chars.next();
                    while let Some(ch) = chars.next() {
                        if ch == '\x07' {
                            break;
                        }
                        if ch == '\x1b' {
                            if chars.peek() == Some(&'\\') {
                                chars.next();
                            }
                            break;
                        }
                    }
                }
                Some(i) if ('\x20'..='\x2f').contains(&i) => {
                    chars.next();
                    while chars.peek().is_some_and(|n| ('\x20'..='\x2f').contains(n)) {
                        chars.next();
                    }
                    if chars.peek().is_some_and(|n| ('\x30'..='\x7e').contains(n)) {
                        chars.next();
                    }
                }
                Some(f) if ('\x30'..='\x7e').contains(&f) => {
                    chars.next();
                }
                _ => {}
            },
            ch if ch == '\t' || ((ch as u32) >= 0x20 && ch != '\x7f') => {
                if col < line.len() {
                    line[col] = ch;
                } else {
                    line.push(ch);
                }
                col += 1;
            }
            _ => {}
        }
    }
    out.extend(&line);
    out
}

pub(super) fn scrub_ephemeral_text(raw: &str) -> String {
    static REDACTORS: OnceLock<[Regex; 7]> = OnceLock::new();
    if raw.is_empty() {
        return String::new();
    }
    let stage1 = strip_ecma48_and_normalize_cr(raw);
    let stage2 = vox_secrets::scrub_resolved_secrets(&stage1);
    let [
        re_pem,
        re_url_cred,
        re_email,
        re_api_key,
        re_token,
        re_ipv4,
        re_home,
    ] = REDACTORS.get_or_init(|| {
        [
            Regex::new(
                r"(?s)-----BEGIN [A-Z0-9 _\-]*PRIVATE KEY-----.*?-----END [A-Z0-9 _\-]*PRIVATE KEY-----",
            )
            .expect("pem regex"),
            Regex::new(r"([a-zA-Z][a-zA-Z0-9+.\-]*://[^/\s:@]+:)([^@\s/]+)@")
                .expect("url cred regex"),
            Regex::new(r"[a-zA-Z0-9._%+\-]+@[a-zA-Z0-9.\-]+\.[a-zA-Z]{2,}")
                .expect("email regex"),
            Regex::new(r"(?i)(?:bearer\s+|api[_-]?key[=:]\s*|token[=:]\s*)[A-Za-z0-9+/\-_]{32,}")
                .expect("api_key regex"),
            Regex::new(
                r"(?x)\b(?:AIza[0-9A-Za-z\-_]{35}|hf_[A-Za-z0-9]{30,}\b|npm_[A-Za-z0-9]{30,}\b|gh[posru]_[A-Za-z0-9]{20,}\b|github_pat_[A-Za-z0-9_]{20,}\b|sk-ant-[A-Za-z0-9\-_]{20,}|(?:sk-(?:proj-|svcacct-)?|gsk-|pk-|ak-)[A-Za-z0-9_\-]{20,}|xox[baprs]-[A-Za-z0-9-]{10,}|AKIA[0-9A-Z]{16}\b|eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,})",
            )
            .expect("token regex"),
            Regex::new(
                r"\b(?:127|10|192\.168|172\.(?:1[6-9]|2\d|3[01]))\.\d{1,3}\.\d{1,3}(?:\.\d{1,3})?\b",
            )
            .expect("ipv4 regex"),
            Regex::new(r"(?:/(?:Users|home)/[^/\s]+|[A-Za-z]:\\Users\\[^\\\s]+)")
                .expect("home regex"),
        ]
    });
    let s = re_pem.replace_all(&stage2, "[REDACTED_PEM]");
    let s = re_url_cred.replace_all(&s, "${1}[REDACTED_CRED]@");
    let s = re_email.replace_all(&s, "[REDACTED_EMAIL]");
    let s = re_api_key.replace_all(&s, "[REDACTED_KEY]");
    let s = re_token.replace_all(&s, "[REDACTED_KEY]");
    let s = re_ipv4.replace_all(&s, "[REDACTED_IP]");
    re_home.replace_all(&s, "~[REDACTED_PATH]").into_owned()
}

pub(super) fn bound_ephemeral_output(
    text: &str,
    session_id: &str,
    block_id: u64,
    head_lines: usize,
    tail_lines: usize,
    max_bytes: usize,
) -> (String, bool, usize) {
    let lines: Vec<&str> = text.lines().collect();
    let keep = head_lines.saturating_add(tail_lines);
    if text.len() <= max_bytes && (keep == 0 || lines.len() <= keep) {
        return (text.to_string(), false, 0);
    }
    let omitted = lines.len().saturating_sub(keep);
    let mut out = if omitted > 0 && keep > 0 {
        let head = lines[..head_lines.min(lines.len())].join("\n");
        let tail = lines[lines.len().saturating_sub(tail_lines)..].join("\n");
        let hs = head_lines.min(lines.len()) + 1;
        let te = lines.len().saturating_sub(tail_lines);
        format!(
            "{head}\n... [{omitted} lines omitted — call vox_terminal_read(session_id=\"{session_id}\", block_id={block_id}, line_start={hs}, line_end={te})] ...\n{tail}"
        )
    } else {
        text.to_string()
    };
    if out.len() > max_bytes {
        let mut cut = max_bytes;
        while cut > 0 && !out.is_char_boundary(cut) {
            cut -= 1;
        }
        out.truncate(cut);
    }
    (out, true, omitted)
}

#[path = "terminal_bridge_ephemeral.rs"]
mod ephemeral;
pub use ephemeral::EphemeralTerminalBridge;
