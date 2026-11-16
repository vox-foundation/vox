use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use vox_orchestrator_mcp::gui_ops::dto::knowledge::CorpusCountsDto;
use vox_orchestrator_mcp::gui_ops::dto::orchestrator::ContextBudgetPayload;
use vox_terminal_core::{
    BlockId, BlockKind, BlockStatus, LocalHost, OutputChunk, SessionHost, Stream, TermControlMode,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::agent::{
    AgentTurnTracker, CLAVIS_UNCONFIGURED_DIAG, execute_agent_turn, term_agent_cancel_impl,
    term_agent_explain_fix_impl, term_agent_prompt_impl,
};
use super::slash::{handle_budget_with, term_slash_exec_impl, term_slash_exec_with};
use crate::remote::calls::fake::FakeBackend;

fn sse_body(content: &str) -> String {
    let chunk = json!({ "choices": [{ "delta": { "content": content } }] });
    format!("data: {chunk}\n\ndata: [DONE]\n\n")
}

fn sse_split_body(content: &str, step: usize) -> String {
    let mut out = String::new();
    for piece in content.as_bytes().chunks(step) {
        let s = std::str::from_utf8(piece).unwrap();
        let chunk = json!({ "choices": [{ "delta": { "content": s } }] });
        out.push_str(&format!("data: {chunk}\n\n"));
    }
    out.push_str("data: [DONE]\n\n");
    out
}

#[tokio::test]
async fn unconfigured_agent_turn_records_diagnostic_block() {
    let local = Arc::new(LocalHost::default());
    let sid = local.open_synthetic(100, 30);
    let host: Arc<dyn SessionHost> = local;
    let tr = AgentTurnTracker::default();

    let res = execute_agent_turn(
        host.clone(),
        &tr,
        &sid,
        "? explain rustc E0382".into(),
        vec![],
        Err(CLAVIS_UNCONFIGURED_DIAG.into()),
    )
    .await
    .expect("unconfigured turn returns structured response");

    assert_eq!(res.status, "unconfigured");
    assert_eq!(res.proposed_command, None);
    let blk = host
        .get_block(&sid, BlockId(res.block_id))
        .await
        .unwrap()
        .expect("agent block recorded");
    assert_eq!(blk.kind, BlockKind::AgentTurn);
    assert_eq!(blk.exit_code, Some(1));
    assert!(blk.plain_output().contains("[clavis-unconfigured]"));
    assert!(!blk.plain_output().contains('·'));
}

#[tokio::test]
async fn streaming_agent_prompt_and_explain_fix_via_llm_facade() {
    let server = MockServer::start().await;
    let tok = ["ghp", "_1234567890abcdefghijklmnopqrstuvwx"].concat();
    let long_sk = ["sk-proj-", &"A1b2C3d4E5f6G7h8I9j0".repeat(3)].concat();
    let reply =
        format!("Root cause found ({tok} and {long_sk}).\n```sh\n$ cargo check -p vox-gui\n```\n");
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_split_body(&reply, 9)),
        )
        .mount(&server)
        .await;

    let local = Arc::new(LocalHost::default());
    let sid = local.open_synthetic(100, 30);
    let host: Arc<dyn SessionHost> = local.clone();
    let fb = FakeBackend::new();
    let tr = AgentTurnTracker::default();
    let port = Some(server.address().port());

    let turn = term_agent_prompt_impl(
        host.clone(),
        &fb,
        port,
        &tr,
        sid.clone(),
        "how do I check vox-gui?".into(),
    )
    .await
    .expect("prompt turn");
    assert_eq!(turn.status, "ok");
    assert_eq!(turn.provider.as_deref(), Some("ollama"));
    assert_eq!(
        turn.proposed_command.as_deref(),
        Some("cargo check -p vox-gui")
    );
    let blk = host
        .get_block(&sid, BlockId(turn.block_id))
        .await
        .unwrap()
        .unwrap();
    let out = blk.plain_output();
    assert!(out.starts_with("[agent-model: ollama - "));
    assert!(
        !out.contains(&tok) && !out.contains(&long_sk),
        "split secrets must be scrubbed"
    );
    assert!(!out.contains('·'), "zero middle dots allowed");

    let failed_id = host.submit(&sid, "!false").await.unwrap();
    let _ = host
        .append_block_chunk(
            &sid,
            failed_id,
            OutputChunk::text(Stream::Stderr, "compile error"),
        )
        .await;
    let fix = term_agent_explain_fix_impl(
        host.clone(),
        &fb,
        port,
        &tr,
        sid.clone(),
        failed_id.0,
        "fix".into(),
    )
    .await
    .expect("explain/fix turn");
    assert_eq!(fix.status, "ok");
    assert_eq!(
        fix.proposed_command.as_deref(),
        Some("cargo check -p vox-gui")
    );
}

#[tokio::test]
async fn agent_turn_cancel_and_control_epoch_preemption_close_block_cancelled() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/event-stream")
                .set_body_string(sse_body("partial\n"))
                .set_delay(Duration::from_secs(5)),
        )
        .mount(&server)
        .await;

    let local = Arc::new(LocalHost::default());
    let sid = local.open_synthetic(100, 30);
    let host: Arc<dyn SessionHost> = local;
    host.set_control_mode(&sid, TermControlMode::Autopilot)
        .await
        .unwrap();
    let tr = Arc::new(AgentTurnTracker::default());
    let port = Some(server.address().port());

    let (h_t, tr_t, s_t) = (host.clone(), tr.clone(), sid.clone());
    let join = tokio::spawn(async move {
        let fb = FakeBackend::new();
        term_agent_prompt_impl(h_t, &fb, port, &tr_t, s_t, "long task".into()).await
    });
    tokio::time::sleep(Duration::from_millis(60)).await;
    host.set_control_mode(&sid, TermControlMode::Manual)
        .await
        .unwrap();
    let res = tokio::time::timeout(Duration::from_secs(1), join)
        .await
        .expect("connect wait must abort promptly on epoch preemption")
        .unwrap()
        .expect("preempted turn");
    assert_eq!(res.status, "cancelled");
    let blk = host
        .get_block(&sid, BlockId(res.block_id))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        (blk.status, blk.exit_code),
        (BlockStatus::Cancelled, Some(130))
    );

    let (h_c, tr_c, s_c) = (host.clone(), tr.clone(), sid.clone());
    let join_c = tokio::spawn(async move {
        let fb = FakeBackend::new();
        term_agent_prompt_impl(h_c, &fb, port, &tr_c, s_c, "cancel connect".into()).await
    });
    let cancel_bid = loop {
        let att = host.attach(&sid, Some(u64::MAX)).await.unwrap();
        if let Some(b) = att
            .blocks
            .iter()
            .rev()
            .find(|b| b.status == BlockStatus::Running)
        {
            break b.id.0;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    };
    assert!(
        term_agent_cancel_impl(host.clone(), &tr, sid.clone(), cancel_bid)
            .await
            .unwrap()
    );
    let res_c = tokio::time::timeout(Duration::from_secs(1), join_c)
        .await
        .expect("connect wait must abort promptly on term_agent_cancel")
        .unwrap()
        .expect("cancelled turn");
    assert_eq!(res_c.status, "cancelled");
    assert_eq!(
        host.get_block(&sid, BlockId(cancel_bid))
            .await
            .unwrap()
            .unwrap()
            .status,
        BlockStatus::Cancelled
    );
}

#[tokio::test]
async fn live_slash_commands_and_compact_digest_execute_end_to_end() {
    let server = MockServer::start().await;
    let chat_json = json!({
        "choices": [{ "message": { "content": "- summarized older shell blocks" } }],
        "usage": { "prompt_tokens": 12, "completion_tokens": 6 }
    });
    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(chat_json))
        .mount(&server)
        .await;

    let mut counts = CorpusCountsDto::default();
    counts.proj = 7;
    let ctx_payload = ContextBudgetPayload {
        used_tokens: 1200,
        usable_tokens: 16000,
        max_context_tokens: 32000,
        threshold_tokens: 14000,
        strategy: "balanced".into(),
        ..Default::default()
    };
    let fb = FakeBackend::new()
        .on_get(
            "/models/preferences",
            Ok(json!({ "v": 1, "data": {
                "active_model": "mens-qwen",
                "routing_priority": { "efficiency": 25, "precision": 30, "latency": 15, "availability": 15, "balance": 10, "mobile": 5 },
                "selection_policy": "{}"
            }})),
        )
        .on_tool("vox_models_preference_set", Ok(json!({ "success": true, "data": "ok" })))
        .on_get("/orchestrator/context-budget?session_id=s-slash", Ok(json!({ "v": 1, "data": ctx_payload })))
        .on_get(
            "/llm/spend?session_id=s-slash",
            Ok(json!({ "v": 1, "data": {
                "sessionUsd": 0.12, "dayUsd": 0.45, "totalUsd": 3.20,
                "dailyBudgetUsd": 5.0, "perSessionBudgetUsd": 2.5
            }})),
        )
        .on_tool(
            "vox_skill_list",
            Ok(json!({ "success": true, "data": [{ "name": "vox-check", "description": "Typecheck vox" }] })),
        )
        .on_tool(
            "vox_skill_search",
            Ok(json!({ "success": true, "data": { "skills": [{ "name": "git-flow", "description": "Git helper" }] } })),
        )
        .on_get("/knowledge/corpus-counts", Ok(json!({ "v": 1, "data": counts })))
        .on_get(
            "/knowledge/recall?q=borrow&limit=5",
            Ok(json!({ "v": 1, "data": [{ "src": "notes.md", "line": 4, "score": 0.91, "kind": "doc", "text": "borrow fix" }] })),
        );

    let local = Arc::new(LocalHost::default());
    let (sid, sid2) = (local.open_synthetic(100, 30), local.open_synthetic(100, 30));
    let host: Arc<dyn SessionHost> = local;
    let tr = AgentTurnTracker::default();

    for cmd in [
        "/help",
        "/model",
        "/model route fix rustc lifetime",
        "/model claude-sonnet-4-5",
        "/skills",
        "/skills git",
        "/memory",
        "/memory borrow",
        "/context",
    ] {
        let r = term_slash_exec_impl(host.clone(), &fb, None, &tr, sid.clone(), cmd.into())
            .await
            .unwrap_or_else(|e| panic!("slash {cmd} failed: {e}"));
        assert!(!r.output.is_empty(), "empty output on {cmd}");
        assert!(!r.output.contains('·'), "middle dot in {cmd}: {}", r.output);
    }

    let fb_sid = FakeBackend::new()
        .on_tool("vox_preference_set", Ok(json!({ "success": true, "data": "ok" })))
        .on_get(
            &format!("/orchestrator/context-budget?session_id={sid}"),
            Ok(json!({ "v": 1, "data": ContextBudgetPayload::default() })),
        )
        .on_get(
            &format!("/llm/spend?session_id={sid}"),
            Ok(json!({ "v": 1, "data": { "sessionUsd": 0.1, "dayUsd": 0.2, "totalUsd": 1.0, "dailyBudgetUsd": 5.0, "perSessionBudgetUsd": 2.5 } })),
        );
    let cfg_writes = std::sync::Mutex::new(Vec::<(String, String)>::new());
    let budget = term_slash_exec_with(
        host.clone(),
        &fb_sid,
        None,
        &tr,
        sid.clone(),
        "/budget 2.50".into(),
        |k, v| {
            cfg_writes.lock().unwrap().push((k, v));
            Ok(())
        },
    )
    .await
    .unwrap();
    assert!(
        budget
            .output
            .contains("updated_per_session_budget_usd: 2.50")
    );
    assert_eq!(
        cfg_writes.lock().unwrap().as_slice(),
        &[("per_session_budget_usd".into(), "2.50".into())]
    );
    assert!(
        fb_sid
            .calls()
            .iter()
            .any(|(k, v)| k == "tool:vox_preference_set" && v["key"] == "per_session_budget_usd")
    );

    let (fail_code, fail_msg) =
        handle_budget_with(&fb_sid, &sid, "2.50", |_, _| Err("disk full".into())).await;
    assert_eq!(fail_code, 1);
    assert!(fail_msg.contains("disk full"));
    let fb_down = FakeBackend::new();
    let (down_code, down_msg) = handle_budget_with(&fb_down, &sid, "", |_, _| Ok(())).await;
    assert_eq!(down_code, 1);
    assert!(down_msg.contains("unavailable"));
    let ctx_down = term_slash_exec_impl(
        host.clone(),
        &fb_down,
        None,
        &tr,
        sid.clone(),
        "/context".into(),
    )
    .await
    .unwrap();
    assert!(ctx_down.output.contains("token_budget: unavailable"));

    for i in 0..3 {
        host.submit(&sid2, &format!("!echo c-{i}")).await.unwrap();
    }
    let pin2 = host.submit(&sid2, "!echo pin2").await.unwrap();
    host.pin_block(&sid2, pin2, true).await.unwrap();
    let _run2 = host
        .begin_custom_block(
            &sid2,
            BlockKind::AgentTurn,
            vox_terminal_core::BlockOrigin::User,
            "? r".into(),
        )
        .await
        .unwrap();
    let no_cmp = term_slash_exec_impl(host.clone(), &fb_sid, None, &tr, sid2, "/compact".into())
        .await
        .unwrap();
    assert!(no_cmp.output.contains("Nothing to compact"));
    for i in 0..5 {
        let bid = host.submit(&sid, &format!("!echo row-{i}")).await.unwrap();
        if i == 0 {
            host.pin_block(&sid, bid, true).await.unwrap();
        }
    }
    let compact = term_slash_exec_impl(
        host.clone(),
        &fb_sid,
        Some(server.address().port()),
        &tr,
        sid.clone(),
        "/compact".into(),
    )
    .await
    .unwrap();
    assert!(
        compact.output.starts_with("[compact digest - ")
            && compact.output.contains("summarized older shell blocks")
            && compact.block_id.is_some()
    );

    let clear = term_slash_exec_impl(
        host.clone(),
        &fb_sid,
        None,
        &tr,
        sid.clone(),
        "/clear".into(),
    )
    .await
    .unwrap();
    assert!(clear.output.contains("Cleared unpinned blocks"));

    let bad = term_slash_exec_impl(
        host.clone(),
        &fb_sid,
        None,
        &tr,
        sid.clone(),
        "/bogus".into(),
    )
    .await
    .unwrap();
    assert_eq!(
        host.get_block(&sid, BlockId(bad.block_id.unwrap()))
            .await
            .unwrap()
            .unwrap()
            .exit_code,
        Some(1)
    );
}
