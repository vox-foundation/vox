//! `term.*` daemon methods: daemon-hosted terminal sessions that the TUI and the
//! Axis GUI attach to (design: `docs/src/architecture/terminal-session-host-design-2026.md`).
//!
//! Lives here, not in `vox-terminal-core`, because the wire types
//! (`DispatchRequest`/`DispatchResponse`) are in `vox-foundation` and the core crate
//! has no edge to it.
//!
//! Sessions run commands as the daemon user, so this is only registered for
//! loopback/stdio transports, and every request is already past the daemon auth gate.
//! `origin: "agent"` is refused on the wire: agents drive a terminal only through the
//! in-process, approval-gated `term_run` tool (task H8), never by claiming an origin here.

use std::sync::Arc;

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use serde_json::{Value, json};
use vox_foundation::protocol::{DispatchPayload, DispatchRequest, DispatchResponse, term_method};
use vox_orchestrator::orch_daemon::{ExtraDispatch, FrameSink, response_err, response_result};
use vox_orchestrator::oversight::floor::{FloorInput, classify_floor};
use vox_terminal_core::block::{BlockId, BlockOrigin};
use vox_terminal_core::{LocalHost, OpenSpec, SessionEvent};

pub struct TermDispatch {
    host: Arc<LocalHost>,
}

impl TermDispatch {
    pub fn new(host: Arc<LocalHost>) -> Self {
        Self { host }
    }

    /// Execute an agent-initiated terminal line in `session_id` gated by the Phase 12
    /// safety floor (`classify_floor`). Refuses unapproved executions before opening
    /// any block; when approved, tags the resulting block with [`BlockOrigin::Agent`].
    // Not yet called from the daemon's dispatch path; exercised by the tests below.
    #[cfg_attr(not(test), allow(dead_code))]
    pub async fn run_agent_command(
        &self,
        session_id: &str,
        line: &str,
        agent_id: &str,
        approved: bool,
    ) -> anyhow::Result<BlockId> {
        let trimmed = line.trim();
        let shell_cmd = trimmed
            .strip_prefix('!')
            .or_else(|| trimmed.strip_prefix("/sh "))
            .unwrap_or(trimmed)
            .trim();
        let args = json!({ "command": shell_cmd });
        let floor = classify_floor(&FloorInput::tool(
            "vox_run_shell",
            &args,
            std::path::Path::new("."),
        ));
        if !approved {
            if let Some(cat) = floor {
                anyhow::bail!(
                    "safety floor ({}) requires human approval before term_run",
                    cat.id()
                );
            }
            anyhow::bail!("term_run requires approval before opening a terminal block");
        }
        let host = Arc::clone(&self.host);
        let sid = session_id.to_string();
        let l = line.to_string();
        let origin = BlockOrigin::Agent {
            agent_id: agent_id.to_string(),
        };
        tokio::task::spawn_blocking(move || host.submit_with_origin(&sid, &l, origin)).await?
    }
}

fn str_param<'a>(p: &'a Value, key: &str) -> anyhow::Result<&'a str> {
    p.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("missing string param `{key}`"))
}

fn dim(p: &Value, key: &str, default: u16) -> u16 {
    p.get(key)
        .and_then(Value::as_u64)
        .and_then(|n| u16::try_from(n).ok())
        .unwrap_or(default)
}

/// Fail-closed: over the wire only a human origin exists. Anything else (including
/// `agent`, any casing) is refused, so a client can never claim to be an approved agent.
fn refuse_agent_origin(p: &Value) -> anyhow::Result<()> {
    match p.get("origin").and_then(Value::as_str) {
        None | Some("user") => Ok(()),
        Some(o) => anyhow::bail!(
            "origin `{o}` is refused over the wire (only `user`); agents drive terminals via the approval-gated term_run tool"
        ),
    }
}

fn req_dim(p: &Value, key: &str) -> anyhow::Result<u16> {
    p.get(key)
        .and_then(Value::as_u64)
        .and_then(|n| u16::try_from(n).ok())
        .ok_or_else(|| anyhow::anyhow!("missing or out-of-range numeric param `{key}`"))
}

fn event_frame(id: &str, value: Value) -> DispatchResponse {
    DispatchResponse {
        id: id.to_string(),
        payload: DispatchPayload::Event { value },
    }
}

fn session_event_json(host: &LocalHost, session: &str, ev: SessionEvent) -> Value {
    match ev {
        SessionEvent::BlockOpened { id } => json!({"type": "block_opened", "id": id.0}),
        SessionEvent::OutputAppended { id, chunk } => {
            json!({"type": "block_output", "id": id.0, "chunk": chunk})
        }
        SessionEvent::BlockClosed { id } => {
            json!({"type": "block_closed", "id": id.0, "block": host.block(session, BlockId(id.0))})
        }
        SessionEvent::AgentMessage { text } => json!({"type": "agent", "text": text}),
    }
}

#[async_trait::async_trait]
impl ExtraDispatch for TermDispatch {
    async fn try_handle(&self, req: &DispatchRequest) -> Option<DispatchResponse> {
        if !req.method.starts_with("term.") || req.method == term_method::ATTACH {
            return None;
        }
        let p = &req.params;
        let result: anyhow::Result<Value> = async {
            match req.method.as_str() {
                term_method::OPEN => {
                    let spec = OpenSpec {
                        cols: dim(p, "cols", 80),
                        rows: dim(p, "rows", 24),
                        shell: p.get("shell").and_then(Value::as_str).map(String::from),
                    };
                    Ok(json!({"session_id": self.host.open(spec).await?}))
                }
                term_method::LIST => Ok(Value::Array(
                    self.host
                        .list()
                        .into_iter()
                        .map(|s| {
                            json!({
                                "session_id": s.id,
                                "blocks": s.blocks,
                                "security_level": s.security_level,
                                "control_mode": s.control_mode,
                            })
                        })
                        .collect(),
                )),
                term_method::INPUT => {
                    refuse_agent_origin(p)?;
                    let bytes = B64.decode(str_param(p, "bytes_b64")?)?;
                    self.host.input(str_param(p, "session_id")?, &bytes)?;
                    Ok(json!({}))
                }
                term_method::SUBMIT => {
                    refuse_agent_origin(p)?;
                    let (sid, line) = (
                        str_param(p, "session_id")?.to_string(),
                        str_param(p, "line")?.to_string(),
                    );
                    // `!cmd` runs synchronously inside Session::submit; keep it off the
                    // async workers so one slow command cannot stall the daemon.
                    let host = Arc::clone(&self.host);
                    let id =
                        tokio::task::spawn_blocking(move || host.submit(&sid, &line)).await??;
                    Ok(json!({"block_id": id.0}))
                }
                term_method::RESIZE => {
                    self.host.resize(
                        str_param(p, "session_id")?,
                        req_dim(p, "cols")?,
                        req_dim(p, "rows")?,
                    )?;
                    Ok(json!({}))
                }
                term_method::CLOSE => {
                    self.host.close(str_param(p, "session_id")?)?;
                    Ok(json!({}))
                }
                "term.context" => {
                    let sid = str_param(p, "session_id")?;
                    let max_blocks =
                        p.get("max_blocks").and_then(Value::as_u64).unwrap_or(10) as usize;
                    let blocks = self.host.redacted_context_bundle(sid, max_blocks)?;
                    Ok(json!({ "blocks": blocks }))
                }
                "term.security" => {
                    let sid = str_param(p, "session_id")?;
                    if let Some(level_val) = p.get("security_level") {
                        let level: vox_terminal_core::TermSecurityLevel =
                            serde_json::from_value(level_val.clone())?;
                        self.host.set_security_level(sid, level)?;
                    }
                    Ok(json!({ "security_level": self.host.security_level(sid)? }))
                }
                "term.control" => {
                    let sid = str_param(p, "session_id")?;
                    if let Some(mode_val) = p.get("control_mode") {
                        let mode: vox_terminal_core::TermControlMode =
                            serde_json::from_value(mode_val.clone())?;
                        self.host.set_control_mode(sid, mode)?;
                    }
                    Ok(json!({ "control_mode": self.host.control_mode(sid)? }))
                }
                other => anyhow::bail!("unknown terminal method `{other}`"),
            }
        }
        .await;
        Some(match result {
            Ok(v) => response_result(&req.id, v),
            Err(e) => response_err(&req.id, e.to_string()),
        })
    }

    async fn try_stream(
        &self,
        req: &DispatchRequest,
        out: &mut dyn FrameSink,
    ) -> Option<anyhow::Result<()>> {
        if req.method != term_method::ATTACH {
            return None;
        }
        let session = match str_param(&req.params, "session_id") {
            Ok(s) => s.to_string(),
            Err(e) => return Some(out.send(&response_err(&req.id, e.to_string())).await),
        };
        let mut a = match self.host.attach(&session) {
            Ok(a) => a,
            Err(e) => return Some(out.send(&response_err(&req.id, e.to_string())).await),
        };
        let snapshot = json!({
            "type": "snapshot",
            "blocks": a.blocks,
            "replay_b64": B64.encode(&a.replay),
            "seq": a.seq,
        });
        Some(
            async {
                out.send(&event_frame(&req.id, snapshot)).await?;
                loop {
                    let value = tokio::select! {
                        o = a.output.recv() => match o {
                            Ok(f) => json!({"type": "output", "seq": f.seq, "bytes_b64": B64.encode(&f.bytes)}),
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                                out.send(&event_frame(&req.id, json!({"type": "lagged"}))).await?;
                                return Ok(());
                            }
                            Err(_) => {
                                out.send(&event_frame(&req.id, json!({"type": "exit"}))).await?;
                                return Ok(());
                            }
                        },
                        e = a.events.recv() => match e {
                            Ok(ev) => session_event_json(&self.host, &session, ev),
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                                out.send(&event_frame(&req.id, json!({"type": "lagged"}))).await?;
                                return Ok(());
                            }
                            Err(_) => {
                                out.send(&event_frame(&req.id, json!({"type": "exit"}))).await?;
                                return Ok(());
                            }
                        },
                    };
                    out.send(&event_frame(&req.id, value)).await?;
                }
            }
            .await,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use std::time::Duration;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::{TcpListener, TcpStream};
    use vox_orchestrator::orch_daemon::serve_listener_with_extra;
    use vox_orchestrator::{Orchestrator, OrchestratorConfig};

    async fn start(token: Option<&str>) -> (SocketAddr, Arc<LocalHost>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let host = Arc::new(LocalHost::default());
        let extra: Arc<dyn ExtraDispatch> = Arc::new(TermDispatch::new(host.clone()));
        let orch = Arc::new(Orchestrator::new(OrchestratorConfig::for_testing()));
        let token: Option<Arc<str>> = token.map(Arc::from);
        tokio::spawn(async move {
            let _ = serve_listener_with_extra(
                listener,
                addr.to_string(),
                "repo".into(),
                orch,
                Some(extra),
                token,
            )
            .await;
        });
        (addr, host)
    }

    fn request(method: &str, params: Value, token: Option<&str>) -> String {
        let r = DispatchRequest {
            id: "1".into(),
            method: method.into(),
            params,
            auth_token: token.map(Into::into),
            permission_mode: None,
        };
        let mut l = serde_json::to_string(&r).unwrap();
        l.push('\n');
        l
    }

    async fn next(rd: &mut BufReader<tokio::net::tcp::OwnedReadHalf>) -> Option<DispatchPayload> {
        let mut buf = String::new();
        // drift-allow(timeout-literal,duplicate-numeric-literal): local test frame read bound
        match tokio::time::timeout(Duration::from_secs(5), rd.read_line(&mut buf)).await {
            Ok(Ok(n)) if n > 0 => Some(
                serde_json::from_str::<DispatchResponse>(buf.trim())
                    .unwrap()
                    .payload,
            ),
            _ => None,
        }
    }

    async fn call(
        addr: SocketAddr,
        method: &str,
        params: Value,
        token: Option<&str>,
    ) -> DispatchPayload {
        let (rd, mut wr) = TcpStream::connect(addr).await.unwrap().into_split();
        wr.write_all(request(method, params, token).as_bytes())
            .await
            .unwrap();
        next(&mut BufReader::new(rd)).await.expect("response frame")
    }

    fn result(p: DispatchPayload) -> Value {
        match p {
            DispatchPayload::Result { value } => value,
            other => panic!("expected Result, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn open_submit_list_then_attach_snapshots_and_streams() {
        let (addr, host) = start(None).await;
        let sid = result(call(addr, term_method::OPEN, json!({}), None).await)["session_id"]
            .as_str()
            .unwrap()
            .to_string();
        call(
            addr,
            term_method::SUBMIT,
            json!({"session_id": sid, "line": "!echo hi"}),
            None,
        )
        .await;
        let list = result(call(addr, term_method::LIST, json!({}), None).await);
        assert_eq!(list[0]["session_id"], sid.as_str());

        let (rd, mut wr) = TcpStream::connect(addr).await.unwrap().into_split();
        let mut rd = BufReader::new(rd);
        wr.write_all(request(term_method::ATTACH, json!({"session_id": sid}), None).as_bytes())
            .await
            .unwrap();
        let DispatchPayload::Event { value: snap } = next(&mut rd).await.unwrap() else {
            panic!("first frame must be the snapshot")
        };
        assert_eq!(snap["type"], "snapshot");
        assert!(snap["blocks"].to_string().contains("hi"), "{snap}");

        call(
            addr,
            term_method::SUBMIT,
            json!({"session_id": sid, "line": "!echo two"}),
            None,
        )
        .await;
        // PTY `output` frames may interleave; wait for the block to close.
        loop {
            let DispatchPayload::Event { value } = next(&mut rd).await.expect("stream ended early")
            else {
                panic!("non-event frame on stream")
            };
            if value["type"] == "block_closed" {
                assert_eq!(value["block"]["input"], "echo two");
                break;
            }
        }
        host.close(&sid).unwrap();
    }

    /// One scenario, run against any host: the two implementations must be indistinguishable.
    async fn host_scenario(host: &dyn vox_terminal_core::SessionHost) {
        let id = host.open(OpenSpec::default()).await.unwrap();
        host.submit(&id, "!echo hi").await.unwrap();
        assert!(host.list().await.unwrap().iter().any(|s| s.id == id));

        let mut a = host.attach(&id).await.unwrap();
        assert_eq!(a.blocks.len(), 1);
        assert!(a.blocks[0].plain_output().contains("hi"));

        host.submit(&id, "!echo two").await.unwrap();
        // drift-allow(timeout-literal,duplicate-numeric-literal): local test event wait bound
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                if matches!(
                    a.events.recv().await.unwrap(),
                    SessionEvent::BlockOpened { .. }
                ) {
                    break;
                }
            }
        })
        .await
        .expect("live BlockOpened event");

        assert!(host.attach("nope").await.is_err());
        host.close(&id).await.unwrap();
        assert!(host.submit(&id, "!echo x").await.is_err());
    }

    #[tokio::test]
    async fn local_and_daemon_hosts_pass_the_same_scenario() {
        host_scenario(&LocalHost::default()).await;

        let (addr, _daemon_host) = start(Some("t")).await;
        let client =
            vox_orchestrator::orch_daemon::OrchDaemonClient::with_token(addr.to_string(), "t");
        host_scenario(&vox_terminal_core::DaemonHost::new(client)).await;
    }

    #[test]
    fn daemon_host_method_names_match_the_protocol() {
        use vox_terminal_core::daemon_host as d;
        assert_eq!(
            [
                d::OPEN,
                d::LIST,
                d::ATTACH,
                d::INPUT,
                d::SUBMIT,
                d::RESIZE,
                d::CLOSE
            ],
            [
                term_method::OPEN,
                term_method::LIST,
                term_method::ATTACH,
                term_method::INPUT,
                term_method::SUBMIT,
                term_method::RESIZE,
                term_method::CLOSE
            ]
        );
    }

    #[tokio::test]
    async fn requests_without_the_daemon_token_are_rejected_and_create_nothing() {
        let (addr, host) = start(Some("secret")).await;
        let p = call(addr, term_method::OPEN, json!({}), None).await;
        assert!(
            matches!(p, DispatchPayload::Error { ref message, .. } if message.contains("unauthorized"))
        );
        assert!(host.list().is_empty());
        let ok = call(addr, term_method::OPEN, json!({}), Some("secret")).await;
        assert!(matches!(ok, DispatchPayload::Result { .. }));
    }

    #[tokio::test]
    async fn origin_is_fail_closed_and_resize_needs_dimensions() {
        let (addr, host) = start(None).await;
        let sid = result(call(addr, term_method::OPEN, json!({}), None).await)["session_id"]
            .as_str()
            .unwrap()
            .to_string();
        // Any non-"user" label is refused, whatever its casing.
        for origin in ["Agent", "AGENT", "system", ""] {
            let p = call(
                addr,
                term_method::SUBMIT,
                json!({"session_id": sid, "line": "!echo no", "origin": origin}),
                None,
            )
            .await;
            assert!(
                matches!(p, DispatchPayload::Error { .. }),
                "origin {origin:?}: {p:?}"
            );
        }
        // Explicit "user" and an absent origin are both fine.
        let ok = call(
            addr,
            term_method::SUBMIT,
            json!({"session_id": sid, "line": "!echo ok", "origin": "user"}),
            None,
        )
        .await;
        assert!(matches!(ok, DispatchPayload::Result { .. }), "{ok:?}");
        // A resize without dimensions must not silently pick defaults.
        let p = call(addr, term_method::RESIZE, json!({"session_id": sid}), None).await;
        assert!(
            matches!(p, DispatchPayload::Error { ref message, .. } if message.contains("cols")),
            "{p:?}"
        );
        host.close(&sid).unwrap();
    }

    #[tokio::test]
    async fn agent_origin_is_refused_on_input_and_submit() {
        let (addr, host) = start(None).await;
        let sid = result(call(addr, term_method::OPEN, json!({}), None).await)["session_id"]
            .as_str()
            .unwrap()
            .to_string();
        for (m, extra) in [
            (term_method::INPUT, json!({"bytes_b64": B64.encode("x")})),
            (term_method::SUBMIT, json!({"line": "!echo no"})),
        ] {
            let mut params = extra;
            params["session_id"] = json!(sid);
            params["origin"] = json!("agent");
            let p = call(addr, m, params, None).await;
            assert!(
                matches!(p, DispatchPayload::Error { ref message, .. } if message.contains("`agent` is refused")),
                "{m}: {p:?}"
            );
        }
        assert!(
            host.attach(&sid).unwrap().blocks.is_empty(),
            "refused submit must not run"
        );
        host.close(&sid).unwrap();
    }

    #[tokio::test]
    async fn agent_command_opens_block_with_agent_origin_and_enforces_floor() {
        let host = Arc::new(LocalHost::default());
        let dispatch = TermDispatch::new(host.clone());
        let sid = host.open(OpenSpec::default()).await.unwrap();

        // Unapproved call must fail and open 0 blocks.
        let err = dispatch
            .run_agent_command(&sid, "!git push --force origin main", "agent-alpha", false)
            .await
            .unwrap_err();
        assert!(
            err.to_string().contains("history_rewrite"),
            "expected history_rewrite floor error, got: {err}"
        );
        assert!(host.attach(&sid).unwrap().blocks.is_empty());

        // Approved call executes and tags BlockOrigin::Agent.
        let bid = dispatch
            .run_agent_command(&sid, "!echo from-agent", "agent-alpha", true)
            .await
            .unwrap();
        let block = host.block(&sid, bid).expect("agent block exists");
        assert_eq!(
            block.origin,
            vox_terminal_core::BlockOrigin::Agent {
                agent_id: "agent-alpha".to_string()
            }
        );
        assert!(block.plain_output().contains("from-agent"));
        host.close(&sid).unwrap();
    }

    #[tokio::test]
    async fn term_context_rpc_returns_redacted_blocks() {
        let (addr, host) = start(None).await;
        let sid = result(call(addr, term_method::OPEN, json!({}), None).await)["session_id"]
            .as_str()
            .unwrap()
            .to_string();
        call(
            addr,
            term_method::SUBMIT,
            json!({"session_id": sid, "line": "!echo contact alice@example.com"}),
            None,
        )
        .await;
        let ctx = result(
            call(
                addr,
                "term.context",
                json!({"session_id": sid, "max_blocks": 5}),
                None,
            )
            .await,
        );
        let blocks = ctx["blocks"].as_array().expect("blocks array");
        assert_eq!(blocks.len(), 1);
        assert!(
            !blocks[0]["output"]
                .as_str()
                .unwrap()
                .contains("alice@example.com")
        );
        assert!(
            blocks[0]["output"]
                .as_str()
                .unwrap()
                .contains("[REDACTED_EMAIL]")
        );
        host.close(&sid).unwrap();
    }
}
