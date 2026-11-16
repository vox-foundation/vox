//! Model-agnostic terminal agent turns (`?prompt`, `/ai`, `Explain Failure`, `Fix Command`).
//! Routes 100% of inference calls through `vox_actor_runtime::llm` + Clavis/local-MENS discovery.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use futures_util::StreamExt;
use tokio::sync::watch;
use vox_actor_runtime::llm::{LlmChatMessage, LlmConfig, llm_chat, llm_stream_activity};
use vox_actor_runtime::{ActivityOptions, ActivityResult};
use vox_terminal_core::{Block, BlockId, BlockKind, BlockOrigin, OutputChunk, SessionHost, Stream};

pub(crate) use super::approvals::resolve_terminal_llm_config;
#[cfg(test)]
pub(crate) use super::approvals::{CLAVIS_UNCONFIGURED_DIAG, resolve_terminal_llm_config_with};
use crate::commands::console_a2a::scrub_for_agent_inbox;
use crate::remote::calls::BackendCalls;

type Sh = Arc<dyn SessionHost>;
type Res = Result<TermAgentTurnResponse, String>;
type CfgRes = Result<LlmConfig, String>;
type Tgt<'a> = (&'a dyn SessionHost, &'a str, BlockId);
const MAX_SLICE: usize = 8_192;
const PRE_SCRUB: usize = 24_576;
const STREAM_WH_MIN: usize = 256;
const SH_LANGS: &[&str] = &[
    "",
    "sh",
    "bash",
    "zsh",
    "shell",
    "pwsh",
    "powershell",
    "nu",
    "vox",
];

#[derive(Default)]
pub struct AgentTurnTracker {
    in_flight: Mutex<HashMap<(String, u64), watch::Sender<bool>>>,
}

impl AgentTurnTracker {
    fn map(&self) -> MutexGuard<'_, HashMap<(String, u64), watch::Sender<bool>>> {
        self.in_flight.lock().unwrap_or_else(|e| e.into_inner())
    }
    pub fn register(&self, sid: &str, bid: u64) -> watch::Receiver<bool> {
        let (tx, rx) = watch::channel(false);
        self.map().insert((sid.to_string(), bid), tx);
        rx
    }
    pub fn finish(&self, sid: &str, bid: u64) {
        self.map().remove(&(sid.to_string(), bid));
    }
    pub fn cancel(&self, sid: &str, bid: u64) -> bool {
        let k = (sid.to_string(), bid);
        self.map().remove(&k).map(|tx| tx.send(true)).is_some()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TermAgentTurnResponse {
    pub block_id: u64,
    pub status: String,
    pub proposed_command: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
}

fn chat_msg(role: &str, content: impl Into<String>) -> LlmChatMessage {
    let mut m = LlmChatMessage::default();
    (m.role, m.content) = (role.to_string(), content.into());
    m
}

pub(crate) fn build_explain_fix_messages(
    m: &str,
    b: &Block,
) -> Result<Vec<LlmChatMessage>, String> {
    if b.credential_prompt {
        return Err("Credential-prompt block cannot be sent to the terminal agent.".to_string());
    }
    let instr = match m.trim().to_ascii_lowercase().as_str() {
        "explain" => {
            "Explain the root cause concisely in Markdown with one fenced `sh` or `vox` block when helpful."
        }
        "fix" => {
            "Diagnose this failed command briefly and provide one fenced `sh` or `vox` block with the exact fix."
        }
        other => return Err(format!("Unsupported explain/fix mode: {other}")),
    };
    let (mut out_r, mut err_r) = (String::new(), String::new());
    for c in &b.output {
        if c.stream == Stream::Stderr {
            err_r.push_str(&c.text);
        } else {
            out_r.push_str(&c.text);
        }
    }
    let cap = |s: &str| {
        let pre: std::borrow::Cow<'_, str> = if s.len() <= PRE_SCRUB {
            s.into()
        } else {
            let h = PRE_SCRUB / 2;
            let he = s.floor_char_boundary(h);
            let ts = s.ceil_char_boundary(s.len().saturating_sub(h));
            let hw = s[..he].rfind(char::is_whitespace).unwrap_or(he);
            let tw = s[ts..].find(char::is_whitespace).map_or(ts, |o| ts + o);
            format!("{}\n...[truncated]...\n{}", &s[..hw], &s[tw..]).into()
        };
        let sc = scrub_for_agent_inbox(&pre);
        if sc.len() <= MAX_SLICE {
            return sc;
        }
        let he = sc.floor_char_boundary(4_096);
        let ts = sc.ceil_char_boundary(sc.len().saturating_sub(4_096));
        format!("{}\n...[truncated]...\n{}", &sc[..he], &sc[ts..])
    };
    let opt = |o: Option<&str>, d: &str| o.map_or_else(|| d.into(), scrub_for_agent_inbox);
    let cmd = scrub_for_agent_inbox(&b.input[..b.input.floor_char_boundary(MAX_SLICE)]);
    let (stdout, stderr) = (cap(&out_r), cap(&err_r));
    let cwd = opt(b.cwd.as_deref(), "<unknown>");
    let br = opt(b.git_branch.as_deref(), "<none>");
    let code = b
        .exit_code
        .map_or_else(|| "unknown".into(), |c| c.to_string());
    let sys = format!("You are Vox Terminal Agent inside Vox Axis. {instr} Never echo secrets.");
    let hdr = format!("Command: {cmd}\nExit code: {code}\nCWD: {cwd}\nGit branch: {br}");
    Ok(vec![
        chat_msg("system", sys),
        chat_msg(
            "user",
            format!("{hdr}\n\nStdout:\n{stdout}\n\nStderr:\n{stderr}"),
        ),
    ])
}

pub fn extract_proposed_command(markdown: &str) -> Option<String> {
    for line in markdown.lines() {
        if let Some(rest) = line.trim().strip_prefix("VOX_PROPOSED_COMMAND:") {
            let cmd = rest.trim().trim_start_matches("$ ").trim();
            if !cmd.is_empty() {
                return Some(cmd.to_string());
            }
        }
    }
    let parts: Vec<&str> = markdown.split("```").collect();
    for (idx, part) in parts.iter().enumerate() {
        if idx % 2 == 1 && idx + 1 < parts.len() {
            let (lang_line, body) = part.split_once('\n').unwrap_or(("", part));
            let ok = SH_LANGS.contains(&lang_line.trim().to_ascii_lowercase().as_str());
            let t = body.trim();
            let out = if t.contains('\n') {
                t
            } else {
                t.trim_start_matches("$ ")
            }
            .trim();
            if ok && !out.is_empty() {
                return Some(out.to_string());
            }
        }
    }
    None
}

pub(crate) async fn summarize_blocks_with_llm(cfg: LlmConfig, b: &str) -> Result<String, String> {
    let sys = "Summarize these terminal blocks into a concise pure-ASCII Markdown bullet digest.";
    let msgs = vec![chat_msg("system", sys), chat_msg("user", b)];
    match llm_chat(&ActivityOptions::new().with_timeout_secs(45), msgs, cfg).await {
        ActivityResult::Ok(Ok(r)) if !r.content.trim().is_empty() => {
            Ok(scrub_for_agent_inbox(r.content.trim()))
        }
        ActivityResult::Ok(Err(e)) => Err(e),
        ActivityResult::Failed(e) => Err(e.to_string()),
        _ => Err("compaction summary unavailable".to_string()),
    }
}

async fn say(t: Tgt<'_>, text: impl Into<String>) {
    let ch = OutputChunk::text(Stream::Agent, text.into());
    let _ = t.0.append_block_chunk(t.1, t.2, ch).await;
}

async fn done(t: Tgt<'_>, code: i32, cmd: Option<String>, meta: Option<(&str, &str)>) -> Res {
    let st = match (code, meta.is_some()) {
        (0, _) => "ok",
        (130, _) => "cancelled",
        (_, false) => "unconfigured",
        _ => "failed",
    };
    let _ = t.0.finish_custom_block(t.1, t.2, code, code == 130).await;
    Ok(TermAgentTurnResponse {
        block_id: t.2.0,
        status: st.into(),
        proposed_command: cmd,
        provider: meta.map(|(p, _)| p.into()),
        model: meta.map(|(_, m)| m.into()),
    })
}

pub(crate) fn take_safe_stream_flush(buf: &mut String) -> Option<String> {
    let cut = if let Some(nl) = buf.rfind('\n') {
        nl + 1
    } else if buf.len() >= STREAM_WH_MIN {
        let ws = buf.rfind(|c: char| c.is_ascii_whitespace())?;
        let mut c = ws + 1;
        let lc = buf[..c].to_ascii_lowercase();
        if ["bearer ", "key= ", "key: ", "token= ", "token: "]
            .iter()
            .any(|p| lc.ends_with(p))
        {
            c = buf[..ws]
                .rfind(|ch: char| ch.is_ascii_whitespace())
                .map_or(0, |p| p + 1);
        }
        c
    } else {
        return None;
    };
    (cut > 0).then(|| buf.drain(..cut).collect())
}

async fn cancel_done(tr: &AgentTurnTracker, t: Tgt<'_>, meta: Option<(&str, &str)>) -> Res {
    tr.finish(t.1, t.2.0);
    done(t, 130, None, meta).await
}

pub(crate) async fn execute_agent_turn(
    h: Sh,
    tr: &AgentTurnTracker,
    sid: &str,
    input: String,
    msgs: Vec<LlmChatMessage>,
    cfg_res: CfgRes,
) -> Res {
    let ep = h.control_epoch(sid).await.unwrap_or(0);
    let orig = BlockOrigin::Agent {
        agent_id: "terminal-agent".into(),
    };
    let bid = h
        .begin_custom_block(
            sid,
            BlockKind::AgentTurn,
            orig,
            scrub_for_agent_inbox(&input),
        )
        .await
        .map_err(|e| e.to_string())?;
    let t: Tgt<'_> = (&*h, sid, bid);
    let Ok(cfg) = cfg_res else {
        say(t, cfg_res.unwrap_err()).await;
        return done(t, 1, None, None).await;
    };
    let (prov, mdl) = (cfg.provider.clone(), cfg.model.clone());
    let meta = Some((prov.as_str(), mdl.as_str()));
    say(t, format!("[agent-model: {prov} - {mdl}]\n")).await;

    let mut rx = tr.register(sid, bid.0);
    let opts = ActivityOptions::new().with_timeout_secs(90);
    let mut tick = tokio::time::interval(std::time::Duration::from_millis(100));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let conn = llm_stream_activity(&opts, msgs, cfg);
    tokio::pin!(conn);
    let mut stream = loop {
        if *rx.borrow() || h.control_epoch(sid).await.unwrap_or(ep) != ep {
            return cancel_done(tr, t, meta).await;
        }
        tokio::select! {
            r = rx.changed() => if *rx.borrow() || r.is_err() { return cancel_done(tr, t, meta).await; },
            _ = tick.tick() => if h.control_epoch(sid).await.unwrap_or(ep) != ep { return cancel_done(tr, t, meta).await; },
            res = &mut conn => match res {
                ActivityResult::Ok(s) => break s,
                ActivityResult::Cancelled => return cancel_done(tr, t, meta).await,
                ActivityResult::Failed(e) => {
                    tr.finish(sid, bid.0);
                    say(t, format!("[stream-error] {}", scrub_for_agent_inbox(&e.to_string()))).await;
                    return done(t, 1, None, meta).await;
                }
            }
        }
    };

    let (mut buf, mut full, mut stop, mut s_err) = (String::new(), String::new(), false, None);
    loop {
        if *rx.borrow() || h.control_epoch(sid).await.unwrap_or(ep) != ep {
            stop = true;
            break;
        }
        tokio::select! {
            r = rx.changed() => if *rx.borrow() || r.is_err() { stop = true; break; },
            _ = tick.tick() => if h.control_epoch(sid).await.unwrap_or(ep) != ep { stop = true; break; },
            item = stream.next() => match item {
                None => break,
                Some(Err(e)) => { s_err = Some(e); break; }
                Some(Ok(d)) => {
                    buf.push_str(&d);
                    if let Some(chunk) = take_safe_stream_flush(&mut buf) {
                        let s = scrub_for_agent_inbox(&chunk);
                        full.push_str(&s);
                        say(t, s).await;
                    }
                }
            }
        }
    }
    tr.finish(sid, bid.0);
    if !buf.is_empty() {
        let s = scrub_for_agent_inbox(&buf);
        full.push_str(&s);
        say(t, s).await;
    }
    if stop || h.control_epoch(sid).await.unwrap_or(ep) != ep {
        return done(t, 130, None, meta).await;
    }
    if let Some(err) = s_err {
        let msg = format!("\n[stream-error] {}", scrub_for_agent_inbox(&err));
        say(t, msg).await;
        return done(t, 1, None, meta).await;
    }
    done(t, 0, extract_proposed_command(&full), meta).await
}

pub(crate) async fn term_agent_prompt_impl<B: BackendCalls>(
    h: Sh,
    b: &B,
    port: Option<u16>,
    tr: &AgentTurnTracker,
    sid: String,
    prompt: String,
) -> Res {
    if prompt.trim().is_empty() {
        return Err("Agent prompt cannot be empty.".to_string());
    }
    let sc = scrub_for_agent_inbox(prompt.trim());
    let cfg = resolve_terminal_llm_config(b, port, &sc).await;
    let sys = "You are Vox Terminal Agent inside Vox Axis. Answer concisely in Markdown. Place any proposed runnable shell or Vox command inside a single fenced `sh` (or `vox`) block. Never echo secrets.";
    let msgs = vec![chat_msg("system", sys), chat_msg("user", sc.clone())];
    execute_agent_turn(h, tr, &sid, format!("? {sc}"), msgs, cfg).await
}

pub(crate) async fn term_agent_explain_fix_impl<B: BackendCalls>(
    h: Sh,
    b: &B,
    port: Option<u16>,
    tr: &AgentTurnTracker,
    sid: String,
    bid: u64,
    mode: String,
) -> Res {
    let blk = h
        .get_block(&sid, BlockId(bid))
        .await
        .map_err(|e| e.to_string())?;
    let block = blk.ok_or_else(|| format!("Terminal block #{bid} not found."))?;
    let msgs = build_explain_fix_messages(&mode, &block)?;
    let hint = format!("{mode} error {}", scrub_for_agent_inbox(&block.input));
    let cfg = resolve_terminal_llm_config(b, port, &hint).await;
    let inp = format!("? {} block #{bid}", mode.trim().to_ascii_lowercase());
    execute_agent_turn(h, tr, &sid, inp, msgs, cfg).await
}

pub(crate) async fn term_agent_cancel_impl(
    h: Sh,
    tr: &AgentTurnTracker,
    sid: String,
    b: u64,
) -> Result<bool, String> {
    if tr.cancel(&sid, b) {
        return Ok(true);
    }
    let res = h.finish_custom_block(&sid, BlockId(b), 130, true).await;
    res.map(|c| c.is_some()).map_err(|e| e.to_string())
}
