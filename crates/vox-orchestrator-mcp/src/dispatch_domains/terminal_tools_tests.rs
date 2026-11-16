use super::*;
use crate::handle_tool_call_with_mode as call_mode;
use async_trait::async_trait;
use parking_lot::Mutex;
use serde_json::{from_value as fv, json};
use std::sync::Arc;
use vox_orchestrator::ApprovalOutcome;
use vox_orchestrator::terminal_bridge::{
    TermBlockDto, TermExecResponse, TermSessionSummary, TerminalBridge,
};

struct MockBridge(Mutex<(String, String, String, u64, Vec<TermBlockDto>)>);

#[async_trait]
impl TerminalBridge for MockBridge {
    async fn exec(&self, r: TermExecRequest) -> anyhow::Result<TermExecResponse> {
        let mut g = self.0.lock();
        if g.0 == "manual" && !r.hitl_approved {
            anyhow::bail!("manual session requires hitl_approved=true");
        }
        if let Some(sub) = r.command.trim().strip_prefix("cd ") {
            g.2 = format!("{}/{}", g.2.trim_end_matches('/'), sub.trim());
        }
        let out = if r.command.trim() == "pwd" {
            g.2.clone()
        } else {
            format!("ok:{}:w={}", r.command, r.wait_ms)
        };
        let st = if r.wait_ms == 0 {
            "running"
        } else {
            "completed"
        };
        g.3 += 1;
        let bid = g.4.len() as u64 + 1;
        let block = fv(json!({"id": bid, "input": r.command, "output": out, "status": st, "exit_code": 0, "cwd": g.2})).unwrap();
        g.4.push(block);
        Ok(fv(json!({"session_id": r.session_id, "block_id": bid, "status": st, "exit_code": 0, "seq": g.3, "output": out, "cwd": g.2, "duration_ms": 1})).unwrap())
    }
    async fn send_input(&self, r: TermSendInputRequest) -> anyhow::Result<()> {
        let mut g = self.0.lock();
        if g.0 == "manual" && !r.hitl_approved {
            anyhow::bail!("manual session send_input requires hitl_approved=true");
        }
        let b = g.4.iter_mut().find(|b| b.id == r.block_id).unwrap();
        if b.credential_prompt {
            anyhow::bail!("CREDENTIAL_PROMPT_USER_ONLY");
        }
        b.output.push_str(&r.text);
        b.status = "ok".into();
        Ok(())
    }
    async fn read(&self, r: TermReadRequest) -> anyhow::Result<TermReadResponse> {
        let g = self.0.lock();
        let mut bs: Vec<_> =
            g.4.iter()
                .filter(|b| r.block_id.is_none_or(|i| b.id == i))
                .cloned()
                .collect();
        if let Some(m) = r.max_blocks
            && bs.len() > m
        {
            bs.drain(0..(bs.len() - m));
        }
        Ok(fv(json!({"session_id": "term-1", "seq": g.3, "cwd": g.2, "security_level": g.1, "control_mode": g.0, "blocks": bs})).unwrap())
    }
    async fn list_sessions(&self) -> anyhow::Result<Vec<TermSessionSummary>> {
        let g = self.0.lock();
        Ok(vec![fv(json!({"session_id": "term-1", "blocks": g.4.len(), "cwd": g.2, "security_level": g.1, "control_mode": g.0})).unwrap()])
    }
    async fn open_session(&self, _: Option<PathBuf>) -> anyhow::Result<String> {
        Ok("term-1".into())
    }
    async fn promote_bg(&self, _: &str, _: u64) -> anyhow::Result<()> {
        Ok(())
    }
    async fn pin_block(&self, _: &str, _: u64, _: bool) -> anyhow::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn terminal_mcp_schemas_scrubbing_sticky_hitl_and_credential_guard() {
    let run = async |s: &ServerState, t: &str, a: Value, m: Option<&str>| -> Value {
        serde_json::from_str(&call_mode(s, t, a, m).await.unwrap()).unwrap()
    };
    let mk = |m: &str, l: &str, c: String, bs: Vec<TermBlockDto>| {
        Arc::new(MockBridge(Mutex::new((m.into(), l.into(), c, 1, bs))))
    };
    let ex = crate::input_schemas::tool_input_schema("vox_terminal_exec");
    assert_eq!(ex["additionalProperties"], false);
    assert_eq!(ex["properties"]["wait_ms"]["maximum"], 120_000);
    let si = crate::input_schemas::tool_input_schema("vox_terminal_send_input");
    assert_eq!(si["required"], json!(["session_id", "data"]));

    let pat = format!("ghp_{}", "A1b2C3d4E5f6G7h8I9j0K1l2M3n4");
    let sc = scrub_terminal_mcp_str(&format!("\x1b[31m/{}/alice/repo\x1b[0m {pat}", "Users"));
    assert!(
        !sc.contains("\x1b") && sc.contains("~[REDACTED_PATH]") && sc.contains("[REDACTED_KEY]")
    );

    let st = ServerState::new_test().await;
    let auto_cwd = format!("/{}/op/ws", "Users");
    st.set_terminal_bridge(mk("autopilot", "l5_autopilot_floor", auto_cwd, vec![]));
    let a1 = json!({"command": "cd sub", "chat_thread_id": "t1"});
    let a2 = json!({"command": "pwd", "chat_thread_id": "t1"});
    let _ = run(&st, "vox_terminal_exec", a1, Some("accept_all")).await;
    let r2 = run(&st, "vox_terminal_exec", a2, Some("accept_all")).await;
    assert!(
        r2["data"]["output"]
            .as_str()
            .unwrap()
            .contains("~[REDACTED_PATH]/ws/sub")
    );
    assert!(st.pending_approvals.list().is_empty());
    let a_rd = json!({"chat_thread_id": "t1", "tail_blocks": 1});
    let rd = run(&st, "vox_terminal_read", a_rd, None).await;
    assert_eq!(rd["data"]["blocks"].as_array().map(Vec::len), Some(1));

    let st_m = ServerState::new_test().await;
    let cb: TermBlockDto = fv(json!({"id": 9, "input": "su", "output": "Password:", "status": "awaiting_input", "credential_prompt": true})).unwrap();
    st_m.set_terminal_bridge(mk("manual", "l3_structured_default", "ws".into(), vec![cb]));
    let pwd_arg = json!({"command": "pwd", "session_id": "term-1"});
    for mode in ["accept_all", "ask"] {
        let cl = st_m.clone();
        tokio::spawn(async move {
            for _ in 0..80 {
                if let Some(f) = cl.pending_approvals.list().first() {
                    cl.pending_approvals
                        .resolve(&f.approval_id, ApprovalOutcome::Approved);
                    return;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        });
        let fut = run(&st_m, "vox_terminal_exec", pwd_arg.clone(), Some(mode));
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), fut)
                .await
                .unwrap()["success"],
            true
        );
    }
    let si_arg = json!({"session_id": "term-1", "block_id": 9, "data": "pw"});
    let cred = run(&st_m, "vox_terminal_send_input", si_arg, Some("accept_all")).await;
    let err_s = cred["error"].as_str().unwrap_or("");
    assert!(cred["success"] == false && err_s.contains("CREDENTIAL_PROMPT_USER_ONLY"));
    assert!(st_m.pending_approvals.list().is_empty());
}

#[tokio::test]
async fn scrub_utf8_ecma48_background_wait_and_pinned_filter_before_slice() {
    assert_eq!(
        scrub_terminal_mcp_str("10%\r100%\n\x1b🦀\x1b(Bhello\r"),
        "100%\n🦀hello"
    );

    let run = async |s: &ServerState, t: &str, a: Value, m: Option<&str>| -> Value {
        serde_json::from_str(&call_mode(s, t, a, m).await.unwrap()).unwrap()
    };
    let mut blocks = vec![
        fv::<TermBlockDto>(
            json!({"id": 1, "input": "echo pin", "output": "pinned-1", "status": "ok", "exit_code": 0, "pinned": true}),
        )
        .unwrap(),
    ];
    for id in 2..=16u64 {
        blocks.push(
            fv(json!({"id": id, "input": "pwd", "output": "x", "status": "ok", "exit_code": 0, "pinned": false}))
                .unwrap(),
        );
    }
    let st = ServerState::new_test().await;
    st.set_terminal_bridge(Arc::new(MockBridge(Mutex::new((
        "autopilot".into(),
        "l5_autopilot_floor".into(),
        "ws".into(),
        16,
        blocks,
    )))));

    let bg = run(
        &st,
        "vox_terminal_exec",
        json!({"command": "echo srv", "session_id": "term-1", "background": true}),
        Some("accept_all"),
    )
    .await;
    assert_eq!(bg["data"]["status"], "running");
    assert!(bg["data"]["output"].as_str().unwrap().contains(":w=0"));

    let pinned_rd = run(
        &st,
        "vox_terminal_read",
        json!({"session_id": "term-1", "pinned_only": true, "tail_blocks": 5}),
        None,
    )
    .await;
    let pbs = pinned_rd["data"]["blocks"].as_array().unwrap();
    assert_eq!(pbs.len(), 1);
    assert_eq!(pbs[0]["id"], 1);
}

#[tokio::test]
async fn edit_and_run_allow_pattern_reject_steer_and_autopilot_preemption_e2e() {
    let run = async |s: &ServerState, t: &str, a: Value, m: Option<&str>| -> Value {
        serde_json::from_str(&call_mode(s, t, a, m).await.unwrap()).unwrap()
    };
    let st = ServerState::new_test().await;
    let bridge = Arc::new(MockBridge(Mutex::new((
        "manual".into(),
        "l3_structured_default".into(),
        "ws".into(),
        1,
        vec![],
    ))));
    st.set_terminal_bridge(bridge.clone());

    let cl = st.clone();
    tokio::spawn(async move {
        for _ in 0..80 {
            if let Some(info) = cl.pending_approvals.list().first() {
                assert_eq!(info.session_id.as_deref(), Some("term-1"));
                assert_eq!(info.command.as_deref(), Some("cargo test --broken"));
                assert_eq!(info.control_mode.as_deref(), Some("manual"));
                let _ = call_mode(
                    &cl,
                    "vox_resolve_approval",
                    json!({
                        "approval_id": info.approval_id,
                        "outcome": "modified",
                        "modified_command": "  cargo check -p vox-cli  ",
                        "allow_pattern": "cargo test"
                    }),
                    None,
                )
                .await;
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    });
    let mod_res = run(
        &st,
        "vox_terminal_exec",
        json!({"command": "cargo test --broken", "session_id": "term-1"}),
        Some("accept_all"),
    )
    .await;
    assert_eq!(mod_res["success"], true);
    assert!(
        mod_res["data"]["output"]
            .as_str()
            .unwrap()
            .starts_with("ok:cargo check -p vox-cli:w=")
    );

    // Matching leaf auto-approves in session without parking:
    let auto_res = tokio::time::timeout(
        Duration::from_secs(2),
        run(
            &st,
            "vox_terminal_exec",
            json!({"command": "cargo test -p vox-cli", "session_id": "term-1"}),
            Some("accept_all"),
        ),
    )
    .await
    .unwrap();
    assert_eq!(auto_res["success"], true);
    assert!(st.pending_approvals.list().is_empty());

    // Compound smuggling parks instead of auto-approving; Reject + Steer scrubs secrets:
    let cl2 = st.clone();
    let tok = format!("ghp_{}", "A1b2C3d4E5f6G7h8I9j0K1l2M3n4");
    let tok_check = tok.clone();
    tokio::spawn(async move {
        for _ in 0..80 {
            if let Some(info) = cl2.pending_approvals.list().first() {
                let _ = call_mode(
                    &cl2,
                    "vox_resolve_approval",
                    json!({
                        "approval_id": info.approval_id,
                        "outcome": "rejected",
                        "steering_note": format!("Drop pipe; auth={tok}")
                    }),
                    None,
                )
                .await;
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    });
    let smuggled_res = run(
        &st,
        "vox_terminal_exec",
        json!({"command": "cargo test | sh", "session_id": "term-1"}),
        Some("accept_all"),
    )
    .await;
    assert_eq!(smuggled_res["success"], false);
    let err_msg = smuggled_res["error"].as_str().unwrap_or("");
    assert!(err_msg.contains("Drop pipe") && !err_msg.contains(&tok_check));
    assert!(err_msg.contains("[REDACTED_KEY]"));

    // Autopilot -> Manual preemption while floored command is parked clears patterns & aborts:
    {
        let mut g = bridge.0.lock();
        g.0 = "autopilot".into();
        g.1 = "l5_autopilot_floor".into();
    }
    let blocks_before = bridge.0.lock().4.len();
    let cl3 = st.clone();
    let br3 = bridge.clone();
    tokio::spawn(async move {
        for _ in 0..80 {
            if let Some(info) = cl3.pending_approvals.list().first() {
                assert!(info.summary.starts_with("[floor:history_rewrite] "));
                assert_eq!(info.floor_category.as_deref(), Some("history_rewrite"));
                br3.0.lock().0 = "manual".into();
                cl3.pending_approvals
                    .resolve(&info.approval_id, ApprovalOutcome::Approved);
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    });
    let raw_preempt = super::dispatch(
        &st,
        "vox_terminal_exec",
        json!({"command": "git push --force origin main", "session_id": "term-1"}),
        false,
    )
    .await
    .unwrap()
    .unwrap();
    let preempt_val: Value = serde_json::from_str(&raw_preempt).unwrap();
    assert_eq!(preempt_val["success"], false);
    assert!(
        preempt_val["error"]
            .as_str()
            .unwrap_or("")
            .contains("Autopilot preempted by operator Manual takeover")
    );
    assert!(st.pending_approvals.session_patterns("term-1").is_empty());
    assert_eq!(bridge.0.lock().4.len(), blocks_before);
}

#[tokio::test]
async fn invalid_edit_and_run_keeps_approval_parked_and_session_allowlist_untouched() {
    let run = async |s: &ServerState, t: &str, a: Value, m: Option<&str>| -> Value {
        serde_json::from_str(&call_mode(s, t, a, m).await.unwrap()).unwrap()
    };
    let st = ServerState::new_test().await;
    let bridge = Arc::new(MockBridge(Mutex::new((
        "manual".into(),
        "l1_read_only".into(),
        "ws".into(),
        1,
        vec![],
    ))));
    st.set_terminal_bridge(bridge.clone());

    let cl = st.clone();
    let probe = tokio::spawn(async move {
        for _ in 0..80 {
            if let Some(info) = cl.pending_approvals.list().first() {
                return info.approval_id.clone();
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        panic!("approval did not park");
    });

    let st_exec = st.clone();
    let exec_task = tokio::spawn(async move {
        serde_json::from_str::<Value>(
            &call_mode(
                &st_exec,
                "vox_terminal_exec",
                json!({"command": "pwd", "session_id": "term-1"}),
                Some("accept_all"),
            )
            .await
            .unwrap(),
        )
        .unwrap()
    });

    let aid = probe.await.unwrap();
    for bad_cmd in [
        "rm -rf /",
        "echo $(whoami)",
        "git push --force origin main",
        "cargo test",
    ] {
        let rej = run(
            &st,
            "vox_resolve_approval",
            json!({
                "approval_id": aid,
                "outcome": "modified",
                "modified_command": bad_cmd,
                "allow_pattern": "pwd"
            }),
            None,
        )
        .await;
        assert_eq!(
            rej["success"], false,
            "must reject invalid edit {bad_cmd:?}"
        );
        assert!(
            st.pending_approvals.peek(&aid).is_some(),
            "approval {aid} must remain parked after rejected edit {bad_cmd:?}"
        );
        assert!(
            st.pending_approvals.session_patterns("term-1").is_empty(),
            "session_allowlist must stay empty after rejected edit {bad_cmd:?}"
        );
    }

    let ok_res = run(
        &st,
        "vox_resolve_approval",
        json!({
            "approval_id": aid,
            "outcome": "modified",
            "modified_command": "ls",
            "allow_pattern": "ls"
        }),
        None,
    )
    .await;
    assert_eq!(ok_res["success"], true);
    let exec_out = exec_task.await.unwrap();
    assert_eq!(exec_out["success"], true);
    assert!(
        exec_out["data"]["output"]
            .as_str()
            .unwrap()
            .starts_with("ok:ls:w=")
    );
    assert_eq!(st.pending_approvals.session_patterns("term-1"), vec!["ls"]);
}
