use crate::drive::protocol::DriveState;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

// vox:defactored-from vox-orchestrator orch_daemon/mod.rs 2026-09-08
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let mut diff = (a.len() != b.len()) as u8;
    for i in 0..a.len().max(b.len()) {
        let ai = *a.get(i).unwrap_or(&0);
        let bi = *b.get(i).unwrap_or(&0);
        diff |= ai ^ bi;
    }
    diff == 0
}

#[derive(Debug, Clone)]
pub struct DriveHttpRequest {
    pub verb: String,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct DriveHttpResponse {
    pub status: u16,
    pub body: String,
}

pub type DriveHandler = Arc<dyn Fn(DriveHttpRequest) -> DriveHttpResponse + Send + Sync>;

#[derive(Clone)]
pub struct ListenerHandle {
    addr: SocketAddr,
    ready: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    handler: Arc<Mutex<Option<DriveHandler>>>,
}

#[allow(dead_code)]
impl ListenerHandle {
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn set_ready(&self, ready: bool) {
        self.ready.store(ready, Ordering::SeqCst);
    }

    pub fn ready(&self) -> bool {
        self.ready.load(Ordering::SeqCst)
    }

    pub fn set_handler(&self, handler: DriveHandler) {
        *self.handler.lock().expect("drive handler lock") = Some(handler);
    }

    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
        let _ = TcpStream::connect_timeout(&self.addr, Duration::from_millis(200));
    }
}

pub struct BoundListener {
    pub handle: ListenerHandle,
    _join: JoinHandle<()>,
}

#[allow(dead_code)]
impl BoundListener {
    pub fn addr(&self) -> SocketAddr {
        self.handle.addr()
    }

    pub fn set_ready(&self, ready: bool) {
        self.handle.set_ready(ready);
    }

    pub fn set_handler(&self, handler: DriveHandler) {
        self.handle.set_handler(handler);
    }

    pub fn shutdown(self) {
        self.handle.shutdown();
    }
}

pub fn bind_loopback(token: &str) -> std::io::Result<BoundListener> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    listener.set_nonblocking(false)?;
    let addr = listener.local_addr()?;
    let ready = Arc::new(AtomicBool::new(false));
    let shutdown = Arc::new(AtomicBool::new(false));
    let handler = Arc::new(Mutex::new(None));
    let handle = ListenerHandle {
        addr,
        ready: Arc::clone(&ready),
        shutdown: Arc::clone(&shutdown),
        handler: Arc::clone(&handler),
    };
    let expected = token.to_string();
    let join_ready = Arc::clone(&ready);
    let join_shutdown = Arc::clone(&shutdown);
    let join_handler = Arc::clone(&handler);
    let join = std::thread::spawn(move || {
        listener
            .set_nonblocking(true)
            .expect("drive listener nonblocking");
        loop {
            if join_shutdown.load(Ordering::SeqCst) {
                break;
            }
            match listener.accept() {
                Ok((stream, peer)) => {
                    if !peer.ip().is_loopback() {
                        continue;
                    }
                    let expected = expected.clone();
                    let ready = join_ready.load(Ordering::SeqCst);
                    let handler = Arc::clone(&join_handler);
                    std::thread::spawn(move || {
                        let _ = handle_conn(stream, &expected, ready, &handler);
                    });
                }
                Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(_) => {
                    if join_shutdown.load(Ordering::SeqCst) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
        }
    });
    Ok(BoundListener {
        handle,
        _join: join,
    })
}

fn handle_conn(
    mut stream: TcpStream,
    expected_token: &str,
    ready: bool,
    handler: &Mutex<Option<DriveHandler>>,
) -> std::io::Result<()> {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let raw = read_http_request(&mut stream)?;
    let (method, path) = parse_request_line(&raw);
    if method == "GET" && path == "/health" {
        let body = serde_json::json!({
            "service": "vox-gui-drive",
            "ready": ready,
        })
        .to_string();
        return write_http(&mut stream, 200, &body);
    }
    if !path.starts_with("/v1/") {
        return write_http(&mut stream, 404, r#"{"error":"not_found"}"#);
    }
    let verb = path.trim_start_matches("/v1/").to_string();
    if !matches!(verb.as_str(), "set" | "send" | "state" | "show" | "ping") {
        return write_http(&mut stream, 404, r#"{"error":"not_found"}"#);
    }
    let provided = bearer_token(&raw);
    match provided {
        None => return write_http(&mut stream, 401, r#"{"error":"unauthorized"}"#),
        Some(got) if !constant_time_eq(got.as_bytes(), expected_token.as_bytes()) => {
            return write_http(&mut stream, 401, r#"{"error":"unauthorized"}"#);
        }
        Some(_) => {}
    }
    if verb == "ping" {
        let body = serde_json::json!({
            "ok": true,
            "ready": ready,
            "service": "vox-gui-drive",
        })
        .to_string();
        return write_http(&mut stream, 200, &body);
    }
    let body = body_after_headers(&raw).to_string();
    let req = DriveHttpRequest {
        verb: verb.clone(),
        body,
    };
    let response = {
        let guard = handler.lock().expect("drive handler lock");
        if let Some(cb) = guard.as_ref() {
            cb(req)
        } else if verb == "state" {
            DriveHttpResponse {
                status: 200,
                body: serde_json::to_string(&DriveState::empty_live())
                    .unwrap_or_else(|_| "{}".into()),
            }
        } else {
            DriveHttpResponse {
                status: 503,
                body: r#"{"error":"not_ready"}"#.into(),
            }
        }
    };
    write_http(&mut stream, response.status, &response.body)
}

fn parse_request_line(raw: &str) -> (String, String) {
    let line = raw.lines().next().unwrap_or("");
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts
        .next()
        .unwrap_or("")
        .split('?')
        .next()
        .unwrap_or("")
        .to_string();
    (method, path)
}

fn bearer_token(raw: &str) -> Option<String> {
    for line in raw.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.starts_with("authorization:") {
            let value = line.split_once(':')?.1.trim();
            return value
                .strip_prefix("Bearer ")
                .or_else(|| value.strip_prefix("bearer "))
                .map(|s| s.trim().to_string());
        }
    }
    None
}

fn body_after_headers(raw: &str) -> &str {
    raw.split("\r\n\r\n").nth(1).unwrap_or("")
}

fn read_http_request(stream: &mut TcpStream) -> std::io::Result<String> {
    const MAX: usize = 16 * 1024;
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    loop {
        let n = stream.read(&mut tmp)?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        if buf.len() > MAX {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "drive request too large",
            ));
        }
        if let Some(header_end) = find_header_end(&buf) {
            let headers = std::str::from_utf8(&buf[..header_end]).unwrap_or("");
            let content_len = parse_content_length(headers).unwrap_or(0);
            let needed = header_end.saturating_add(4).saturating_add(content_len);
            if needed > MAX {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "drive request too large",
                ));
            }
            while buf.len() < needed {
                let n = stream.read(&mut tmp)?;
                if n == 0 {
                    break;
                }
                buf.extend_from_slice(&tmp[..n]);
                if buf.len() > MAX {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "drive request too large",
                    ));
                }
            }
            break;
        }
    }
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn parse_content_length(headers: &str) -> Option<usize> {
    for line in headers.lines() {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        if name.eq_ignore_ascii_case("content-length") {
            return value.trim().parse().ok();
        }
    }
    None
}

fn write_http(stream: &mut TcpStream, status: u16, body: &str) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        409 => "Conflict",
        503 => "Service Unavailable",
        _ => "Error",
    };
    let header = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body.as_bytes())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn http_exchange(addr: SocketAddr, req: &str) -> (u16, String) {
        let mut stream = TcpStream::connect(addr).expect("connect");
        stream.write_all(req.as_bytes()).expect("write");
        let mut out = String::new();
        stream.read_to_string(&mut out).expect("read");
        let status = out
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let body = out.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
        (status, body)
    }

    #[test]
    fn health_names_service_without_token() {
        let h = bind_loopback("secret-token").unwrap();
        let (status, body) = http_exchange(
            h.addr(),
            "GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        );
        assert_eq!(status, 200);
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["service"], "vox-gui-drive");
        assert_eq!(v["ready"], false);
        assert!(v.get("token").is_none());
        h.shutdown();
    }

    #[test]
    fn missing_token_is_401() {
        let h = bind_loopback("secret-token").unwrap();
        let (status, _) = http_exchange(
            h.addr(),
            "POST /v1/state HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
        );
        assert_eq!(status, 401);
        h.shutdown();
    }

    #[test]
    fn wrong_token_is_401() {
        let h = bind_loopback("secret-token").unwrap();
        let (status, _) = http_exchange(
            h.addr(),
            "POST /v1/state HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer wrong-token\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
        );
        assert_eq!(status, 401);
        h.shutdown();
    }

    #[test]
    fn valid_token_state_is_200() {
        let h = bind_loopback("secret-token").unwrap();
        h.set_ready(true);
        h.set_handler(Arc::new(|req| {
            assert_eq!(req.verb, "state");
            DriveHttpResponse {
                status: 200,
                body: r#"{"plane":"live"}"#.into(),
            }
        }));
        let (status, body) = http_exchange(
            h.addr(),
            "POST /v1/state HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer secret-token\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}",
        );
        assert_eq!(status, 200);
        assert!(body.contains("live"));
        h.shutdown();
    }

    #[test]
    fn constant_time_eq_rejects_wrong_length() {
        assert!(!constant_time_eq(b"short", b"shorter"));
        assert!(constant_time_eq(b"same-token", b"same-token"));
    }

    #[test]
    fn ping_is_authenticated_and_does_not_need_handler() {
        let h = bind_loopback("secret-token").unwrap();
        let (status, body) = http_exchange(
            h.addr(),
            "GET /v1/ping HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer secret-token\r\nConnection: close\r\n\r\n",
        );
        assert_eq!(status, 200);
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        assert_eq!(v["ok"], true);
        assert_eq!(v["ready"], false);
        assert_eq!(v["service"], "vox-gui-drive");
        h.shutdown();
    }

    #[test]
    fn ping_without_token_is_401() {
        let h = bind_loopback("secret-token").unwrap();
        let (status, _) = http_exchange(
            h.addr(),
            "GET /v1/ping HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
        );
        assert_eq!(status, 401);
        h.shutdown();
    }

    #[test]
    fn parse_content_length_skips_request_line() {
        let headers = "POST /v1/set HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 35\r\n";
        assert_eq!(parse_content_length(headers), Some(35));
    }

    #[test]
    fn reads_body_after_split_packets() {
        let h = bind_loopback("secret-token").unwrap();
        h.set_handler(Arc::new(|req| {
            assert_eq!(req.verb, "set");
            assert_eq!(req.body, r#"{"model_override":"mens/e2e-smoke"}"#);
            DriveHttpResponse {
                status: 200,
                body: r#"{"ok":true}"#.into(),
            }
        }));
        let addr = h.addr();
        let mut stream = TcpStream::connect(addr).expect("connect");
        let _ = stream.set_nodelay(true);
        stream
            .write_all(b"POST /v1/set HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer secret-token\r\nContent-Length: 35\r\nConnection: close\r\n")
            .expect("headers");
        stream.flush().expect("flush headers");
        std::thread::sleep(Duration::from_millis(40));
        stream
            .write_all(b"\r\n{\"model_override\":\"mens/e2e-smoke\"}")
            .expect("body");
        stream.flush().expect("flush body");
        let mut out = String::new();
        stream.read_to_string(&mut out).expect("read");
        let status = out
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        assert_eq!(status, 200, "response={out:?}");
        h.shutdown();
    }
}
