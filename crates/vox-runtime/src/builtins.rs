//! Standard library builtins available to compiled Vox programs.
//!
//! Three-tier hashing strategy:
//! - `vox_hash_fast`   → XXH3-128 (20-80 GB/s, non-cryptographic, 32-char hex)
//! - `vox_hash_secure` → BLAKE3   (6-12 GB/s, cryptographic, 64-char hex)
//! - `vox_uuid`        → monotonic unique ID (timestamp + atomic counter)
//! - `vox_now_ms`      → current UNIX time in milliseconds

use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};
use vox_skills::ars_shim::{
    DefaultOpenClawRuntimeAdapter, OpenClawRuntimeAdapter, connect_default_runtime_adapter,
};
use rust_decimal::prelude::FromStr;

/// Fast, non-cryptographic hash using XXH3-128 (128-bit output).
///
/// Use for: HashMap keys, cache keys, dedup within a process, activity IDs
/// in hot workflow paths where you control the input (no adversarial keys).
///
/// Output: 32-char lowercase hex string (128-bit → 2 × u64 in hex).
/// Deterministic for the same input within a process; also cross-machine
/// deterministic (XXH3-128 is unkeyed / uses a fixed internal secret).
///
/// ⚠ NOT cryptographic — do not use for stored provenance hashes.
pub fn vox_hash_fast(input: &str) -> String {
    use xxhash_rust::xxh3::xxh3_128;
    let h = xxh3_128(input.as_bytes());
    format!("{h:032x}")
}

/// Cryptographic hash using BLAKE3 (256-bit output).
///
/// Use for: `input_hash` provenance stored in DB, content-addressable IDs
/// shared across machines / process lifetimes, data integrity verification.
///
/// Output: 64-char lowercase hex string (256-bit).
/// Fully deterministic, cross-machine stable, collision probability ≈ 2^-128.
///
/// ✅ Cryptographically secure. Safe to store permanently.
pub fn vox_hash_secure(input: &str) -> String {
    let hash = blake3::hash(input.as_bytes());
    hash.to_hex().to_string()
}

/// Generate a unique identifier.
///
/// Combines nanosecond-precision UNIX timestamp with a monotonic atomic counter
/// to guarantee uniqueness even within the same nanosecond (parallel workflow steps).
///
/// Format: `vox-{nanos_hex}-{counter_hex}`
/// Example: `vox-17a8c3f2d8b00000-0000000000000001`
pub fn vox_uuid() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("vox-{:016x}-{:016x}", nanos, count)
}

/// Current UNIX time in milliseconds.
pub fn vox_now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn vox_log_debug(message: &str) {
    tracing::debug!(target: "vox_runtime::builtins", "{message}");
}

pub fn vox_log_info(message: &str) {
    tracing::info!(target: "vox_runtime::builtins", "{message}");
}

pub fn vox_log_warn(message: &str) {
    tracing::warn!(target: "vox_runtime::builtins", "{message}");
}

pub fn vox_log_error(message: &str) {
    tracing::error!(target: "vox_runtime::builtins", "{message}");
}

/// Read a process environment variable (`std.env.get` in Vox scripts).
pub fn vox_env_get(key: &str) -> Option<String> {
    std::env::var(key).ok()
}

/// List directory entries (non-recursive) as file names (`std.fs.list_dir`).
pub fn vox_list_dir(path: &str) -> Result<Vec<String>, String> {
    let rd = std::fs::read_dir(path).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for ent in rd {
        let ent = ent.map_err(|e| e.to_string())?;
        out.push(ent.file_name().to_string_lossy().into_owned());
    }
    Ok(out)
}

/// Spawn a subprocess; on success returns exit code (`std.process.run`).
///
/// Non-zero exit is surfaced as `Err` so Vox `Result` can represent failure.
pub fn vox_process_run(cmd: &str, args: &[String]) -> Result<i32, String> {
    let mut c = std::process::Command::new(cmd);
    c.args(args);
    let st = c.status().map_err(|e| e.to_string())?;
    if st.success() {
        Ok(st.code().unwrap_or(0))
    } else {
        Err(format!("exit status {:?}", st.code()))
    }
}

/// Resolve `cmd` on the process `PATH` (`std.process.which` in Vox scripts).
///
/// Returns the absolute path as a string when found, or `None` when not found or resolution fails.
/// This is argv-first tooling: pass a single executable name or filename, not a shell pipeline.
pub fn vox_process_which(cmd: &str) -> Option<String> {
    let cmd = cmd.trim();
    if cmd.is_empty() {
        return None;
    }
    which::which(cmd)
        .ok()
        .map(|p| p.to_string_lossy().into_owned())
}

/// Captured stdout/stderr/exit from a subprocess (`std.process.run_capture` in Vox scripts).
///
/// Unlike [`vox_process_run`], this always returns **`Ok`** when the process was spawned and
/// output was read; non-zero exits are represented by the `exit` field (guard-style scripts).
/// I/O or spawn failures return `Err`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VoxProcessCapture {
    /// Process exit code (platform convention; may be negative if no code was available).
    pub exit: i32,
    pub stdout: String,
    pub stderr: String,
}

pub fn vox_process_run_capture(cmd: &str, args: &[String]) -> Result<VoxProcessCapture, String> {
    vox_process_run_capture_ex(cmd, args, "", &[])
}

/// Like [`vox_process_run_capture`], with optional working directory and extra `KEY=value` env pairs.
///
/// When `cwd` is empty, the subprocess inherits the current working directory. When `env_pairs`
/// is empty, no extra variables are applied (the process still inherits the parent environment).
pub fn vox_process_run_capture_ex(
    cmd: &str,
    args: &[String],
    cwd: &str,
    env_pairs: &[String],
) -> Result<VoxProcessCapture, String> {
    let mut c = std::process::Command::new(cmd);
    c.args(args);
    if !cwd.is_empty() {
        c.current_dir(cwd);
    }
    for pair in env_pairs {
        if let Some((k, v)) = pair.split_once('=') {
            if !k.is_empty() {
                c.env(k, v);
            }
        }
    }
    let out = c.output().map_err(|e| e.to_string())?;
    Ok(VoxProcessCapture {
        exit: out.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    })
}

/// Like [`vox_process_run`], with optional working directory and extra `KEY=value` env pairs.
pub fn vox_process_run_ex(
    cmd: &str,
    args: &[String],
    cwd: &str,
    env_pairs: &[String],
) -> Result<i32, String> {
    let mut c = std::process::Command::new(cmd);
    c.args(args);
    if !cwd.is_empty() {
        c.current_dir(cwd);
    }
    for pair in env_pairs {
        if let Some((k, v)) = pair.split_once('=') {
            if !k.is_empty() {
                c.env(k, v);
            }
        }
    }
    let st = c.status().map_err(|e| e.to_string())?;
    if st.success() {
        Ok(st.code().unwrap_or(0))
    } else {
        Err(format!("exit status {:?}", st.code()))
    }
}

/// Remove a directory tree (`std.fs.remove_dir_all`).
pub fn vox_fs_remove_dir_all(path: &str) -> Result<(), String> {
    std::fs::remove_dir_all(path).map_err(|e| e.to_string())
}

/// Copy a file (`src` → `dst`).
pub fn vox_fs_copy(src: &str, dst: &str) -> Result<(), String> {
    std::fs::copy(src, dst)
        .map_err(|e| e.to_string())
        .map(|_| ())
}

/// Join path segments with the platform separator. Empty input yields `"."`.
pub fn vox_path_join_many(segments: &[String]) -> String {
    if segments.is_empty() {
        return ".".to_string();
    }
    let mut p = std::path::PathBuf::from(&segments[0]);
    for s in segments.iter().skip(1) {
        p.push(s);
    }
    p.to_string_lossy().into_owned()
}

/// Read a string field from a JSON object (top-level). Returns error if JSON is invalid or not an object.
pub fn vox_json_read_str(json: &str, key: &str) -> Result<String, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let obj = v
        .as_object()
        .ok_or_else(|| "JSON root must be an object".to_string())?;
    let val = obj.get(key).ok_or_else(|| format!("missing key {key:?}"))?;
    val.as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("key {key:?} is not a string"))
}

/// Read an `f64` field from a JSON object (top-level). Integers are coerced to float.
pub fn vox_json_read_f64(json: &str, key: &str) -> Result<f64, String> {
    let v: serde_json::Value = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let obj = v
        .as_object()
        .ok_or_else(|| "JSON root must be an object".to_string())?;
    let val = obj.get(key).ok_or_else(|| format!("missing key {key:?}"))?;
    val.as_f64()
        .or_else(|| val.as_i64().map(|i| i as f64))
        .ok_or_else(|| format!("key {key:?} is not a number"))
}

/// JSON-encode a string value (quotes and escapes).
pub fn vox_json_quote(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}

/// HTTP GET text response body (`std.http.get_text`).
pub fn vox_http_get_text(url: &str) -> Result<String, String> {
    run_http_op(HttpOp::GetText {
        url: url.to_string(),
    })
}

/// HTTP POST with JSON body; returns text response body (`std.http.post_json`).
pub fn vox_http_post_json(url: &str, body_json: &str) -> Result<String, String> {
    run_http_op(HttpOp::PostJson {
        url: url.to_string(),
        body_json: body_json.to_string(),
    })
}

/// OpenClaw WS control-plane call from Vox scripts (`OpenClaw.call`).
pub fn vox_openclaw_call(method: &str, params_json: &str) -> Result<String, String> {
    run_openclaw_op(OpenClawOp::GatewayCall {
        method: method.to_string(),
        params_json: params_json.to_string(),
    })
}

/// OpenClaw convenience: list remote skills as JSON (`OpenClaw.list_skills`).
pub fn vox_openclaw_list_skills() -> Result<String, String> {
    run_openclaw_op(OpenClawOp::ListSkills)
}

/// OpenClaw convenience: subscribe domain (`OpenClaw.subscribe`).
pub fn vox_openclaw_subscribe(domain: &str) -> Result<String, String> {
    run_openclaw_op(OpenClawOp::Subscribe {
        domain: domain.to_string(),
    })
}

/// OpenClaw convenience: unsubscribe domain (`OpenClaw.unsubscribe`).
pub fn vox_openclaw_unsubscribe(domain: &str) -> Result<String, String> {
    run_openclaw_op(OpenClawOp::Unsubscribe {
        domain: domain.to_string(),
    })
}

/// OpenClaw convenience: notify domain (`OpenClaw.notify`).
pub fn vox_openclaw_notify(domain: &str, message: &str) -> Result<String, String> {
    run_openclaw_op(OpenClawOp::Notify {
        domain: domain.to_string(),
        message: message.to_string(),
    })
}

async fn connect_openclaw_adapter() -> Result<DefaultOpenClawRuntimeAdapter, String> {
    let clavis_token = vox_clavis::resolve_secret(vox_clavis::SecretId::OpenClawToken)
        .expose()
        .map(std::string::ToString::to_string);
    connect_default_runtime_adapter(clavis_token)
        .await
        .map_err(|e| format!("openclaw adapter connect failed: {e}"))
}

enum OpenClawOp {
    GatewayCall { method: String, params_json: String },
    ListSkills,
    Subscribe { domain: String },
    Unsubscribe { domain: String },
    Notify { domain: String, message: String },
}

enum HttpOp {
    GetText { url: String },
    PostJson { url: String, body_json: String },
}

struct HttpRequest {
    op: HttpOp,
    reply_tx: std::sync::mpsc::Sender<Result<String, String>>,
}

struct HttpWorker {
    tx: std::sync::mpsc::Sender<HttpRequest>,
}

fn http_worker() -> &'static HttpWorker {
    static WORKER: OnceLock<HttpWorker> = OnceLock::new();
    WORKER.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::channel::<HttpRequest>();
        std::thread::Builder::new()
            .name("vox-http-runtime".to_string())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| format!("http runtime init failed: {e}"));
                match runtime {
                    Ok(rt) => {
                        let client = vox_reqwest_defaults::client();
                        while let Ok(req) = rx.recv() {
                            let result = rt.block_on(handle_http_op(&client, req.op));
                            let _ = req.reply_tx.send(result);
                        }
                    }
                    Err(err) => {
                        while let Ok(req) = rx.recv() {
                            let _ = req.reply_tx.send(Err(err.clone()));
                        }
                    }
                }
            })
            .expect("spawn http runtime worker");
        HttpWorker { tx }
    })
}

fn run_http_op(op: HttpOp) -> Result<String, String> {
    let worker = http_worker();
    run_http_op_with_worker(worker, op)
}

fn run_http_op_with_worker(worker: &HttpWorker, op: HttpOp) -> Result<String, String> {
    let (reply_tx, reply_rx) = std::sync::mpsc::channel();
    worker
        .tx
        .send(HttpRequest { op, reply_tx })
        .map_err(|e| format!("http worker send failed: {e}"))?;
    reply_rx
        .recv()
        .map_err(|e| format!("http worker recv failed: {e}"))?
}

async fn handle_http_op(client: &reqwest::Client, op: HttpOp) -> Result<String, String> {
    match op {
        HttpOp::GetText { url } => {
            let resp = client.get(&url).send().await.map_err(|e| e.to_string())?;
            let status = resp.status();
            let text = resp.text().await.map_err(|e| e.to_string())?;
            if status.is_success() {
                Ok(text)
            } else {
                Err(format!("GET {url} failed with status {status}: {text}"))
            }
        }
        HttpOp::PostJson { url, body_json } => {
            let body: serde_json::Value =
                serde_json::from_str(&body_json).map_err(|e| format!("invalid JSON body: {e}"))?;
            let resp = client
                .post(&url)
                .json(&body)
                .send()
                .await
                .map_err(|e| e.to_string())?;
            let status = resp.status();
            let text = resp.text().await.map_err(|e| e.to_string())?;
            if status.is_success() {
                Ok(text)
            } else {
                Err(format!("POST {url} failed with status {status}: {text}"))
            }
        }
    }
}

struct OpenClawRequest {
    op: OpenClawOp,
    reply_tx: std::sync::mpsc::Sender<Result<String, String>>,
}

struct OpenClawWorker {
    tx: std::sync::mpsc::Sender<OpenClawRequest>,
}

fn openclaw_worker() -> &'static OpenClawWorker {
    static WORKER: OnceLock<OpenClawWorker> = OnceLock::new();
    WORKER.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::channel::<OpenClawRequest>();
        std::thread::Builder::new()
            .name("vox-openclaw-runtime".to_string())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| format!("openclaw runtime init failed: {e}"));
                match runtime {
                    Ok(rt) => {
                        let mut adapter: Option<DefaultOpenClawRuntimeAdapter> = None;
                        while let Ok(req) = rx.recv() {
                            let result = rt.block_on(handle_openclaw_op(&mut adapter, req.op));
                            let _ = req.reply_tx.send(result);
                        }
                    }
                    Err(err) => {
                        while let Ok(req) = rx.recv() {
                            let _ = req.reply_tx.send(Err(err.clone()));
                        }
                    }
                }
            })
            .expect("spawn openclaw runtime worker");
        OpenClawWorker { tx }
    })
}

fn run_openclaw_op(op: OpenClawOp) -> Result<String, String> {
    let worker = openclaw_worker();
    run_openclaw_op_with_worker(worker, op)
}

fn run_openclaw_op_with_worker(worker: &OpenClawWorker, op: OpenClawOp) -> Result<String, String> {
    let (reply_tx, reply_rx) = std::sync::mpsc::channel();
    worker
        .tx
        .send(OpenClawRequest { op, reply_tx })
        .map_err(|e| format!("openclaw worker send failed: {e}"))?;
    reply_rx
        .recv()
        .map_err(|e| format!("openclaw worker recv failed: {e}"))?
}

async fn handle_openclaw_op(
    adapter: &mut Option<DefaultOpenClawRuntimeAdapter>,
    op: OpenClawOp,
) -> Result<String, String> {
    match op {
        OpenClawOp::GatewayCall {
            method,
            params_json,
        } => {
            let params = serde_json::from_str::<serde_json::Value>(&params_json)
                .map_err(|e| format!("invalid params_json: {e}"))?;
            if adapter.is_none() {
                *adapter = Some(connect_openclaw_adapter().await?);
            }
            let adapter = adapter
                .as_mut()
                .ok_or_else(|| "openclaw adapter unavailable".to_string())?;
            let payload = adapter
                .gateway_call(&method, params)
                .await
                .map_err(|e| e.to_string())?;
            serde_json::to_string(&payload).map_err(|e| e.to_string())
        }
        OpenClawOp::ListSkills => {
            if adapter.is_none() {
                *adapter = Some(connect_openclaw_adapter().await?);
            }
            let adapter = adapter
                .as_mut()
                .ok_or_else(|| "openclaw adapter unavailable".to_string())?;
            let skills = adapter
                .list_remote_skills()
                .await
                .map_err(|e| e.to_string())?;
            serde_json::to_string(&serde_json::json!({ "skills": skills }))
                .map_err(|e| e.to_string())
        }
        OpenClawOp::Subscribe { domain } => {
            if adapter.is_none() {
                *adapter = Some(connect_openclaw_adapter().await?);
            }
            let adapter = adapter
                .as_mut()
                .ok_or_else(|| "openclaw adapter unavailable".to_string())?;
            let payload = adapter
                .subscribe_domain(&domain)
                .await
                .map_err(|e| e.to_string())?;
            serde_json::to_string(&payload).map_err(|e| e.to_string())
        }
        OpenClawOp::Unsubscribe { domain } => {
            if adapter.is_none() {
                *adapter = Some(connect_openclaw_adapter().await?);
            }
            let adapter = adapter
                .as_mut()
                .ok_or_else(|| "openclaw adapter unavailable".to_string())?;
            let payload = adapter
                .unsubscribe_domain(&domain)
                .await
                .map_err(|e| e.to_string())?;
            serde_json::to_string(&payload).map_err(|e| e.to_string())
        }
        OpenClawOp::Notify { domain, message } => {
            if adapter.is_none() {
                *adapter = Some(connect_openclaw_adapter().await?);
            }
            let adapter = adapter
                .as_mut()
                .ok_or_else(|| "openclaw adapter unavailable".to_string())?;
            let payload = adapter
                .notify_domain(&domain, &message)
                .await
                .map_err(|e| e.to_string())?;
            serde_json::to_string(&payload).map_err(|e| e.to_string())
        }
    }
}

// ── Browser (CDP / chromiumoxide; native host thread) ───────────────────────

enum BrowserOp {
    Open {
        url: String,
        headless: bool,
    },
    Close {
        page_id: String,
    },
    Goto {
        page_id: String,
        url: String,
    },
    Click {
        page_id: String,
        target: String,
    },
    Fill {
        page_id: String,
        target: String,
        value: String,
    },
    WaitFor {
        page_id: String,
        target: String,
        timeout_secs: u64,
    },
    Text {
        page_id: String,
        target: String,
    },
    Html {
        page_id: String,
        target: String,
    },
    Screenshot {
        page_id: String,
        path: String,
    },
}

enum BrowserReply {
    Unit,
    Str(String),
}

struct BrowserRequest {
    op: BrowserOp,
    reply_tx: std::sync::mpsc::Sender<Result<BrowserReply, String>>,
}

struct BrowserWorker {
    tx: std::sync::mpsc::Sender<BrowserRequest>,
}

fn browser_worker() -> &'static BrowserWorker {
    static WORKER: OnceLock<BrowserWorker> = OnceLock::new();
    WORKER.get_or_init(|| {
        let (tx, rx) = std::sync::mpsc::channel::<BrowserRequest>();
        std::thread::Builder::new()
            .name("vox-browser-runtime".to_string())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| format!("browser runtime init failed: {e}"));
                match runtime {
                    Ok(rt) => {
                        let eng = vox_browser::global_engine();
                        while let Ok(req) = rx.recv() {
                            let result = rt.block_on(handle_browser_op(&eng, req.op));
                            let _ = req.reply_tx.send(result);
                        }
                    }
                    Err(err) => {
                        while let Ok(req) = rx.recv() {
                            let _ = req.reply_tx.send(Err(err.clone()));
                        }
                    }
                }
            })
            .expect("spawn browser runtime worker");
        BrowserWorker { tx }
    })
}

fn run_browser_op(op: BrowserOp) -> Result<BrowserReply, String> {
    let worker = browser_worker();
    let (reply_tx, reply_rx) = std::sync::mpsc::channel();
    worker
        .tx
        .send(BrowserRequest { op, reply_tx })
        .map_err(|e| format!("browser worker send failed: {e}"))?;
    reply_rx
        .recv()
        .map_err(|e| format!("browser worker recv failed: {e}"))?
}

async fn handle_browser_op(
    eng: &std::sync::Arc<vox_browser::BrowserEngine>,
    op: BrowserOp,
) -> Result<BrowserReply, String> {
    match op {
        BrowserOp::Open { url, headless } => eng.open(&url, headless).await.map(BrowserReply::Str),
        BrowserOp::Close { page_id } => eng.close(&page_id).await.map(|_| BrowserReply::Unit),
        BrowserOp::Goto { page_id, url } => {
            eng.goto(&page_id, &url).await.map(|_| BrowserReply::Unit)
        }
        BrowserOp::Click { page_id, target } => eng
            .click(&page_id, &target)
            .await
            .map(|_| BrowserReply::Unit),
        BrowserOp::Fill {
            page_id,
            target,
            value,
        } => eng
            .fill(&page_id, &target, &value)
            .await
            .map(|_| BrowserReply::Unit),
        BrowserOp::WaitFor {
            page_id,
            target,
            timeout_secs,
        } => eng
            .wait_for(&page_id, &target, timeout_secs)
            .await
            .map(|_| BrowserReply::Unit),
        BrowserOp::Text { page_id, target } => {
            eng.text(&page_id, &target).await.map(BrowserReply::Str)
        }
        BrowserOp::Html { page_id, target } => {
            eng.html(&page_id, &target).await.map(BrowserReply::Str)
        }
        BrowserOp::Screenshot { page_id, path } => {
            eng.screenshot(&page_id, &path).await.map(BrowserReply::Str)
        }
    }
}

/// `Browser.open` — returns opaque `page_id` (`Result[str]` in Vox).
pub fn vox_browser_open(url: &str, headless: bool) -> Result<String, String> {
    match run_browser_op(BrowserOp::Open {
        url: url.to_string(),
        headless,
    })? {
        BrowserReply::Str(s) => Ok(s),
        BrowserReply::Unit => Err("browser: unexpected Unit reply for open".into()),
    }
}

pub fn vox_browser_close(page_id: &str) -> Result<(), String> {
    match run_browser_op(BrowserOp::Close {
        page_id: page_id.to_string(),
    })? {
        BrowserReply::Unit => Ok(()),
        BrowserReply::Str(_) => Err("browser: unexpected Str reply for close".into()),
    }
}

pub fn vox_browser_goto(page_id: &str, url: &str) -> Result<(), String> {
    match run_browser_op(BrowserOp::Goto {
        page_id: page_id.to_string(),
        url: url.to_string(),
    })? {
        BrowserReply::Unit => Ok(()),
        BrowserReply::Str(_) => Err("browser: unexpected Str reply for goto".into()),
    }
}

pub fn vox_browser_click(page_id: &str, target: &str) -> Result<(), String> {
    match run_browser_op(BrowserOp::Click {
        page_id: page_id.to_string(),
        target: target.to_string(),
    })? {
        BrowserReply::Unit => Ok(()),
        BrowserReply::Str(_) => Err("browser: unexpected Str reply for click".into()),
    }
}

pub fn vox_browser_fill(page_id: &str, target: &str, value: &str) -> Result<(), String> {
    match run_browser_op(BrowserOp::Fill {
        page_id: page_id.to_string(),
        target: target.to_string(),
        value: value.to_string(),
    })? {
        BrowserReply::Unit => Ok(()),
        BrowserReply::Str(_) => Err("browser: unexpected Str reply for fill".into()),
    }
}

pub fn vox_browser_wait_for(page_id: &str, target: &str, timeout_secs: u64) -> Result<(), String> {
    match run_browser_op(BrowserOp::WaitFor {
        page_id: page_id.to_string(),
        target: target.to_string(),
        timeout_secs,
    })? {
        BrowserReply::Unit => Ok(()),
        BrowserReply::Str(_) => Err("browser: unexpected Str reply for wait_for".into()),
    }
}

pub fn vox_browser_text(page_id: &str, target: &str) -> Result<String, String> {
    match run_browser_op(BrowserOp::Text {
        page_id: page_id.to_string(),
        target: target.to_string(),
    })? {
        BrowserReply::Str(s) => Ok(s),
        BrowserReply::Unit => Err("browser: unexpected Unit reply for text".into()),
    }
}

pub fn vox_browser_html(page_id: &str, target: &str) -> Result<String, String> {
    match run_browser_op(BrowserOp::Html {
        page_id: page_id.to_string(),
        target: target.to_string(),
    })? {
        BrowserReply::Str(s) => Ok(s),
        BrowserReply::Unit => Err("browser: unexpected Unit reply for html".into()),
    }
}

pub fn vox_browser_screenshot(page_id: &str, path: &str) -> Result<String, String> {
    match run_browser_op(BrowserOp::Screenshot {
        page_id: page_id.to_string(),
        path: path.to_string(),
    })? {
        BrowserReply::Str(s) => Ok(s),
        BrowserReply::Unit => Err("browser: unexpected Unit reply for screenshot".into()),
    }
}

/// Expand a glob pattern and return sorted paths as strings (`std.fs.glob`).
///
/// Patterns follow the Rust [`glob`] crate (e.g. `*.rs`, `target/**/*.toml`). Invalid patterns
/// return `Err`.
pub fn vox_fs_glob(pattern: &str) -> Result<Vec<String>, String> {
    let mut paths: Vec<String> = Vec::new();
    for entry in glob::glob(pattern).map_err(|e| e.to_string())? {
        let p = entry.map_err(|e| e.to_string())?;
        paths.push(p.to_string_lossy().into_owned());
    }
    paths.sort();
    Ok(paths)
}

/// Convert a `dec` value to a string (`std.dec.to_string`).
pub fn vox_dec_to_str(d: rust_decimal::Decimal) -> String {
    d.to_string()
}

/// Parse a string into a `dec` value (`std.dec.from_str`).
pub fn vox_str_to_dec(s: &str) -> Result<rust_decimal::Decimal, String> {
    rust_decimal::Decimal::from_str(s).map_err(|e| e.to_string())
}

/// Return JSON list of runtime capabilities (`std.meta.capabilities`).
pub fn vox_meta_capabilities() -> String {
    let mut caps = vec!["hashing", "fs", "process", "http"];
    #[cfg(feature = "database")]
    caps.push("database");

    if std::env::var("VOX_CHROME_EXECUTABLE").is_ok()
        || which::which("google-chrome").is_ok()
        || which::which("chromium").is_ok()
    {
        caps.push("browser");
    }

    serde_json::to_string(&caps).unwrap_or_else(|_| "[]".to_string())
}

/// Return JSON list of registered tools (`std.meta.tools`).
pub fn vox_meta_tools() -> String {
    // Note: To avoid circular dependency on vox-mcp-registry, this currently returns an empty list
    // until the registry is flattened or moved to a core crate.
    "[]".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_hash_is_deterministic() {
        assert_eq!(vox_hash_fast("hello world"), vox_hash_fast("hello world"));
        assert_eq!(vox_hash_fast("hello world").len(), 32);
    }

    #[test]
    fn fast_hash_differs_for_different_inputs() {
        assert_ne!(vox_hash_fast("foo"), vox_hash_fast("bar"));
    }

    #[test]
    fn list_dir_finds_file() {
        let dir = std::env::temp_dir().join(format!("vox-list-dir-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("x.txt"), b"a").unwrap();
        let res = vox_list_dir(dir.to_string_lossy().as_ref()).unwrap();
        assert!(res.iter().any(|n| n == "x.txt"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn fast_hash_differs_for_similar_inputs() {
        // Avalanche effect: single char change → totally different hash
        assert_ne!(vox_hash_fast("gain"), vox_hash_fast("Gain"));
        assert_ne!(vox_hash_fast("loss"), vox_hash_fast("los"));
    }

    #[test]
    fn secure_hash_is_deterministic() {
        assert_eq!(
            vox_hash_secure("hello world"),
            vox_hash_secure("hello world")
        );
        assert_eq!(vox_hash_secure("hello world").len(), 64);
    }

    #[tokio::test]
    async fn openclaw_gateway_call_invalid_json_is_reported_without_adapter() {
        let mut adapter = None;
        let err = handle_openclaw_op(
            &mut adapter,
            OpenClawOp::GatewayCall {
                method: "subscriptions.list".to_string(),
                params_json: "{not-valid-json".to_string(),
            },
        )
        .await
        .expect_err("invalid JSON must fail before adapter access");
        assert!(
            err.contains("invalid params_json"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn openclaw_worker_send_failure_is_reported() {
        let (tx, rx) = std::sync::mpsc::channel::<OpenClawRequest>();
        drop(rx);
        let worker = OpenClawWorker { tx };
        let err = run_openclaw_op_with_worker(&worker, OpenClawOp::ListSkills)
            .expect_err("send should fail when receiver is dropped");
        assert!(
            err.contains("openclaw worker send failed"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn openclaw_worker_recv_failure_is_reported() {
        let (tx, rx) = std::sync::mpsc::channel::<OpenClawRequest>();
        std::thread::spawn(move || {
            if let Ok(req) = rx.recv() {
                drop(req.reply_tx);
            }
        });
        let worker = OpenClawWorker { tx };
        let err = run_openclaw_op_with_worker(&worker, OpenClawOp::ListSkills)
            .expect_err("recv should fail when worker closes reply channel");
        assert!(
            err.contains("openclaw worker recv failed"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn secure_hash_known_vector() {
        // BLAKE3 test vector from official spec
        let h = vox_hash_secure("");
        assert_eq!(
            h,
            "af1349b9f5f9a1a6a0404dea36dcc9499bcb25c9adc112b7cc9a93cae41f3262"
        );
    }

    #[test]
    fn secure_hash_differs_from_fast_hash() {
        let input = "test input";
        assert_ne!(vox_hash_fast(input), vox_hash_secure(input));
    }

    #[test]
    fn uuid_is_unique() {
        let u1 = vox_uuid();
        let u2 = vox_uuid();
        assert_ne!(u1, u2);
        assert!(u1.starts_with("vox-"));
        // Format: vox-{16 hex}-{16 hex}
        let parts: Vec<&str> = u1.splitn(3, '-').collect();
        assert_eq!(parts.len(), 3);
        assert_eq!(parts[1].len(), 16);
        assert_eq!(parts[2].len(), 16);
    }

    #[test]
    fn uuid_counter_is_monotonic() {
        let ids: Vec<String> = (0..100).map(|_| vox_uuid()).collect();
        // All must be unique
        let unique: std::collections::HashSet<&String> = ids.iter().collect();
        assert_eq!(unique.len(), 100);
    }

    #[test]
    fn now_ms_is_reasonable() {
        let ts = vox_now_ms();
        // Must be after 2025-01-01T00:00:00Z (1735689600000 ms)
        assert!(ts > 1_735_689_600_000, "timestamp too old: {}", ts);
    }

    #[test]
    fn process_run_capture_reads_echo() {
        let cap = if cfg!(windows) {
            vox_process_run_capture("cmd.exe", &["/C".into(), "echo".into(), "hello".into()])
        } else {
            vox_process_run_capture("echo", &["hello".into()])
        }
        .expect("spawn echo");
        assert_eq!(cap.exit, 0);
        assert!(cap.stdout.contains("hello"), "stdout={:?}", cap.stdout);
    }

    #[test]
    fn process_which_finds_system_executable() {
        let name = if cfg!(windows) { "cmd.exe" } else { "sh" };
        let resolved = vox_process_which(name);
        assert!(
            resolved.is_some(),
            "expected to resolve {name} on PATH, got None"
        );
        let p = resolved.unwrap();
        assert!(!p.trim().is_empty(), "empty path for {name}");
    }

    #[test]
    fn fs_glob_finds_temp_file() {
        let dir = std::env::temp_dir().join(format!("vox-glob-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("a.txt"), b"x").unwrap();
        let pat = dir.join("*.txt").to_string_lossy().into_owned();
        let got = vox_fs_glob(&pat).unwrap();
        assert!(
            got.iter().any(|p| p.ends_with("a.txt")),
            "glob {pat} -> {got:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn path_join_many_joins_segments() {
        let segs = vec!["a".into(), "b".into(), "c".into()];
        let j = vox_path_join_many(&segs);
        assert!(j.contains("a") && j.contains("b") && j.contains("c"));
        assert_eq!(vox_path_join_many(&[]), ".");
    }

    #[test]
    fn json_read_str_and_f64() {
        let raw = r#"{"name":"x","n":3,"f":1.5}"#;
        assert_eq!(vox_json_read_str(raw, "name").unwrap(), "x");
        assert!((vox_json_read_f64(raw, "n").unwrap() - 3.0).abs() < f64::EPSILON);
        assert!((vox_json_read_f64(raw, "f").unwrap() - 1.5).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn http_invalid_json_body_is_rejected_before_network() {
        let client = vox_reqwest_defaults::client();
        let err = handle_http_op(
            &client,
            HttpOp::PostJson {
                url: "http://127.0.0.1:1".to_string(),
                body_json: "{not-json".to_string(),
            },
        )
        .await
        .expect_err("invalid JSON body should fail before HTTP call");
        assert!(err.contains("invalid JSON body"), "unexpected error: {err}");
    }

    #[tokio::test]
    async fn http_invalid_url_reports_error() {
        let client = vox_reqwest_defaults::client();
        let err = handle_http_op(
            &client,
            HttpOp::GetText {
                url: "notaurl".to_string(),
            },
        )
        .await
        .expect_err("invalid URL must fail");
        assert!(!err.trim().is_empty(), "error should not be empty");
    }

    #[test]
    fn http_worker_send_failure_is_reported() {
        let (tx, rx) = std::sync::mpsc::channel::<HttpRequest>();
        drop(rx);
        let worker = HttpWorker { tx };
        let err = run_http_op_with_worker(
            &worker,
            HttpOp::GetText {
                url: "https://example.invalid".to_string(),
            },
        )
        .expect_err("send should fail when receiver is dropped");
        assert!(
            err.contains("http worker send failed"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn http_worker_recv_failure_is_reported() {
        let (tx, rx) = std::sync::mpsc::channel::<HttpRequest>();
        std::thread::spawn(move || {
            if let Ok(req) = rx.recv() {
                drop(req.reply_tx);
            }
        });
        let worker = HttpWorker { tx };
        let err = run_http_op_with_worker(
            &worker,
            HttpOp::GetText {
                url: "https://example.invalid".to_string(),
            },
        )
        .expect_err("recv should fail when worker closes reply channel");
        assert!(
            err.contains("http worker recv failed"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn process_run_capture_ex_respects_cwd() {
        let dir = std::env::temp_dir().join(format!("vox-cwd-cap-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("marker.txt"), b"ok").unwrap();
        let cap = if cfg!(windows) {
            vox_process_run_capture_ex(
                "cmd.exe",
                &["/C".into(), "type".into(), "marker.txt".into()],
                &dir.to_string_lossy(),
                &[],
            )
        } else {
            vox_process_run_capture_ex("cat", &["marker.txt".into()], &dir.to_string_lossy(), &[])
        }
        .unwrap();
        assert_eq!(cap.exit, 0);
        assert!(cap.stdout.contains("ok"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
