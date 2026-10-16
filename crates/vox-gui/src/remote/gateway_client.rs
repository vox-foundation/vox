//! Remote Axis client for a `vox serve` gateway: `GET /v1/profile` and `/api/v2/*` over reqwest,
//! MCP tool calls and agent events over the `/v1/ws` WebSocket RPC. TLS uses the TOFU
//! `PinVerifier`; plain http is accepted only for loopback.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::sync::{Mutex, broadcast, mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::http::header::AUTHORIZATION;
use tokio_tungstenite::{Connector, MaybeTlsStream, WebSocketStream};
use vox_foundation::protocol::orch_daemon_method;

use super::error::AxisError;
use super::tls_pin::{PinOutcome, PinVerifier, pinned_client_config};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerProfile {
    pub profiles: Vec<String>,
    pub max_exposure: String,
    pub subsystems: ServerSubsystems,
    pub tools: Vec<String>,
    pub axis_surfaces: Vec<String>,
    pub server_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ServerSubsystems {
    pub booted: Vec<String>,
    pub failed: Vec<FailedSubsystem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FailedSubsystem {
    pub id: String,
    pub error: String,
}

pub fn parse_server_url(raw: &str) -> Result<Url, AxisError> {
    let url = Url::parse(raw.trim()).map_err(|e| AxisError::InvalidUrl(format!("{raw}: {e}")))?;
    let host = url
        .host_str()
        .ok_or_else(|| AxisError::InvalidUrl(format!("{raw}: no host")))?;
    match url.scheme() {
        "https" => {}
        "http" if is_loopback_host(host) => {}
        "http" => {
            return Err(AxisError::InsecureUrl(format!(
                "{raw}: use https:// (plain http is only allowed for loopback)"
            )));
        }
        other => {
            return Err(AxisError::InvalidUrl(format!(
                "{raw}: unsupported scheme {other}"
            )));
        }
    }
    let mut origin = url;
    origin.set_path("/");
    origin.set_query(None);
    origin.set_fragment(None);
    Ok(origin)
}

fn is_loopback_host(host: &str) -> bool {
    host == "localhost"
        || host
            .trim_matches(|c| c == '[' || c == ']')
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback())
}

pub fn ws_url(base: &Url) -> Result<Url, AxisError> {
    let mut url = base
        .join("/v1/ws")
        .map_err(|e| AxisError::InvalidUrl(e.to_string()))?;
    let scheme = if base.scheme() == "https" {
        "wss"
    } else {
        "ws"
    };
    url.set_scheme(scheme).map_err(|()| {
        AxisError::InvalidUrl(format!("cannot derive a websocket url from {base}"))
    })?;
    Ok(url)
}

pub fn major_minor(version: &str) -> Option<(u64, u64)> {
    let core = version.split(['+', '-', ' ']).next()?;
    let mut parts = core.split('.');
    Some((parts.next()?.parse().ok()?, parts.next()?.parse().ok()?))
}

pub fn check_version(server: &str, gui: &str) -> Result<(), AxisError> {
    match (major_minor(server), major_minor(gui)) {
        (Some(a), Some(b)) if a == b => Ok(()),
        _ => Err(AxisError::VersionMismatch {
            server: server.to_string(),
            gui: gui.to_string(),
        }),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteRoute {
    ToolCall,
    ApiGet(&'static str),
    ApiPost(&'static str),
}

/// Gateway equivalent of an orchestrator daemon method; `None` = not exposed remotely yet.
pub fn remote_route(method: &str) -> Option<RemoteRoute> {
    match method {
        orch_daemon_method::TOOL_CALL => Some(RemoteRoute::ToolCall),
        orch_daemon_method::VCS_ISOLATION_STATUS => {
            Some(RemoteRoute::ApiGet("/api/v2/vcs/isolation"))
        }
        orch_daemon_method::VCS_ISOLATION_SET_STRATEGY => {
            Some(RemoteRoute::ApiPost("/api/v2/vcs/isolation/strategy"))
        }
        orch_daemon_method::SUBMIT_TASK => {
            Some(RemoteRoute::ApiPost("/api/v2/orchestrator/tasks/submit"))
        }
        orch_daemon_method::LIST_TASKS => Some(RemoteRoute::ApiGet("/api/v2/orchestrator/tasks")),
        orch_daemon_method::EDIT_TASK => {
            Some(RemoteRoute::ApiPost("/api/v2/orchestrator/tasks/edit"))
        }
        orch_daemon_method::CANCEL_TASK => {
            Some(RemoteRoute::ApiPost("/api/v2/orchestrator/tasks/cancel"))
        }
        orch_daemon_method::INTERRUPT_TASK => {
            Some(RemoteRoute::ApiPost("/api/v2/orchestrator/tasks/interrupt"))
        }
        orch_daemon_method::REORDER_TASK => {
            Some(RemoteRoute::ApiPost("/api/v2/orchestrator/tasks/reorder"))
        }
        orch_daemon_method::DOUBT_TASK => {
            Some(RemoteRoute::ApiPost("/api/v2/orchestrator/tasks/doubt"))
        }
        orch_daemon_method::OVERRULE_TASK => {
            Some(RemoteRoute::ApiPost("/api/v2/orchestrator/tasks/overrule"))
        }
        orch_daemon_method::APPROVE_PLAN => Some(RemoteRoute::ApiPost(
            "/api/v2/orchestrator/tasks/approve-plan",
        )),
        orch_daemon_method::SKIP_VERIFY => Some(RemoteRoute::ApiPost(
            "/api/v2/orchestrator/tasks/skip-verify",
        )),
        orch_daemon_method::FORCE_VERIFY => Some(RemoteRoute::ApiPost(
            "/api/v2/orchestrator/tasks/force-verify",
        )),
        orch_daemon_method::PAUSE_AGENT => {
            Some(RemoteRoute::ApiPost("/api/v2/orchestrator/agents/pause"))
        }
        orch_daemon_method::RESUME_AGENT => {
            Some(RemoteRoute::ApiPost("/api/v2/orchestrator/agents/resume"))
        }
        orch_daemon_method::SUBAGENT_TREE => {
            Some(RemoteRoute::ApiGet("/api/v2/orchestrator/subagent-tree"))
        }
        orch_daemon_method::LIST_PENDING_APPROVALS => {
            Some(RemoteRoute::ApiGet("/api/v2/orchestrator/approvals"))
        }
        orch_daemon_method::SET_MESH_POLICY => {
            Some(RemoteRoute::ApiPost("/api/v2/orchestrator/mesh-policy"))
        }
        vox_foundation::protocol::dei_method::RESEARCH_RUN => {
            Some(RemoteRoute::ApiPost("/api/v2/research/run"))
        }
        vox_foundation::protocol::dei_method::RESEARCH_PERSIST_CLAIMS => {
            Some(RemoteRoute::ApiPost("/api/v2/research/persist-claims"))
        }
        _ => None,
    }
}

pub fn unwrap_v2_envelope(body: Value) -> Result<Value, AxisError> {
    if let Some(err) = body.get("error") {
        return Err(AxisError::Remote {
            code: err
                .get("code")
                .and_then(Value::as_str)
                .unwrap_or("error")
                .to_string(),
            message: err
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
        });
    }
    Ok(body.get("data").cloned().unwrap_or(Value::Null))
}

#[derive(Debug, Deserialize)]
struct WsReply {
    #[serde(default)]
    id: Option<String>,
    #[serde(rename = "type")]
    msg_type: String,
    #[serde(default)]
    data: Value,
    #[serde(default)]
    error: Option<String>,
}

type Pending = Arc<Mutex<HashMap<String, oneshot::Sender<WsReply>>>>;

pub struct WsRpc {
    out: mpsc::UnboundedSender<Message>,
    pending: Pending,
    events: broadcast::Sender<Value>,
    next_id: AtomicU64,
    tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl WsRpc {
    pub fn spawn<S>(ws: WebSocketStream<S>) -> Self
    where
        S: AsyncRead + AsyncWrite + Unpin + Send + 'static,
    {
        let (mut sink, mut stream) = ws.split();
        let (out, mut out_rx) = mpsc::unbounded_channel::<Message>();
        let pending: Pending = Arc::default();
        let (events, _) = broadcast::channel(256);
        let writer = tokio::spawn(async move {
            while let Some(msg) = out_rx.recv().await {
                if sink.send(msg).await.is_err() {
                    break;
                }
            }
        });
        let reader_pending = pending.clone();
        let reader_events = events.clone();
        let reader = tokio::spawn(async move {
            while let Some(Ok(msg)) = stream.next().await {
                let Message::Text(text) = msg else { continue };
                let Ok(reply) = serde_json::from_str::<WsReply>(&text) else {
                    continue;
                };
                if let Some(id) = reply.id.clone()
                    && let Some(tx) = reader_pending.lock().await.remove(&id)
                {
                    let _ = tx.send(reply);
                    continue;
                }
                if reply.msg_type == "agent_event" {
                    let _ = reader_events.send(reply.data);
                }
            }
            // Dropping the senders fails every waiter with RecvError -> Unreachable.
            reader_pending.lock().await.clear();
        });
        Self {
            out,
            pending,
            events,
            next_id: AtomicU64::new(1),
            tasks: vec![writer, reader],
        }
    }

    pub async fn call_tool(
        &self,
        name: &str,
        args: Value,
        deadline: Duration,
    ) -> Result<Value, AxisError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed).to_string();
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id.clone(), tx);
        let frame =
            serde_json::json!({ "id": id, "type": "call_tool", "name": name, "args": args });
        if self.out.send(Message::Text(frame.to_string())).is_err() {
            self.pending.lock().await.remove(&id);
            return Err(AxisError::Unreachable("websocket closed".into()));
        }
        match tokio::time::timeout(deadline, rx).await {
            Ok(Ok(reply)) if reply.msg_type == "call_tool_result" => Ok(reply.data),
            Ok(Ok(reply)) => Err(AxisError::Remote {
                code: "ws_error".into(),
                message: reply.error.unwrap_or(reply.msg_type),
            }),
            Ok(Err(_)) => Err(AxisError::Unreachable("websocket closed".into())),
            Err(_) => {
                self.pending.lock().await.remove(&id);
                Err(AxisError::Unreachable(format!("tool {name} timed out")))
            }
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Value> {
        self.events.subscribe()
    }
}

impl Drop for WsRpc {
    fn drop(&mut self) {
        for task in &self.tasks {
            task.abort();
        }
    }
}

pub struct ConnectRequest {
    pub url: Url,
    pub token: String,
    pub expected_pin: Option<String>,
}

pub struct GatewayClient {
    base: Url,
    token: String,
    http: reqwest::Client,
    profile: ServerProfile,
    ws: WsRpc,
}

impl GatewayClient {
    /// Connects, checks the version, and opens `/v1/ws`. Returns the pin actually used (https only).
    pub async fn connect(req: ConnectRequest) -> Result<(Self, Option<String>), AxisError> {
        let verifier = Arc::new(PinVerifier::new(req.expected_pin.clone()));
        let mut builder = vox_http_client::client_builder();
        let mut connector = None;
        if req.url.scheme() == "https" {
            let cfg = pinned_client_config(verifier.clone())
                .map_err(|e| AxisError::Unreachable(e.to_string()))?;
            connector = Some(Connector::Rustls(Arc::new(cfg.clone())));
            builder = builder.use_preconfigured_tls(cfg);
        }
        let http = builder
            .build()
            .map_err(|e| AxisError::Unreachable(e.to_string()))?;
        let profile = match fetch_profile(&http, &req.url, &req.token).await {
            Ok(p) => p,
            Err(e) => return Err(pin_error(&verifier).unwrap_or(e)),
        };
        check_version(&profile.server_version, env!("CARGO_PKG_VERSION"))?;
        let ws = connect_ws(&req.url, &req.token, connector)
            .await
            .map_err(|e| pin_error(&verifier).unwrap_or(e))?;
        let pin = verifier.seen();
        Ok((
            Self::from_parts(req.url, req.token, http, profile, WsRpc::spawn(ws)),
            pin,
        ))
    }

    pub(crate) fn from_parts(
        base: Url,
        token: String,
        http: reqwest::Client,
        profile: ServerProfile,
        ws: WsRpc,
    ) -> Self {
        Self {
            base,
            token,
            http,
            profile,
            ws,
        }
    }

    pub fn profile(&self) -> &ServerProfile {
        &self.profile
    }

    pub fn base_url(&self) -> &Url {
        &self.base
    }

    pub async fn call_daemon_method(
        &self,
        method: &str,
        params: Value,
        deadline: Duration,
    ) -> Result<Value, AxisError> {
        match remote_route(method) {
            Some(RemoteRoute::ToolCall) => {
                let name = params.get("name").and_then(Value::as_str).ok_or_else(|| {
                    AxisError::Remote {
                        code: "bad_request".into(),
                        message: "params.name (string) required".into(),
                    }
                })?;
                let args = params
                    .get("args")
                    .cloned()
                    .unwrap_or_else(|| serde_json::json!({}));
                self.ws.call_tool(name, args, deadline).await
            }
            Some(RemoteRoute::ApiGet(path)) => self.api_get(path).await,
            Some(RemoteRoute::ApiPost(path)) => self.api_post(path, params).await,
            None => Err(AxisError::NotRemoteCapable {
                method: method.to_string(),
            }),
        }
    }

    pub async fn api_get(&self, path: &str) -> Result<Value, AxisError> {
        let url = self
            .base
            .join(path)
            .map_err(|e| AxisError::InvalidUrl(e.to_string()))?;
        let resp = self.http.get(url).bearer_auth(&self.token).send().await;
        read_v2(resp).await
    }

    pub async fn api_post(&self, path: &str, body: Value) -> Result<Value, AxisError> {
        let url = self
            .base
            .join(path)
            .map_err(|e| AxisError::InvalidUrl(e.to_string()))?;
        let resp = self
            .http
            .post(url)
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await;
        read_v2(resp).await
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<Value> {
        self.ws.subscribe()
    }
}

fn pin_error(verifier: &PinVerifier) -> Option<AxisError> {
    match verifier.outcome()? {
        PinOutcome::Match => None,
        PinOutcome::FirstUse { fingerprint } => Some(AxisError::TlsPinRequired { fingerprint }),
        PinOutcome::Changed {
            pinned,
            fingerprint,
        } => Some(AxisError::TlsPinMismatch {
            pinned,
            fingerprint,
        }),
    }
}

async fn read_v2(resp: reqwest::Result<reqwest::Response>) -> Result<Value, AxisError> {
    let resp = resp.map_err(|e| AxisError::Unreachable(e.to_string()))?;
    match resp.status().as_u16() {
        401 | 403 => Err(AxisError::Unauthorized(format!(
            "server rejected the token ({})",
            resp.status()
        ))),
        _ => unwrap_v2_envelope(resp.json::<Value>().await.map_err(|e| AxisError::Remote {
            code: "bad_response".into(),
            message: e.to_string(),
        })?),
    }
}

pub(crate) async fn fetch_profile(
    http: &reqwest::Client,
    base: &Url,
    token: &str,
) -> Result<ServerProfile, AxisError> {
    let url = base
        .join("/v1/profile")
        .map_err(|e| AxisError::InvalidUrl(e.to_string()))?;
    let resp = http
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| AxisError::Unreachable(e.to_string()))?;
    match resp.status().as_u16() {
        200 => resp
            .json::<ServerProfile>()
            .await
            .map_err(|e| AxisError::Remote {
                code: "bad_profile".into(),
                message: e.to_string(),
            }),
        401 | 403 => Err(AxisError::Unauthorized(format!(
            "server rejected the token ({})",
            resp.status()
        ))),
        status => Err(AxisError::Remote {
            code: format!("http_{status}"),
            message: resp.text().await.unwrap_or_default(),
        }),
    }
}

async fn connect_ws(
    base: &Url,
    token: &str,
    connector: Option<Connector>,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, AxisError> {
    let mut request = ws_url(base)?
        .as_str()
        .into_client_request()
        .map_err(|e| AxisError::InvalidUrl(e.to_string()))?;
    let bearer = vox_http_client::bearer_auth_header(token).map_err(|_| {
        AxisError::Unauthorized("token contains characters not allowed in a header".into())
    })?;
    request.headers_mut().insert(AUTHORIZATION, bearer);
    let (ws, _response) =
        tokio_tungstenite::connect_async_tls_with_config(request, None, false, connector)
            .await
            .map_err(|e| AxisError::Unreachable(e.to_string()))?;
    Ok(ws)
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use tokio::io::DuplexStream;
    use tokio_tungstenite::tungstenite::protocol::Role;

    pub(crate) fn profile(axis_surfaces: &[&str]) -> ServerProfile {
        ServerProfile {
            profiles: vec!["mesh".into()],
            max_exposure: "operator".into(),
            subsystems: ServerSubsystems {
                booted: vec!["db".into(), "mesh".into()],
                failed: vec![],
            },
            tools: vec!["vox_mesh_status".into()],
            axis_surfaces: axis_surfaces.iter().map(|s| s.to_string()).collect(),
            server_version: env!("CARGO_PKG_VERSION").into(),
        }
    }

    /// A `GatewayClient` wired to an in-memory websocket; the returned half plays `/v1/ws`.
    pub(crate) async fn duplex_client(
        profile: ServerProfile,
    ) -> (GatewayClient, WebSocketStream<DuplexStream>) {
        let (client_io, server_io) = tokio::io::duplex(64 * 1024);
        let client_ws = WebSocketStream::from_raw_socket(client_io, Role::Client, None).await;
        let server_ws = WebSocketStream::from_raw_socket(server_io, Role::Server, None).await;
        let base = Url::parse("https://vox.lan:3921/").expect("url");
        (
            GatewayClient::from_parts(
                base,
                "t0ken".into(),
                // drift-allow(reqwest-bypass): in-memory duplex websocket test fixture; no HTTP egress
                reqwest::Client::new(),
                profile,
                WsRpc::spawn(client_ws),
            ),
            server_ws,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use serde_json::json;
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn url_policy() {
        assert_eq!(
            parse_server_url("https://vox.lan:3921/x?y=1")
                .unwrap()
                .as_str(),
            "https://vox.lan:3921/"
        );
        assert!(parse_server_url("http://127.0.0.1:3921").is_ok());
        assert!(parse_server_url("http://localhost:3921").is_ok());
        assert!(parse_server_url("http://[::1]:3921").is_ok());
        assert_eq!(
            parse_server_url("http://vox.lan:3921").unwrap_err().code(),
            "insecure_url"
        );
        assert_eq!(
            parse_server_url("ftp://vox.lan").unwrap_err().code(),
            "invalid_url"
        );
        assert_eq!(
            parse_server_url("not a url").unwrap_err().code(),
            "invalid_url"
        );
    }

    #[test]
    fn ws_url_swaps_scheme_and_path() {
        assert_eq!(
            ws_url(&Url::parse("https://vox.lan:3921/").unwrap())
                .unwrap()
                .as_str(),
            "wss://vox.lan:3921/v1/ws"
        );
        assert_eq!(
            ws_url(&Url::parse("http://127.0.0.1:3921/").unwrap())
                .unwrap()
                .as_str(),
            "ws://127.0.0.1:3921/v1/ws"
        );
    }

    #[test]
    fn version_check_compares_major_minor_only() {
        assert_eq!(major_minor("0.6.3+build.12 (abc)"), Some((0, 6)));
        assert_eq!(major_minor("1.2"), Some((1, 2)));
        assert_eq!(major_minor("garbage"), None);
        assert!(check_version("0.6.9", "0.6.0").is_ok());
        assert_eq!(
            check_version("0.7.0", "0.6.0").unwrap_err().code(),
            "version_mismatch"
        );
        assert_eq!(
            check_version("garbage", "0.6.0").unwrap_err().code(),
            "version_mismatch"
        );
    }

    #[test]
    fn route_table() {
        use vox_foundation::protocol::{dei_method, orch_daemon_method as m};
        assert_eq!(remote_route(m::TOOL_CALL), Some(RemoteRoute::ToolCall));
        assert_eq!(
            remote_route(m::VCS_ISOLATION_STATUS),
            Some(RemoteRoute::ApiGet("/api/v2/vcs/isolation"))
        );
        assert_eq!(
            remote_route(m::VCS_ISOLATION_SET_STRATEGY),
            Some(RemoteRoute::ApiPost("/api/v2/vcs/isolation/strategy"))
        );
        assert_eq!(
            remote_route(m::SUBMIT_TASK),
            Some(RemoteRoute::ApiPost("/api/v2/orchestrator/tasks/submit"))
        );
        assert_eq!(
            remote_route(m::SUBAGENT_TREE),
            Some(RemoteRoute::ApiGet("/api/v2/orchestrator/subagent-tree"))
        );
        assert_eq!(
            remote_route(m::LIST_PENDING_APPROVALS),
            Some(RemoteRoute::ApiGet("/api/v2/orchestrator/approvals"))
        );
        assert_eq!(
            remote_route(m::SET_MESH_POLICY),
            Some(RemoteRoute::ApiPost("/api/v2/orchestrator/mesh-policy"))
        );
        assert_eq!(
            remote_route(dei_method::RESEARCH_RUN),
            Some(RemoteRoute::ApiPost("/api/v2/research/run"))
        );
        assert_eq!(
            remote_route(dei_method::RESEARCH_PERSIST_CLAIMS),
            Some(RemoteRoute::ApiPost("/api/v2/research/persist-claims"))
        );
        assert_eq!(remote_route(m::PING), None);
    }

    #[test]
    fn v2_envelope() {
        assert_eq!(
            unwrap_v2_envelope(json!({"v":1,"data":{"a":1}})).unwrap(),
            json!({"a":1})
        );
        let err = unwrap_v2_envelope(json!({"v":1,"error":{"code":"lock","message":"busy"}}))
            .unwrap_err();
        assert_eq!(
            err,
            AxisError::Remote {
                code: "lock".into(),
                message: "busy".into()
            }
        );
    }

    #[tokio::test]
    async fn fetch_profile_sends_bearer_and_maps_status() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/v1/profile"))
            // drift-allow(bearer-header-inline): wiremock header matcher in unit test
            .and(header("authorization", "Bearer good"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "profiles": ["mesh"], "max_exposure": "operator",
                "subsystems": {"booted": ["db", "mesh"], "failed": [{"id": "research", "error": "no key"}]},
                "tools": ["vox_mesh_status"], "axis_surfaces": ["mesh"], "server_version": "0.6.0"
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/v1/profile"))
            .respond_with(ResponseTemplate::new(401))
            .mount(&server)
            .await;
        let base = parse_server_url(&server.uri()).unwrap();
        // drift-allow(reqwest-bypass): wiremock unit test client
        let http = reqwest::Client::new();
        let p = fetch_profile(&http, &base, "good").await.expect("profile");
        assert_eq!(p.axis_surfaces, vec!["mesh".to_string()]);
        assert_eq!(p.subsystems.failed[0].id, "research");
        assert_eq!(
            fetch_profile(&http, &base, "bad").await.unwrap_err().code(),
            "unauthorized"
        );
    }

    #[tokio::test]
    async fn ws_rpc_matches_replies_by_id_and_forwards_agent_events() {
        let (client, mut server) = duplex_client(profile(&["mesh"])).await;
        let mut events = client.subscribe_events();
        let answer = tokio::spawn(async move {
            let Some(Ok(Message::Text(text))) = server.next().await else {
                panic!("no request")
            };
            let req: Value = serde_json::from_str(&text).unwrap();
            assert_eq!(req["type"], "call_tool");
            assert_eq!(req["name"], "vox_mesh_status");
            server
                .send(Message::Text(
                    json!({"type":"agent_event","success":true,"is_error":false,"data":{"id":7}})
                        .to_string(),
                ))
                .await
                .unwrap();
            server.send(Message::Text(json!({"id":req["id"],"type":"call_tool_result","success":true,"is_error":false,"data":{"success":true,"data":{"nodes":[]}}}).to_string())).await.unwrap();
            server
        });
        let out = client
            .call_daemon_method(
                vox_foundation::protocol::orch_daemon_method::TOOL_CALL,
                json!({"name":"vox_mesh_status","args":{}}),
                // drift-allow(timeout-literal,duplicate-numeric-literal): in-memory websocket test timeout
                Duration::from_secs(5),
            )
            .await
            .expect("tool result");
        assert_eq!(out, json!({"success":true,"data":{"nodes":[]}}));
        assert_eq!(events.recv().await.unwrap(), json!({"id":7}));
        drop(answer.await.unwrap());
    }

    #[tokio::test]
    async fn unrouted_method_is_not_remote_capable() {
        let (client, _server) = duplex_client(profile(&["mesh"])).await;
        let err = client
            .call_daemon_method(
                vox_foundation::protocol::orch_daemon_method::PING,
                json!({}),
                // drift-allow(timeout-literal,duplicate-numeric-literal): in-memory websocket test timeout
                Duration::from_secs(1),
            )
            .await
            .unwrap_err();
        assert_eq!(err.code(), "not_remote_capable");
    }

    #[tokio::test]
    async fn closed_socket_fails_pending_calls() {
        let (client, server) = duplex_client(profile(&["mesh"])).await;
        drop(server);
        let err = client
            .call_daemon_method(
                vox_foundation::protocol::orch_daemon_method::TOOL_CALL,
                json!({"name":"vox_mesh_status"}),
                // drift-allow(timeout-literal,duplicate-numeric-literal): in-memory websocket test timeout
                Duration::from_secs(5),
            )
            .await
            .unwrap_err();
        assert_eq!(err.code(), "unreachable");
    }
}
