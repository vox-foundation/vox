//! Live terminal slash-command dispatcher (`/model`, `/budget`, `/skills`, `/memory`,
//! `/context`, `/clear`, `/compact`, `/help`, `/nu`, `/sh`, `/ai`).

use std::sync::Arc;

use serde_json::{Value, json};
use vox_terminal_core::commands::CommandRegistry;
use vox_terminal_core::{BlockKind, BlockOrigin, BlockStatus, OutputChunk, SessionHost, Stream};

use crate::commands::console_a2a::scrub_for_agent_inbox;
use crate::commands::context_budget::context_budget_impl;
use crate::commands::llm_spend::get_llm_spend_impl;
use crate::commands::memory::{memory_status_impl, recall_impl};
use crate::commands::model_active::{get_active_model_impl, set_active_model_impl};
use crate::remote::calls::{BackendCalls, tool_data};

type Sh = Arc<dyn SessionHost>;
type Res = Result<TermSlashResponse, String>;
const KEEP_RECENT: usize = 3;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TermSlashResponse {
    pub command: String,
    pub block_id: Option<u64>,
    pub delegated_to: Option<String>,
    pub output: String,
}

fn resp(cmd: &str, block_id: Option<u64>, del: Option<&str>, output: String) -> TermSlashResponse {
    TermSlashResponse {
        command: cmd.into(),
        block_id,
        delegated_to: del.map(Into::into),
        output,
    }
}

fn format_skill_rows(d: &Value) -> String {
    let arr = |k| d.get(k).and_then(Value::as_array);
    let list = d
        .as_array()
        .or_else(|| arr("skills"))
        .or_else(|| arr("items"))
        .or_else(|| arr("hits"));
    let Some(list) = list else {
        let s = d.as_str().filter(|s| !s.trim().is_empty());
        return s.unwrap_or("[skills]\nNo skills returned.").to_string();
    };
    if list.is_empty() {
        return "[skills]\nNo matching skills found.".to_string();
    }
    let mut lines = vec![format!("[skills - {} entries]", list.len())];
    for it in list.iter().take(12) {
        let s = |k| it.get(k).and_then(Value::as_str);
        let n = s("name").or_else(|| s("id")).unwrap_or("unnamed");
        let desc = s("description").or_else(|| s("summary")).unwrap_or("");
        lines.push(if desc.is_empty() {
            format!("- {n}")
        } else {
            format!("- {n}: {desc}")
        });
    }
    lines.join("\n")
}

async fn handle_model<B: BackendCalls>(b: &B, args: &str) -> (i32, String) {
    if args.is_empty() {
        let act = get_active_model_impl(b).await.ok().flatten();
        let active = act
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "auto (routed)".into());
        let r = vox_terminal_core::agent::route_agent_prompt("terminal debugging");
        let (mid, tier) = (r.model_id, r.tier);
        let out = format!(
            "[model]\nactive_model: {active}\ndefault_route: {mid} ({tier})\nusage: /model <model_id> | /model route <prompt>"
        );
        return (0, out);
    }
    if let Some(rest) = args
        .strip_prefix("route ")
        .or_else(|| args.strip_prefix("explain "))
    {
        return (
            0,
            vox_terminal_core::agent::route_agent_prompt(rest.trim()).summary(),
        );
    }
    match set_active_model_impl(b, args.to_string()).await {
        Ok(()) => (0, format!("[model]\nactive_model: {args}\nstatus: updated")),
        Err(e) => (1, format!("Failed to update active model: {e}")),
    }
}

pub(crate) async fn handle_budget_with<B, F>(
    b: &B,
    sid: &str,
    args: &str,
    set_cfg: F,
) -> (i32, String)
where
    B: BackendCalls,
    F: FnOnce(String, String) -> Result<(), String>,
{
    let mut note = String::new();
    let mut updated = false;
    if !args.trim().is_empty() {
        let Ok(lim) = args.trim().trim_start_matches('$').parse::<f64>() else {
            return (
                1,
                "Usage: /budget [usd_limit] - e.g. /budget 2.50".to_string(),
            );
        };
        if !lim.is_finite() || lim < 0.0 {
            return (1, "Budget limit must be a non-negative number.".to_string());
        }
        let val = format!("{lim:.2}");
        if let Err(e) = set_cfg("per_session_budget_usd".into(), val.clone()) {
            return (1, format!("Failed to update per-session budget: {e}"));
        }
        updated = true;
        let p = json!({ "user_id": "local_user", "key": "per_session_budget_usd", "value": val });
        let mirror = match b.call_tool("vox_preference_set", p).await {
            Ok(env) => match tool_data::<Value>("vox_preference_set", env) {
                Ok(_) => String::new(),
                Err(e) => format!("remote_preference_mirror: warning ({e})\n"),
            },
            Err(e) => format!("remote_preference_mirror: unavailable ({e})\n"),
        };
        note = format!("updated_per_session_budget_usd: {val}\n{mirror}");
    }
    let ctx_res = context_budget_impl(b, Some(sid.to_string())).await;
    let sp_res = get_llm_spend_impl(b, Some(sid.to_string())).await;
    if !updated && ctx_res.is_err() && sp_res.is_err() {
        return (
            1,
            format!(
                "[budget]\nBudget backend unavailable (context: {}; spend: {}).",
                ctx_res.unwrap_err(),
                sp_res.unwrap_err()
            ),
        );
    }
    let tok = match ctx_res {
        Ok(ctx) => {
            let st = if ctx.strategy.is_empty() {
                "balanced"
            } else {
                ctx.strategy.as_str()
            };
            format!(
                "context_tokens: {} / {} (max {}, threshold {}, strategy {st})",
                ctx.used_tokens, ctx.usable_tokens, ctx.max_context_tokens, ctx.threshold_tokens
            )
        }
        Err(e) => format!("context_tokens: unavailable ({e})"),
    };
    let usd = match sp_res {
        Ok(sp) => format!(
            "session_spend_usd: {:.4} / {:.2}\ndaily_spend_usd: {:.4} / {:.2}\ntotal_spend_usd: {:.4}",
            sp.session_usd,
            sp.per_session_budget_usd,
            sp.day_usd,
            sp.daily_budget_usd,
            sp.total_usd
        ),
        Err(e) => format!("spend_usd: unavailable ({e})"),
    };
    (0, format!("[budget]\n{note}{tok}\n{usd}"))
}

async fn handle_skills<B: BackendCalls>(b: &B, args: &str) -> (i32, String) {
    let (tool, p) = match args.is_empty() {
        true => ("vox_skill_list", json!({})),
        false => ("vox_skill_search", json!({ "query": args })),
    };
    match b.call_tool(tool, p).await {
        Ok(env) => match tool_data::<Value>(tool, env) {
            Ok(v) => (0, format_skill_rows(&v)),
            Err(e) => (1, format!("[skills]\nError: {e}")),
        },
        Err(e) => (1, format!("[skills]\nBackend unavailable: {e}")),
    }
}

async fn handle_memory<B: BackendCalls>(b: &B, args: &str) -> (i32, String) {
    if args.is_empty() {
        return match memory_status_impl(b).await {
            Ok(st) => {
                let c = |k: &str| st.corpus_counts.get(k).copied().unwrap_or(0);
                let dim = st
                    .embedding_dim
                    .map_or_else(|| "unknown".into(), |d| d.to_string());
                let (p, d, ch, r, w) = (c("proj"), c("docs"), c("chats"), c("rules"), c("web"));
                (
                    0,
                    format!(
                        "[memory]\ncorpora: proj={p} docs={d} chats={ch} rules={r} web={w}\nembedding_dim: {dim}"
                    ),
                )
            }
            Err(e) => (1, format!("[memory]\nBackend unavailable: {e}")),
        };
    }
    match recall_impl(b, args, 5).await {
        Ok(hits) if hits.is_empty() => (0, format!("[memory]\nNo recall hits for `{args}`.")),
        Ok(hits) => {
            let mut out = vec![format!("[memory recall - {} hits]", hits.len())];
            for h in hits {
                let (k, s, l, sc, t) = (&h.kind, &h.src, h.line, h.score, &h.text);
                out.push(format!("- [{k}] {s}:{l} (score {sc:.2}): {t}"));
            }
            (0, out.join("\n"))
        }
        Err(e) => (1, format!("[memory]\nRecall failed: {e}")),
    }
}

async fn handle_context<B: BackendCalls>(h: &dyn SessionHost, b: &B, sid: &str) -> (i32, String) {
    let Ok(att) = h.attach(sid, Some(u64::MAX)).await else {
        return (1, format!("[context]\nSession `{sid}` unavailable."));
    };
    let pinned = att.blocks.iter().filter(|x| x.pinned).count();
    let agents = att
        .blocks
        .iter()
        .filter(|x| x.kind == BlockKind::AgentTurn)
        .count();
    let cwd = att.cwd.as_deref().unwrap_or("<unknown>");
    let br = att.git_branch.as_deref().unwrap_or("<none>");
    let budget_line = match context_budget_impl(b, Some(sid.to_string())).await {
        Ok(ctx) => format!(
            "token_budget: {} / {} usable",
            ctx.used_tokens, ctx.usable_tokens
        ),
        Err(_) => "token_budget: unavailable".to_string(),
    };
    let n = att.blocks.len();
    let out = format!(
        "[context]\nsession_id: {sid}\nblocks: {n} total ({pinned} pinned, {agents} agent)\ncwd: {cwd}\ngit_branch: {br}\n{budget_line}"
    );
    (0, out)
}

async fn handle_compact<B: BackendCalls>(h: Sh, b: &B, port: Option<u16>, sid: &str) -> Res {
    let att = h
        .attach(sid, Some(u64::MAX))
        .await
        .map_err(|e| e.to_string())?;
    let eligible: Vec<_> = att
        .blocks
        .iter()
        .filter(|x| {
            !x.pinned
                && !x.credential_prompt
                && !matches!(x.status, BlockStatus::Running | BlockStatus::AwaitingInput)
        })
        .collect();
    if eligible.len() <= KEEP_RECENT {
        let msg = "Nothing to compact (no eligible older unpinned blocks).".into();
        return Ok(resp("/compact", None, None, msg));
    }
    let cands = &eligible[..eligible.len() - KEEP_RECENT];
    let mut lines = vec![format!("[compact digest - {} blocks]", cands.len())];
    for x in cands {
        let (id, cs) = (x.id.0, scrub_for_agent_inbox(x.input.trim()));
        let ec = x.exit_code.map_or_else(|| "-".into(), |c| c.to_string());
        let hd = scrub_for_agent_inbox(x.plain_output().lines().next().unwrap_or("").trim());
        lines.push(if hd.is_empty() {
            format!("- #{id} `{cs}` (exit {ec})")
        } else {
            format!("- #{id} `{cs}` (exit {ec}): {hd}")
        });
    }
    let offline = lines.join("\n");
    let cfg = super::agent::resolve_terminal_llm_config(b, port, "summarize terminal blocks").await;
    let digest = match cfg {
        Ok(c) => match super::agent::summarize_blocks_with_llm(c, &offline).await {
            Ok(s) if s.starts_with("[compact digest") => s,
            Ok(s) => format!("[compact digest - {} blocks]\n{s}", cands.len()),
            Err(_) => offline,
        },
        Err(_) => offline,
    };
    let bid = h
        .compact_blocks(sid, digest.clone(), KEEP_RECENT)
        .await
        .map_err(|e| e.to_string())?;
    Ok(resp("/compact", bid.map(|x| x.0), None, digest))
}

pub(crate) async fn term_slash_exec_with<B, F>(
    h: Sh,
    b: &B,
    port: Option<u16>,
    tr: &super::agent::AgentTurnTracker,
    sid: String,
    raw_line: String,
    set_cfg: F,
) -> Res
where
    B: BackendCalls,
    F: FnOnce(String, String) -> Result<(), String>,
{
    let t = raw_line.trim();
    let norm = if t.starts_with('/') {
        t.to_string()
    } else {
        format!("/{t}")
    };
    let (cmd_raw, rest) = norm.split_once(char::is_whitespace).unwrap_or((&norm, ""));
    let (cmd, args) = (cmd_raw.to_ascii_lowercase(), rest.trim());

    if cmd == "/ai" && !args.is_empty() {
        let r = super::agent::term_agent_prompt_impl(h, b, port, tr, sid, args.into()).await?;
        return Ok(resp(
            &cmd,
            Some(r.block_id),
            Some("agent"),
            r.proposed_command.unwrap_or_default(),
        ));
    }
    if (cmd == "/sh" || cmd == "/nu") && !args.is_empty() {
        let sub = if cmd == "/sh" {
            format!("!{args}")
        } else {
            format!("/nu {args}")
        };
        let bid = h.submit(&sid, &sub).await.map_err(|e| e.to_string())?;
        let lbl = if cmd == "/sh" {
            "shell command"
        } else {
            "Nushell pipeline"
        };
        let out = format!("Submitted {lbl}: {}", scrub_for_agent_inbox(args));
        return Ok(resp(&cmd, Some(bid.0), Some("session"), out));
    }
    if cmd == "/clear" {
        let kept = h
            .clear_unpinned_blocks(&sid)
            .await
            .map_err(|e| e.to_string())?;
        let out = format!("Cleared unpinned blocks ({kept} pinned/active retained).");
        return Ok(resp(&cmd, None, None, out));
    }
    if cmd == "/compact" {
        return handle_compact(h, b, port, &sid).await;
    }

    let (code, body) = match cmd.as_str() {
        "/help" => {
            let mut lines = vec!["[slash commands]".to_string()];
            for e in CommandRegistry::default().entries() {
                lines.push(format!("- /{}: {}", e.name, e.description));
            }
            (0, lines.join("\n"))
        }
        "/ai" => (1, "Usage: /ai <prompt>".into()),
        "/sh" => (1, "Usage: /sh <command>".into()),
        "/nu" => (
            0,
            "Usage: /nu <pipeline> - e.g. /nu ls | where size > 1kb | to json".into(),
        ),
        "/model" => handle_model(b, args).await,
        "/budget" => handle_budget_with(b, &sid, args, set_cfg).await,
        "/skills" => handle_skills(b, args).await,
        "/memory" => handle_memory(b, args).await,
        "/context" => handle_context(&*h, b, &sid).await,
        _ => (
            1,
            format!("Unknown slash command: {cmd}. Run /help to list available commands."),
        ),
    };
    let s_in = scrub_for_agent_inbox(&norm);
    let bid = h
        .begin_custom_block(&sid, BlockKind::SlashCommand, BlockOrigin::User, s_in)
        .await
        .map_err(|e| e.to_string())?;
    let out = scrub_for_agent_inbox(&body);
    let _ = h
        .append_block_chunk(&sid, bid, OutputChunk::text(Stream::Agent, out.clone()))
        .await;
    let _ = h.finish_custom_block(&sid, bid, code, false).await;
    Ok(resp(&cmd, Some(bid.0), None, out))
}

pub(crate) async fn term_slash_exec_impl<B: BackendCalls>(
    h: Sh,
    b: &B,
    port: Option<u16>,
    tr: &super::agent::AgentTurnTracker,
    sid: String,
    raw_line: String,
) -> Res {
    term_slash_exec_with(
        h,
        b,
        port,
        tr,
        sid,
        raw_line,
        crate::commands::user_config::set_user_config,
    )
    .await
}
