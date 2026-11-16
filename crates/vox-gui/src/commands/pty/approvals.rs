//! Terminal MENS flywheel feedback persistence (`term_block_feedback`), `JournalSink`
//! initialization, attachment DTO conversion, and Clavis/local-MENS config resolution.

use std::net::Ipv4Addr;
use std::path::Path;
use std::sync::Arc;

use vox_actor_runtime::llm::LlmConfig;
use vox_orchestrator_mcp::gui_ops::dto::models::ProviderStatusDto;
use vox_secrets::{SecretId, resolve_secret};
use vox_terminal_core::{
    Attachment, Block, BlockFeedbackVerdict, BlockId, DaemonHost, LocalHost, SessionEvent,
    SessionHost, Utf8StreamDecoder,
};

use crate::commands::console_a2a::scrub_for_agent_inbox;
use crate::commands::model_active::get_active_model_impl;
use crate::remote::calls::BackendCalls;

pub(crate) const CLAVIS_UNCONFIGURED_DIAG: &str = "[clavis-unconfigured] No LLM provider or local MENS endpoint configured.\nRun `vox secrets doctor` or configure a key with `vox clavis set <SECRET_ID>` (OpenRouterApiKey, AnthropicApiKey, GeminiApiKey, OpenaiApiKey, GroqApiKey) or start local MENS serving in Vox Axis.";

fn tuned_cfg(p: &str, m: impl Into<String>, url: Option<String>, key: Option<String>) -> LlmConfig {
    let mut cfg = LlmConfig::direct(p, m);
    (cfg.max_tokens, cfg.temperature) = (Some(1536), Some(0.2));
    (cfg.base_url, cfg.api_key) = (url, key);
    cfg
}

fn norm_local_url(raw: &str) -> String {
    let t = raw.trim().trim_end_matches('/');
    match (t.ends_with("/chat/completions"), t.ends_with("/v1")) {
        (true, _) => t.to_string(),
        (false, true) => format!("{t}/chat/completions"),
        (false, false) => format!("{t}/v1/chat/completions"),
    }
}

fn cloud_model<'a>(prov: &str, r_prov: &str, r_id: &'a str) -> &'a str {
    match prov {
        "openrouter" if r_id.is_empty() => "openrouter/auto",
        "openrouter" => r_id,
        _ if r_prov.eq_ignore_ascii_case(prov) && !r_id.is_empty() => r_id,
        "anthropic" => "claude-sonnet-4-5",
        "gemini" => "gemini-2.5-flash",
        "openai" => "gpt-4o-mini",
        "groq" => "llama-3.3-70b-versatile",
        _ => r_id,
    }
}

fn explicit_cloud_provider(model: &str) -> Option<(SecretId, &'static str)> {
    let l = model.trim().to_ascii_lowercase();
    if l.starts_with("anthropic/") || l.starts_with("claude-") {
        Some((SecretId::AnthropicApiKey, "anthropic"))
    } else if l.starts_with("gemini/") || l.starts_with("gemini-") {
        Some((SecretId::GeminiApiKey, "gemini"))
    } else if l.starts_with("openai/")
        || l.starts_with("gpt-")
        || l.starts_with("o1-")
        || l.starts_with("o3-")
    {
        Some((SecretId::OpenaiApiKey, "openai"))
    } else if l.starts_with("openrouter/") {
        Some((SecretId::OpenRouterApiKey, "openrouter"))
    } else if l.starts_with("groq/") {
        Some((SecretId::GroqApiKey, "groq"))
    } else {
        None
    }
}

pub(crate) fn resolve_terminal_llm_config_with<F: Fn(SecretId) -> Option<String>>(
    act_in: Option<&str>,
    port: Option<u16>,
    st: &[ProviderStatusDto],
    sec: F,
    prompt: &str,
) -> Result<LlmConfig, String> {
    use SecretId::*;
    let act = act_in.map(str::trim).filter(|s| !s.is_empty());
    if let Some(model) = act
        && let Some((key_id, prov)) = explicit_cloud_provider(model)
    {
        if let Some(k) = sec(key_id).filter(|v| !v.trim().is_empty()) {
            return Ok(tuned_cfg(prov, model, None, Some(k)));
        }
        if prov != "openrouter"
            && model.contains('/')
            && let Some(k) = sec(OpenRouterApiKey).filter(|v| !v.trim().is_empty())
        {
            return Ok(tuned_cfg("openrouter", model, None, Some(k)));
        }
    }
    let loc_m = act.unwrap_or("mens-local");
    let routed = vox_terminal_core::agent::route_agent_prompt(prompt);
    if let Some(p) = port {
        let url = format!("http://{}:{p}/v1/chat/completions", Ipv4Addr::LOCALHOST);
        return Ok(tuned_cfg("ollama", loc_m, Some(url), None));
    }
    for id in [
        VoxPopuliLocalOllamaUrl,
        OllamaUrl,
        VoxOrchestratorPopuliInferenceBaseUrl,
    ] {
        if let Some(u) = sec(id).filter(|v| !v.trim().is_empty()) {
            return Ok(tuned_cfg("ollama", loc_m, Some(norm_local_url(&u)), None));
        }
    }
    if let Some(s) = st
        .iter()
        .find(|x| x.is_local && x.local_reachable == Some(true))
    {
        let vox = s.provider.eq_ignore_ascii_case("VoxLocal");
        let fb = if vox { "mens-local" } else { "llama3.2" };
        let m = act
            .map(String::from)
            .or_else(|| s.local_models.first().cloned());
        let cands = vox_config::inference::vox_local_endpoint_probe_candidates();
        let base = (cands.len() == 1)
            .then(|| cands.into_iter().next())
            .flatten()
            .unwrap_or_else(|| {
                vox_config::inference::VOX_LOCAL_ENDPOINT_OLLAMA_CONFLICT_ALT.into()
            });
        let prov = if vox { "voxlocal" } else { "ollama" };
        let url = vox.then(|| norm_local_url(&base));
        return Ok(tuned_cfg(prov, m.unwrap_or_else(|| fb.into()), url, None));
    }
    for (id, p) in [
        (OpenRouterApiKey, "openrouter"),
        (AnthropicApiKey, "anthropic"),
        (GeminiApiKey, "gemini"),
        (OpenaiApiKey, "openai"),
        (GroqApiKey, "groq"),
    ] {
        if let Some(k) = sec(id).filter(|v| !v.trim().is_empty()) {
            let m = act.unwrap_or_else(|| cloud_model(p, &routed.provider, &routed.model_id));
            return Ok(tuned_cfg(p, m, None, Some(k)));
        }
    }
    Err(CLAVIS_UNCONFIGURED_DIAG.to_string())
}

fn env_secret(id: SecretId) -> Option<String> {
    let raw = resolve_secret(id);
    let v = raw.expose()?.trim();
    (!v.is_empty()).then(|| v.to_string())
}

pub(crate) async fn resolve_terminal_llm_config<B: BackendCalls>(
    b: &B,
    p: Option<u16>,
    q: &str,
) -> Result<LlmConfig, String> {
    let act = get_active_model_impl(b).await.ok().flatten();
    let st = match p {
        Some(_) => Vec::new(),
        None => vox_orchestrator_mcp::gui_ops::models::provider_status().await,
    };
    resolve_terminal_llm_config_with(act.as_deref(), p, &st, env_secret, q)
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct TermAttachmentDto {
    pub session_id: String,
    pub blocks: Vec<Block>,
    pub replay_utf8: String,
    pub seq: u64,
    #[serde(default)]
    pub resync_required: bool,
    #[serde(default)]
    pub alt_screen: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct TermOutputPayload {
    pub session_id: String,
    pub seq: u64,
    pub data: String,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct TermEventPayload {
    pub session_id: String,
    pub event: SessionEvent,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TermFeedbackResponse {
    pub recorded: bool,
    pub session_id: String,
    pub block_id: u64,
    pub verdict: String,
}

pub(crate) fn build_local_host_with_journal_at(base_dir: &Path) -> LocalHost {
    match LocalHost::open_default_journal_sink(base_dir, vox_secrets::scrub_resolved_secrets) {
        Ok(sink) => LocalHost::new().with_transcript_sink(sink),
        Err(err) => {
            tracing::warn!(error = %err, path = %base_dir.display(), "terminal journal sink unavailable; using in-memory LocalHost");
            LocalHost::new()
        }
    }
}

pub(crate) fn build_local_host_with_journal() -> LocalHost {
    build_local_host_with_journal_at(Path::new(".vox/terminal-journal"))
}

pub(super) async fn resolve_session_host(
    local_host: &Arc<LocalHost>,
    daemon: &Arc<crate::commands::daemon::PersistentDaemon>,
) -> Arc<dyn SessionHost> {
    if let Ok(addr) = daemon.ensure().await {
        let client = match daemon.token().await {
            Some(tok) => vox_orchestrator::orch_daemon::OrchDaemonClient::with_token(addr, tok),
            None => vox_orchestrator::orch_daemon::OrchDaemonClient::new(addr),
        };
        return Arc::new(DaemonHost::new(client));
    }
    local_host.clone()
}

pub(crate) async fn term_block_feedback_impl(
    host: Arc<dyn SessionHost>,
    session_id: String,
    block_id: u64,
    verdict: String,
    corrected_command: Option<String>,
    original_command: Option<String>,
) -> Result<TermFeedbackResponse, String> {
    let norm = verdict.trim().to_ascii_lowercase();
    let parsed = match norm.as_str() {
        "accepted" => BlockFeedbackVerdict::Accepted,
        "rejected" => BlockFeedbackVerdict::Rejected,
        "corrected" => {
            let raw = corrected_command
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or_else(|| {
                    "Corrected feedback requires a non-empty `corrected_command`.".to_string()
                })?;
            let scrubbed = scrub_for_agent_inbox(raw);
            if scrubbed.trim().is_empty() {
                return Err("Corrected command cannot be empty after scrubbing.".to_string());
            }
            let original_scrubbed = original_command
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(scrub_for_agent_inbox)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            BlockFeedbackVerdict::Corrected {
                corrected_command: scrubbed,
                original_command: original_scrubbed,
            }
        }
        other => {
            return Err(format!(
                "Invalid feedback verdict `{other}` (expected accepted, rejected, or corrected)."
            ));
        }
    };
    let recorded = host
        .record_block_feedback(&session_id, BlockId(block_id), parsed)
        .await
        .map_err(|e| e.to_string())?;
    Ok(TermFeedbackResponse {
        recorded,
        session_id,
        block_id,
        verdict: norm,
    })
}

fn decode_replay_bytes(replay_bytes: &[u8]) -> String {
    let mut dec = Utf8StreamDecoder::new();
    let mut s = dec.push(replay_bytes);
    s.push_str(&dec.flush_lossy());
    s
}

#[allow(dead_code)]
pub fn term_attachment_dto_from_parts(
    sid: &str,
    blocks: Vec<Block>,
    replay: &[u8],
    seq: u64,
) -> TermAttachmentDto {
    TermAttachmentDto {
        session_id: sid.to_string(),
        blocks,
        replay_utf8: decode_replay_bytes(replay),
        seq,
        resync_required: true,
        alt_screen: false,
        cwd: None,
        git_branch: None,
    }
}

pub fn term_attachment_dto_and_decoder(
    sid: &str,
    att: &Attachment,
) -> (TermAttachmentDto, Utf8StreamDecoder) {
    let mut decoder = Utf8StreamDecoder::new();
    let replay_utf8 = decoder.push(&att.replay);
    let dto = TermAttachmentDto {
        session_id: sid.to_string(),
        blocks: att.blocks.clone(),
        replay_utf8,
        seq: att.seq,
        resync_required: att.resync_required,
        alt_screen: att.alt_screen,
        cwd: att.cwd.clone(),
        git_branch: att.git_branch.clone(),
    };
    (dto, decoder)
}

#[allow(dead_code)]
pub fn term_attachment_dto_from_attachment(sid: &str, att: &Attachment) -> TermAttachmentDto {
    TermAttachmentDto {
        session_id: sid.to_string(),
        blocks: att.blocks.clone(),
        replay_utf8: decode_replay_bytes(&att.replay),
        seq: att.seq,
        resync_required: att.resync_required,
        alt_screen: att.alt_screen,
        cwd: att.cwd.clone(),
        git_branch: att.git_branch.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_terminal_core::{AgentExecRequest, OpenSpec};

    #[tokio::test]
    async fn block_feedback_persists_redacted_journal_and_refuses_credential_blocks() {
        let dir = tempfile::tempdir().expect("tempdir");
        let local = Arc::new(build_local_host_with_journal_at(dir.path()));
        let sid = local.open_synthetic(100, 30);
        let host: Arc<dyn SessionHost> = local.clone();

        let b1 = host.submit(&sid, "!echo ok").await.unwrap();
        let acc = term_block_feedback_impl(
            host.clone(),
            sid.clone(),
            b1.0,
            "accepted".into(),
            None,
            None,
        )
        .await
        .unwrap();
        assert!(acc.recorded);
        assert!(
            term_block_feedback_impl(
                host.clone(),
                sid.clone(),
                b1.0,
                "corrected".into(),
                Some("   ".into()),
                None
            )
            .await
            .is_err()
        );

        let tok = ["ghp", "_1234567890abcdefghijklmnopqrstuvwx"].concat();
        let corr = term_block_feedback_impl(
            host.clone(),
            sid.clone(),
            0,
            "corrected".into(),
            Some(format!("cargo test --token {tok}")),
            Some(format!("cargo build --token {tok}")),
        )
        .await
        .unwrap();
        assert!(corr.recorded);

        #[cfg(unix)]
        {
            let pty_sid = local
                .open_with_cwd(OpenSpec::default(), Some(dir.path()))
                .await
                .unwrap();
            let cred_out = local
                .exec_agent_command(AgentExecRequest {
                    session_id: pty_sid.clone(),
                    command: "printf '[sudo] challenge for user: '; read c; printf 'DONE\\n'"
                        .into(),
                    agent_id: "agent-test".into(),
                    wait_ms: 4_000,
                    stall_quiescence_ms: 250,
                    head_lines: 40,
                    tail_lines: 160,
                    max_bytes: 16_384,
                    rationale: None,
                    hitl_approved: true,
                })
                .await
                .unwrap();
            assert!(cred_out.credential_prompt);
            let refused = term_block_feedback_impl(
                host.clone(),
                pty_sid.clone(),
                cred_out.block_id,
                "accepted".into(),
                None,
                None,
            )
            .await
            .unwrap();
            assert!(
                !refused.recorded,
                "credential block must never enter MENS flywheel"
            );
            local.close(&pty_sid).unwrap();
        }

        drop(host);
        drop(local);
        let journal =
            std::fs::read_to_string(dir.path().join("transcript.journal")).expect("journal file");
        let corpus = std::fs::read_to_string(dir.path().join("corpus.jsonl")).expect("corpus file");
        assert!(
            journal.contains("\"accepted\"")
                && journal.contains("\"corrected\"")
                && journal.contains("cargo build --token")
        );
        assert!(
            !journal.contains(&tok) && !corpus.contains(&tok),
            "journal and corpus must never contain raw secret"
        );
    }
}
