//! Browser preview and agent live-view commands for the Vox GUI.
//!
//! - **Preview tab:** embed a localhost dev URL (direct URL or Vite via
//!   [`vox_cli::frontend::OrchestratedViteGuard`]).
//! - **Agent tab:** mirror CDP browser sessions via periodic PNG frames on
//!   [`BROWSER_FRAME_EVENT`].

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine;
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tokio::sync::Mutex;
use vox_cli::frontend::OrchestratedViteGuard;
use vox_foundation::protocol::orch_daemon_method;
use vox_orchestrator::orch_daemon::OrchDaemonClient;

use crate::commands::daemon::PersistentDaemon;
use crate::commands::process_util::quiet_command;

/// Tauri event channel carrying browser live-view frames to the UI.
pub const BROWSER_FRAME_EVENT: &str = "vox://browser-frame";
/// Tauri event emitted when a preview URL becomes available.
pub const PREVIEW_AVAILABLE_EVENT: &str = "vox://preview-available";

const FRAME_INTERVAL_MS: u64 = 3_000;
const MAX_ACTION_LOG: usize = 50;

#[derive(Debug, Clone, Serialize)]
pub struct PreviewStatus {
    pub active: bool,
    pub url: Option<String>,
    pub app_dir: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserFramePayload {
    pub timestamp_ms: u64,
    pub page_id: Option<String>,
    pub image_base64: Option<String>,
    pub viewport_width: Option<u32>,
    pub viewport_height: Option<u32>,
    pub mime: Option<String>,
    pub action_log: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlaywrightValidateResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub preview_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PreviewAvailablePayload {
    pub url: String,
    pub app_dir: Option<String>,
    pub source: String,
}

struct PreviewRuntime {
    guard: Option<OrchestratedViteGuard>,
    url: Option<String>,
    app_dir: Option<String>,
    source: String,
}

struct BrowserSession {
    selected_page_id: Option<String>,
    headless: bool,
    control_mode: String,
    action_log: Vec<String>,
}

pub struct BrowserState {
    preview: Mutex<PreviewRuntime>,
    session: Mutex<BrowserSession>,
}

impl Default for BrowserState {
    fn default() -> Self {
        Self {
            preview: Mutex::new(PreviewRuntime {
                guard: None,
                url: None,
                app_dir: None,
                source: "none".to_string(),
            }),
            session: Mutex::new(BrowserSession {
                selected_page_id: None,
                headless: true,
                control_mode: "you".to_string(),
                action_log: Vec::new(),
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct BrowserPageSummary {
    pub page_id: String,
    pub url: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct BrowserPageInfo {
    pub page_id: String,
    pub url: String,
    pub title: String,
    pub can_go_back: bool,
    pub can_go_forward: bool,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn resolve_app_dir(app_dir: Option<String>) -> Result<PathBuf, String> {
    let repo = vox_repository::resolve_repo_root_for_ci();
    match app_dir {
        None => Ok(repo),
        Some(raw) if raw.trim().is_empty() => Ok(repo),
        Some(raw) => {
            let p = PathBuf::from(raw.trim());
            if p.is_absolute() {
                Ok(p)
            } else {
                Ok(repo.join(p))
            }
        }
    }
}

/// True when `package.json` at `pkg_path` declares `scripts.<script>`.
fn package_has_script(pkg_path: &std::path::Path, script: &str) -> bool {
    let Ok(raw) = std::fs::read_to_string(pkg_path) else {
        return false;
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return false;
    };
    json.get("scripts")
        .and_then(|s| s.get(script))
        .and_then(|v| v.as_str())
        .is_some()
}

fn push_action_log(session: &mut BrowserSession, line: impl Into<String>) {
    session.action_log.push(line.into());
    if session.action_log.len() > MAX_ACTION_LOG {
        let drain = session.action_log.len() - MAX_ACTION_LOG;
        session.action_log.drain(0..drain);
    }
}

async fn mcp_tool_call(
    daemon: &PersistentDaemon,
    tool: &str,
    args: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let addr = daemon.ensure().await?;
    let client = match daemon.token().await {
        Some(token) => OrchDaemonClient::with_token(addr, token),
        None => OrchDaemonClient::new(addr),
    };
    client
        .call(
            orch_daemon_method::TOOL_CALL,
            serde_json::json!({ "name": tool, "args": args }),
        )
        .await
        .map_err(|e| format!("MCP tool '{tool}' failed: {e}"))
}

/// Unwraps the daemon `orch.tool_call` envelope (a parsed [`ToolResult`]:
/// `{ success, data, error, remediation, meta }`) into its `data` payload,
/// surfacing `error` (+ `remediation`) on failure.
fn mcp_data(result: &serde_json::Value) -> Result<serde_json::Value, String> {
    if result.get("success") == Some(&serde_json::Value::Bool(false)) {
        let err = result
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("tool failed");
        return match result.get("remediation").and_then(|v| v.as_str()) {
            Some(rem) if !rem.is_empty() => Err(format!("{err} ({rem})")),
            _ => Err(err.to_string()),
        };
    }
    // Successful results carry the payload under `data`; fall back to the whole
    // value for tools that return a bare object.
    Ok(result
        .get("data")
        .cloned()
        .unwrap_or_else(|| result.clone()))
}

fn extract_page_id_from_mcp(result: &serde_json::Value) -> Option<String> {
    let data = mcp_data(result).ok()?;
    data.get("page_id")
        .and_then(|v| v.as_str())
        .map(ToString::to_string)
}

/// Host of a preview URL, parsed by hand (no `url` crate). Used to keep the
/// iframe on loopback so named/attach work cannot widen it to arbitrary sites.
fn preview_url_host(url: &str) -> String {
    let trimmed = url.trim();
    let authority = trimmed
        .split_once("://")
        .map_or(trimmed, |(_, rest)| rest)
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();
    if authority.contains('\\') {
        return String::new();
    }
    let authority = authority.rsplit('@').next().unwrap_or_default();
    if authority.starts_with('[') {
        authority
            .find(']')
            .map_or(authority, |end| &authority[..=end])
            .to_ascii_lowercase()
    } else {
        authority
            .split(':')
            .next()
            .unwrap_or_default()
            .trim_end_matches('.')
            .to_ascii_lowercase()
    }
}

fn is_loopback_preview_url(url: &str) -> bool {
    matches!(
        preview_url_host(url).as_str(),
        "localhost" | "127.0.0.1" | "[::1]"
    )
}

fn require_loopback_preview_url(url: &str) -> Result<(), String> {
    if is_loopback_preview_url(url) {
        Ok(())
    } else {
        Err(
            "preview URL must be loopback (localhost, 127.0.0.1, or [::1]); \
named/Connect Chrome sessions use the agent live view, not the iframe"
                .to_string(),
        )
    }
}

/// `needs_human` from MCP is surfaced as toast + `action_log`, not NeedsYou.
fn needs_human_reason(result: &serde_json::Value) -> Option<String> {
    let data = mcp_data(result).ok()?;
    if data.get("needs_human").and_then(|v| v.as_bool()) != Some(true) {
        return None;
    }
    Some(
        data.get("reason")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .unwrap_or("needs_human")
            .to_string(),
    )
}

fn log_needs_human(session: &mut BrowserSession, result: &serde_json::Value) {
    if let Some(reason) = needs_human_reason(result) {
        push_action_log(session, format!("needs_human: {reason}"));
    }
}

fn frame_bytes_from_mcp_data(
    data: &serde_json::Value,
    cache_root: &std::path::Path,
) -> Result<(String, Option<u32>, Option<u32>, String), String> {
    let width = data
        .get("width")
        .or_else(|| data.get("viewport_width"))
        .and_then(|v| v.as_u64())
        .map(|v| u32::try_from(v).map_err(|_| "screenshot width exceeds u32".to_string()))
        .transpose()?;
    let height = data
        .get("height")
        .or_else(|| data.get("viewport_height"))
        .and_then(|v| v.as_u64())
        .map(|v| u32::try_from(v).map_err(|_| "screenshot height exceeds u32".to_string()))
        .transpose()?;
    if let Some(path) = data.get("path").and_then(|v| v.as_str()) {
        let p = std::path::Path::new(path);
        if !p.is_absolute() {
            return Err("screenshot path must be absolute".into());
        }
        if !vox_config::paths::path_is_under(cache_root, p) {
            return Err("screenshot path escaped cache jail".into());
        }
        let metadata = std::fs::metadata(p).map_err(|e| e.to_string())?;
        if metadata.len() > 400_000 {
            return Err("screenshot frame exceeds image part cap".into());
        }
        let bytes = std::fs::read(p).map_err(|e| e.to_string())?;
        if bytes.len() > 400_000 {
            return Err("screenshot frame exceeds image part cap".into());
        }
        let mime = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
            "image/png".to_string()
        } else if bytes.len() >= 3 && bytes[0] == 0xff && bytes[1] == 0xd8 && bytes[2] == 0xff {
            "image/jpeg".to_string()
        } else {
            return Err("screenshot path is not PNG or JPEG".into());
        };
        let image_base64 = base64::engine::general_purpose::STANDARD.encode(bytes);
        return Ok((image_base64, width, height, mime));
    }
    if let Some(image_base64) = data.get("image_base64").and_then(|v| v.as_str()) {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(image_base64.trim())
            .map_err(|_| "screenshot image_base64 is invalid".to_string())?;
        if bytes.len() > 400_000 {
            return Err("screenshot frame exceeds image part cap".into());
        }
        let mime = if bytes.starts_with(&[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]) {
            "image/png"
        } else if bytes.len() >= 3 && bytes[0] == 0xff && bytes[1] == 0xd8 && bytes[2] == 0xff {
            "image/jpeg"
        } else {
            return Err("screenshot image_base64 is not PNG or JPEG".into());
        };
        let image_base64 = base64::engine::general_purpose::STANDARD.encode(bytes);
        return Ok((image_base64, width, height, mime.to_string()));
    }
    Err("screenshot_viewport returned no path or image_base64".into())
}

async fn capture_frame_png_base64(
    daemon: &PersistentDaemon,
    page_id: &str,
) -> Result<(String, Option<u32>, Option<u32>, String), String> {
    let cache_root = vox_config::paths::browser_frames_cache_dir();
    let screencast = mcp_tool_call(
        daemon,
        "vox_browser_screencast_frame",
        serde_json::json!({ "page_id": page_id }),
    )
    .await;
    if let Ok(result) = screencast {
        let data = mcp_data(&result)?;
        if data.get("path").is_some() || data.get("image_base64").is_some() {
            return frame_bytes_from_mcp_data(&data, &cache_root);
        }
    }
    let result = mcp_tool_call(
        daemon,
        "vox_browser_screenshot_viewport",
        serde_json::json!({ "page_id": page_id }),
    )
    .await?;
    let data = mcp_data(&result)?;
    frame_bytes_from_mcp_data(&data, &cache_root)
}

/// Spawn a background task that polls the active CDP page and emits
/// [`BROWSER_FRAME_EVENT`] every ~3 seconds. Idle (no active session) iterations
/// are cheap: a single mutex read and an early continue.
pub fn spawn_browser_frame_stream(
    app_handle: AppHandle,
    daemon: Arc<PersistentDaemon>,
    browser_state: Arc<BrowserState>,
) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(FRAME_INTERVAL_MS)).await;
            let (page_id, action_log) = {
                let session = browser_state.session.lock().await;
                (session.selected_page_id.clone(), session.action_log.clone())
            };
            let Some(page_id) = page_id else {
                continue;
            };
            let capture = capture_frame_png_base64(&daemon, &page_id).await;
            let payload = BrowserFramePayload {
                timestamp_ms: now_ms(),
                page_id: Some(page_id),
                image_base64: capture.as_ref().ok().map(|c| c.0.clone()),
                viewport_width: capture.as_ref().ok().and_then(|c| c.1),
                viewport_height: capture.as_ref().ok().and_then(|c| c.2),
                mime: capture.as_ref().ok().map(|c| c.3.clone()),
                action_log,
                error: capture.err().map(|e| e.to_string()),
            };
            let _ = app_handle.emit(BROWSER_FRAME_EVENT, payload);
        }
    });
}

/// Bootstrap preview discovery from environment for CodeGen/dev flows that
/// already expose `VOX_SSR_DEV_URL`.
fn approved_loopback_preview_url(raw: &str) -> Option<String> {
    let url = raw.trim();
    if url.is_empty() || !is_loopback_preview_url(url) {
        None
    } else {
        Some(url.to_string())
    }
}

pub fn emit_preview_available_from_env(app_handle: AppHandle, browser_state: Arc<BrowserState>) {
    if let Ok(raw) = std::env::var("VOX_SSR_DEV_URL")
        && let Some(url) = approved_loopback_preview_url(&raw)
    {
        tokio::spawn(async move {
            {
                let mut preview = browser_state.preview.lock().await;
                preview.url = Some(url.clone());
                preview.source = "env".to_string();
            }
            let _ = app_handle.emit(
                PREVIEW_AVAILABLE_EVENT,
                PreviewAvailablePayload {
                    url,
                    app_dir: None,
                    source: "env".to_string(),
                },
            );
        });
    }
}

#[tauri::command]
pub async fn preview_status(
    browser_state: tauri::State<'_, Arc<BrowserState>>,
) -> Result<PreviewStatus, String> {
    let preview = browser_state.preview.lock().await;
    Ok(PreviewStatus {
        active: preview.url.is_some(),
        url: preview.url.clone(),
        app_dir: preview.app_dir.clone(),
        source: preview.source.clone(),
    })
}

#[derive(serde::Deserialize)]
pub struct PreviewStartInput {
    pub url: Option<String>,
    pub app_dir: Option<String>,
}

#[tauri::command]
pub async fn preview_start(
    app: AppHandle,
    browser_state: tauri::State<'_, Arc<BrowserState>>,
    input: PreviewStartInput,
) -> Result<PreviewStatus, String> {
    let mut preview = browser_state.preview.lock().await;
    // Tear down any prior preview.
    preview.guard = None;

    if let Some(url) = input
        .url
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        require_loopback_preview_url(url)?;
        preview.url = Some(url.to_string());
        preview.app_dir = input.app_dir.clone();
        preview.source = "url".to_string();
        let _ = app.emit(
            PREVIEW_AVAILABLE_EVENT,
            PreviewAvailablePayload {
                url: url.to_string(),
                app_dir: preview.app_dir.clone(),
                source: preview.source.clone(),
            },
        );
        return Ok(PreviewStatus {
            active: true,
            url: preview.url.clone(),
            app_dir: preview.app_dir.clone(),
            source: preview.source.clone(),
        });
    }

    let app_dir = resolve_app_dir(input.app_dir)?;
    let pkg_path = app_dir.join("package.json");
    if !pkg_path.is_file() {
        return Err(format!(
            "no package.json in {}; provide a direct url or a Vite app directory",
            app_dir.display()
        ));
    }
    // OrchestratedViteGuard supports either `dev:ssr-upstream` (preferred) or
    // plain `dev`. Fail fast with an actionable message when neither exists.
    if !package_has_script(&pkg_path, "dev:ssr-upstream") && !package_has_script(&pkg_path, "dev") {
        return Err(format!(
            "{} has no \"dev:ssr-upstream\" or \"dev\" script (required for preview orchestration). \
Use a `vox build` web app output, or start the dev server yourself and pass its URL directly.",
            pkg_path.display()
        ));
    }

    let app_dir_clone = app_dir.clone();
    let (guard, inject) =
        tokio::task::spawn_blocking(move || OrchestratedViteGuard::maybe_spawn(&app_dir_clone))
            .await
            .map_err(|e| format!("spawn_blocking: {e}"))?
            .map_err(|e| e.to_string())?;

    // Resolve the dev URL from the SSOT rather than hardcoding: an explicit
    // VOX_SSR_DEV_URL wins, else the pair the guard injects, else `vox dev`
    // default port.
    let url = std::env::var("VOX_SSR_DEV_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| inject.as_ref().map(|(_, v)| v.clone()))
        .unwrap_or_else(|| "http://127.0.0.1:3000".to_string());
    require_loopback_preview_url(&url)?;
    preview.guard = Some(guard);
    preview.url = Some(url.clone());
    preview.app_dir = Some(app_dir.to_string_lossy().to_string());
    preview.source = "vite".to_string();
    let _ = app.emit(
        PREVIEW_AVAILABLE_EVENT,
        PreviewAvailablePayload {
            url: url.clone(),
            app_dir: preview.app_dir.clone(),
            source: preview.source.clone(),
        },
    );

    Ok(PreviewStatus {
        active: true,
        url: Some(url),
        app_dir: preview.app_dir.clone(),
        source: preview.source.clone(),
    })
}

#[tauri::command]
pub async fn preview_stop(
    browser_state: tauri::State<'_, Arc<BrowserState>>,
) -> Result<PreviewStatus, String> {
    let mut preview = browser_state.preview.lock().await;
    preview.guard = None;
    preview.url = None;
    preview.app_dir = None;
    preview.source = "none".to_string();
    Ok(PreviewStatus {
        active: false,
        url: None,
        app_dir: None,
        source: "none".to_string(),
    })
}

#[derive(serde::Deserialize)]
pub struct BrowserOpenInput {
    pub url: String,
    pub headless: Option<bool>,
    pub mode: Option<String>,
    pub profile_id: Option<String>,
    pub save_profile: Option<bool>,
    pub cdp_url: Option<String>,
}

/// `None` / `"ephemeral"` → `vox_browser_open`. Named / Connect Chrome → `vox_browser_open_ex`.
fn open_session_mcp_call(
    input: &BrowserOpenInput,
) -> Result<(&'static str, serde_json::Value), String> {
    let headless = input.headless.unwrap_or(true);
    let mode = input
        .mode
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_ascii_lowercase);
    match mode.as_deref() {
        None | Some("ephemeral") => Ok((
            "vox_browser_open",
            serde_json::json!({ "url": input.url, "headless": headless }),
        )),
        Some("named") => Ok((
            "vox_browser_open_ex",
            serde_json::json!({
                "url": input.url,
                "headless": headless,
                "mode": "named",
                "profile_id": input.profile_id,
                "save_profile": input.save_profile.unwrap_or(false),
                "cdp_url": serde_json::Value::Null,
            }),
        )),
        Some("connect-chrome") => Ok((
            "vox_browser_open_ex",
            serde_json::json!({
                "url": input.url,
                "headless": headless,
                "mode": "attach",
                "profile_id": serde_json::Value::Null,
                "save_profile": input.save_profile.unwrap_or(false),
                "cdp_url": input.cdp_url,
            }),
        )),
        Some(other) => Err(format!(
            "unknown launch mode {other:?}; use ephemeral, named, or connect-chrome"
        )),
    }
}

#[tauri::command]
pub async fn browser_open_session(
    browser_state: tauri::State<'_, Arc<BrowserState>>,
    daemon: tauri::State<'_, Arc<PersistentDaemon>>,
    input: BrowserOpenInput,
) -> Result<serde_json::Value, String> {
    let headless = input.headless.unwrap_or(true);
    let (tool, args) = open_session_mcp_call(&input)?;
    let result = mcp_tool_call(&daemon, tool, args).await?;
    let page_id = extract_page_id_from_mcp(&result);
    if let Some(ref page_id) = page_id {
        mcp_tool_call(
            &daemon,
            "vox_browser_set_viewport",
            serde_json::json!({ "page_id": page_id, "width": 1280, "height": 800, "actor": "human" }),
        )
        .await
        .and_then(|v| mcp_data(&v).map(|_| ()))?;
        mcp_tool_call(
            &daemon,
            "vox_browser_set_control_lock",
            serde_json::json!({ "page_id": page_id, "owner": "human" }),
        )
        .await
        .and_then(|v| mcp_data(&v).map(|_| ()))?;
    }
    let needs_human = needs_human_reason(&result);
    let mut session = browser_state.session.lock().await;
    session.selected_page_id = page_id.clone();
    session.headless = headless;
    push_action_log(
        &mut session,
        format!(
            "open {} (headless={headless}) page_id={:?}",
            input.url, page_id
        ),
    );
    if let Some(ref reason) = needs_human {
        push_action_log(&mut session, format!("needs_human: {reason}"));
    }
    Ok(serde_json::json!({
        "page_id": page_id,
        "headless": headless,
        "needs_human": needs_human.is_some(),
        "reason": needs_human,
        "raw": result,
    }))
}

#[tauri::command]
pub async fn browser_close_session(
    browser_state: tauri::State<'_, Arc<BrowserState>>,
    daemon: tauri::State<'_, Arc<PersistentDaemon>>,
) -> Result<(), String> {
    let page_id = {
        let session = browser_state.session.lock().await;
        session.selected_page_id.clone()
    };
    let Some(page_id) = page_id else {
        return Ok(());
    };
    let _ = mcp_tool_call(
        &daemon,
        "vox_browser_set_control_lock",
        serde_json::json!({ "page_id": page_id, "owner": "none" }),
    )
    .await;
    mcp_tool_call(
        &daemon,
        "vox_browser_close",
        serde_json::json!({ "page_id": page_id, "actor": "human" }),
    )
    .await?;
    let mut session = browser_state.session.lock().await;
    push_action_log(&mut session, "close session");
    session.selected_page_id = None;
    Ok(())
}

#[derive(serde::Deserialize)]
pub struct BrowserCloseInput {
    pub page_id: String,
}

#[tauri::command]
pub async fn browser_close_page(
    browser_state: tauri::State<'_, Arc<BrowserState>>,
    daemon: tauri::State<'_, Arc<PersistentDaemon>>,
    input: BrowserCloseInput,
) -> Result<(), String> {
    let page_id = input.page_id.trim().to_string();
    if page_id.is_empty() {
        return Err("page_id is required".to_string());
    }
    let _ = mcp_tool_call(
        &daemon,
        "vox_browser_set_control_lock",
        serde_json::json!({ "page_id": page_id, "owner": "none" }),
    )
    .await;
    mcp_tool_call(
        &daemon,
        "vox_browser_close",
        serde_json::json!({ "page_id": page_id, "actor": "human" }),
    )
    .await?;
    let mut session = browser_state.session.lock().await;
    if session.selected_page_id.as_deref() == Some(input.page_id.as_str()) {
        session.selected_page_id = None;
    }
    push_action_log(&mut session, format!("close {}", input.page_id));
    Ok(())
}

fn snapshot_mcp_args(page_id: &str) -> serde_json::Value {
    serde_json::json!({
        "page_id": page_id,
        "include_boxes": true,
    })
}

/// Overlay enable path: MCP `vox_browser_snapshot` with boxes for `[eN]` labels.
#[tauri::command]
pub async fn browser_snapshot(
    browser_state: tauri::State<'_, Arc<BrowserState>>,
    daemon: tauri::State<'_, Arc<PersistentDaemon>>,
) -> Result<serde_json::Value, String> {
    let page_id = {
        let session = browser_state.session.lock().await;
        session
            .selected_page_id
            .clone()
            .ok_or_else(|| "no selected page; open or attach first".to_string())?
    };
    let result =
        mcp_tool_call(&daemon, "vox_browser_snapshot", snapshot_mcp_args(&page_id)).await?;
    mcp_data(&result)
}

#[tauri::command]
pub async fn browser_screenshot_frame(
    browser_state: tauri::State<'_, Arc<BrowserState>>,
    daemon: tauri::State<'_, Arc<PersistentDaemon>>,
) -> Result<BrowserFramePayload, String> {
    let (page_id, action_log) = {
        let session = browser_state.session.lock().await;
        (session.selected_page_id.clone(), session.action_log.clone())
    };
    let Some(page_id) = page_id else {
        return Ok(BrowserFramePayload {
            timestamp_ms: now_ms(),
            page_id: None,
            image_base64: None,
            viewport_width: None,
            viewport_height: None,
            mime: None,
            action_log,
            error: Some("no active browser session; call browser_open_session first".to_string()),
        });
    };
    match capture_frame_png_base64(&daemon, &page_id).await {
        Ok((image_base64, viewport_width, viewport_height, mime)) => Ok(BrowserFramePayload {
            timestamp_ms: now_ms(),
            page_id: Some(page_id),
            image_base64: Some(image_base64),
            viewport_width,
            viewport_height,
            mime: Some(mime),
            action_log,
            error: None,
        }),
        Err(e) => Ok(BrowserFramePayload {
            timestamp_ms: now_ms(),
            page_id: Some(page_id),
            image_base64: None,
            viewport_width: None,
            viewport_height: None,
            mime: None,
            action_log,
            error: Some(e),
        }),
    }
}

#[tauri::command]
pub async fn browser_session_status(
    browser_state: tauri::State<'_, Arc<BrowserState>>,
) -> Result<serde_json::Value, String> {
    let session = browser_state.session.lock().await;
    Ok(serde_json::json!({
        "page_id": session.selected_page_id,
        "headless": session.headless,
        "control_mode": session.control_mode,
        "action_log": session.action_log,
    }))
}

#[tauri::command]
pub async fn browser_list_pages(
    daemon: tauri::State<'_, Arc<PersistentDaemon>>,
) -> Result<Vec<BrowserPageSummary>, String> {
    let result = mcp_tool_call(&daemon, "vox_browser_list_pages", serde_json::json!({})).await?;
    let data = mcp_data(&result)?;
    let pages = data
        .get("pages")
        .cloned()
        .ok_or_else(|| "list_pages returned no pages payload".to_string())?;
    serde_json::from_value(pages).map_err(|e| format!("decode pages: {e}"))
}

#[derive(serde::Deserialize)]
pub struct BrowserAttachInput {
    pub page_id: String,
}

#[tauri::command]
pub async fn browser_attach_session(
    browser_state: tauri::State<'_, Arc<BrowserState>>,
    daemon: tauri::State<'_, Arc<PersistentDaemon>>,
    input: BrowserAttachInput,
) -> Result<serde_json::Value, String> {
    let page_id = input.page_id.trim().to_string();
    if page_id.is_empty() {
        return Err("page_id is required".to_string());
    }
    mcp_tool_call(
        &daemon,
        "vox_browser_set_viewport",
        serde_json::json!({ "page_id": page_id, "width": 1280, "height": 800, "actor": "human" }),
    )
    .await
    .and_then(|v| mcp_data(&v).map(|_| ()))?;
    mcp_tool_call(
        &daemon,
        "vox_browser_set_control_lock",
        serde_json::json!({ "page_id": page_id, "owner": "human" }),
    )
    .await
    .and_then(|v| mcp_data(&v).map(|_| ()))?;
    let mut session = browser_state.session.lock().await;
    session.selected_page_id = Some(input.page_id.clone());
    push_action_log(&mut session, format!("attach page_id={}", input.page_id));
    Ok(serde_json::json!({ "page_id": input.page_id }))
}

#[derive(serde::Deserialize)]
pub struct BrowserPageInfoInput {
    pub page_id: Option<String>,
}

#[tauri::command]
pub async fn browser_page_info(
    browser_state: tauri::State<'_, Arc<BrowserState>>,
    daemon: tauri::State<'_, Arc<PersistentDaemon>>,
    input: BrowserPageInfoInput,
) -> Result<BrowserPageInfo, String> {
    let page_id = if let Some(p) = input
        .page_id
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        p.to_string()
    } else {
        let session = browser_state.session.lock().await;
        session
            .selected_page_id
            .clone()
            .ok_or_else(|| "no selected page; open or attach first".to_string())?
    };
    let result = mcp_tool_call(
        &daemon,
        "vox_browser_page_info",
        serde_json::json!({ "page_id": page_id }),
    )
    .await?;
    let data = mcp_data(&result)?;
    let info = data
        .get("info")
        .cloned()
        .ok_or_else(|| "page_info returned no info payload".to_string())?;
    serde_json::from_value(info).map_err(|e| format!("decode page_info: {e}"))
}

#[derive(serde::Deserialize)]
pub struct BrowserNavigateInput {
    pub action: String,
}

#[tauri::command]
pub async fn browser_navigate(
    browser_state: tauri::State<'_, Arc<BrowserState>>,
    daemon: tauri::State<'_, Arc<PersistentDaemon>>,
    input: BrowserNavigateInput,
) -> Result<(), String> {
    let page_id = {
        let session = browser_state.session.lock().await;
        session
            .selected_page_id
            .clone()
            .ok_or_else(|| "no selected page; open or attach first".to_string())?
    };
    let action = input.action.trim().to_lowercase();
    let tool = match action.as_str() {
        "back" => "vox_browser_back",
        "forward" => "vox_browser_forward",
        "reload" => "vox_browser_reload",
        "stop" => "vox_browser_stop",
        _ => return Err(format!("unknown navigate action: {}", input.action)),
    };
    mcp_tool_call(
        &daemon,
        tool,
        serde_json::json!({ "page_id": page_id, "actor": "human" }),
    )
    .await
    .and_then(|v| mcp_data(&v).map(|_| ()))?;
    let mut session = browser_state.session.lock().await;
    push_action_log(&mut session, format!("navigate {action}"));
    Ok(())
}

#[derive(serde::Deserialize)]
pub struct BrowserGotoUrlInput {
    pub url: String,
}

#[tauri::command]
pub async fn browser_goto_url(
    browser_state: tauri::State<'_, Arc<BrowserState>>,
    daemon: tauri::State<'_, Arc<PersistentDaemon>>,
    input: BrowserGotoUrlInput,
) -> Result<(), String> {
    let page_id = {
        let session = browser_state.session.lock().await;
        session
            .selected_page_id
            .clone()
            .ok_or_else(|| "no selected page; open or attach first".to_string())?
    };
    let url = input.url.trim();
    if url.is_empty() {
        return Err("url is required".to_string());
    }
    let result = mcp_tool_call(
        &daemon,
        "vox_browser_goto",
        serde_json::json!({ "page_id": page_id, "url": url, "actor": "human" }),
    )
    .await?;
    mcp_data(&result)?;
    let mut session = browser_state.session.lock().await;
    push_action_log(&mut session, format!("goto {url}"));
    log_needs_human(&mut session, &result);
    Ok(())
}

#[derive(serde::Deserialize)]
pub struct BrowserScrollInput {
    pub dx: i64,
    pub dy: i64,
}

#[tauri::command]
pub async fn browser_scroll(
    browser_state: tauri::State<'_, Arc<BrowserState>>,
    daemon: tauri::State<'_, Arc<PersistentDaemon>>,
    input: BrowserScrollInput,
) -> Result<(), String> {
    let page_id = {
        let session = browser_state.session.lock().await;
        session
            .selected_page_id
            .clone()
            .ok_or_else(|| "no selected page; open or attach first".to_string())?
    };
    mcp_tool_call(
        &daemon,
        "vox_browser_scroll",
        serde_json::json!({ "page_id": page_id, "dx": input.dx, "dy": input.dy, "actor": "human" }),
    )
    .await
    .and_then(|v| mcp_data(&v).map(|_| ()))?;
    Ok(())
}

#[derive(serde::Deserialize)]
pub struct BrowserClickInput {
    pub x: f64,
    pub y: f64,
}

#[tauri::command]
pub async fn browser_click_xy(
    browser_state: tauri::State<'_, Arc<BrowserState>>,
    daemon: tauri::State<'_, Arc<PersistentDaemon>>,
    input: BrowserClickInput,
) -> Result<(), String> {
    let page_id = {
        let session = browser_state.session.lock().await;
        session
            .selected_page_id
            .clone()
            .ok_or_else(|| "no selected page; open or attach first".to_string())?
    };
    let result = mcp_tool_call(
        &daemon,
        "vox_browser_click_xy",
        serde_json::json!({ "page_id": page_id, "x": input.x, "y": input.y, "actor": "human" }),
    )
    .await?;
    mcp_data(&result)?;
    let mut session = browser_state.session.lock().await;
    push_action_log(
        &mut session,
        format!("click_xy ({:.1}, {:.1})", input.x, input.y),
    );
    log_needs_human(&mut session, &result);
    Ok(())
}

#[derive(serde::Deserialize)]
pub struct BrowserTypeInput {
    pub text: String,
}

#[tauri::command]
pub async fn browser_type_text(
    browser_state: tauri::State<'_, Arc<BrowserState>>,
    daemon: tauri::State<'_, Arc<PersistentDaemon>>,
    input: BrowserTypeInput,
) -> Result<(), String> {
    let page_id = {
        let session = browser_state.session.lock().await;
        session
            .selected_page_id
            .clone()
            .ok_or_else(|| "no selected page; open or attach first".to_string())?
    };
    let result = mcp_tool_call(
        &daemon,
        "vox_browser_type",
        serde_json::json!({ "page_id": page_id, "text": input.text, "actor": "human" }),
    )
    .await?;
    mcp_data(&result)?;
    let mut session = browser_state.session.lock().await;
    log_needs_human(&mut session, &result);
    Ok(())
}

#[derive(serde::Deserialize)]
pub struct BrowserKeyInput {
    pub key: String,
}

#[tauri::command]
pub async fn browser_input_key(
    browser_state: tauri::State<'_, Arc<BrowserState>>,
    daemon: tauri::State<'_, Arc<PersistentDaemon>>,
    input: BrowserKeyInput,
) -> Result<(), String> {
    let page_id = {
        let session = browser_state.session.lock().await;
        session
            .selected_page_id
            .clone()
            .ok_or_else(|| "no selected page; open or attach first".to_string())?
    };
    let result = mcp_tool_call(
        &daemon,
        "vox_browser_press",
        serde_json::json!({ "page_id": page_id, "key": input.key, "actor": "human" }),
    )
    .await?;
    mcp_data(&result)?;
    let mut session = browser_state.session.lock().await;
    log_needs_human(&mut session, &result);
    Ok(())
}

#[derive(serde::Deserialize)]
pub struct BrowserControlModeInput {
    pub mode: String,
}

#[tauri::command]
pub async fn browser_set_control_mode(
    browser_state: tauri::State<'_, Arc<BrowserState>>,
    daemon: tauri::State<'_, Arc<PersistentDaemon>>,
    input: BrowserControlModeInput,
) -> Result<(), String> {
    let mode = input.mode.trim().to_ascii_lowercase();
    if mode != "you" && mode != "agent" {
        return Err("mode must be \"you\" or \"agent\"".to_string());
    }
    let page_id = {
        let mut session = browser_state.session.lock().await;
        session.control_mode = mode.clone();
        push_action_log(&mut session, format!("control mode -> {mode}"));
        session.selected_page_id.clone()
    };
    if let Some(page_id) = page_id {
        let owner = if mode == "you" { "human" } else { "agent" };
        mcp_tool_call(
            &daemon,
            "vox_browser_set_control_lock",
            serde_json::json!({ "page_id": page_id, "owner": owner }),
        )
        .await
        .and_then(|v| mcp_data(&v).map(|_| ()))?;
    }
    Ok(())
}

#[derive(serde::Deserialize)]
pub struct PlaywrightValidateInput {
    pub preview_url: Option<String>,
}

#[tauri::command]
pub async fn browser_validate_playwright(
    browser_state: tauri::State<'_, Arc<BrowserState>>,
    input: PlaywrightValidateInput,
) -> Result<PlaywrightValidateResult, String> {
    let preview_url = if let Some(u) = input
        .preview_url
        .as_ref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
    {
        u.to_string()
    } else {
        let preview = browser_state.preview.lock().await;
        preview
            .url
            .clone()
            .ok_or_else(|| "no preview URL; start preview or pass preview_url".to_string())?
    };

    let repo = vox_repository::resolve_repo_root_for_ci();
    let ui_dir = repo.join("crates/vox-gui/ui");
    if !ui_dir.is_dir() {
        return Err(format!("GUI ui dir not found: {}", ui_dir.display()));
    }

    let spec_path = ui_dir.join("e2e").join("browser-preview.spec.ts");
    if !spec_path.is_file() {
        return Err(format!("playwright spec missing: {}", spec_path.display()));
    }

    // Playwright install + run can take minutes; never block the async runtime.
    tokio::task::spawn_blocking(move || run_playwright_validate(&ui_dir, &preview_url))
        .await
        .map_err(|e| format!("spawn_blocking: {e}"))?
}

/// Blocking helper: install Chromium (idempotent) then run the preview spec.
fn run_playwright_validate(
    ui_dir: &std::path::Path,
    preview_url: &str,
) -> Result<PlaywrightValidateResult, String> {
    let pnpm = vox_cli::frontend::pnpm_executable();

    let install = quiet_command(pnpm)
        .args(["exec", "playwright", "install", "chromium"])
        .current_dir(ui_dir)
        .env("VOX_PREVIEW_URL", preview_url)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| format!("playwright install spawn: {e}"))?;

    if !install.status.success() {
        return Ok(PlaywrightValidateResult {
            exit_code: install.status.code().unwrap_or(1),
            stdout: String::from_utf8_lossy(&install.stdout).to_string(),
            stderr: format!(
                "playwright install failed: {}",
                String::from_utf8_lossy(&install.stderr)
            ),
            preview_url: Some(preview_url.to_string()),
        });
    }

    let test = quiet_command(pnpm)
        .args([
            "exec",
            "playwright",
            "test",
            "e2e/browser-preview.spec.ts",
            "--reporter=line",
        ])
        .current_dir(ui_dir)
        .env("VOX_PREVIEW_URL", preview_url)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .map_err(|e| format!("playwright test spawn: {e}"))?;

    Ok(PlaywrightValidateResult {
        exit_code: test.status.code().unwrap_or(1),
        stdout: String::from_utf8_lossy(&test.stdout).to_string(),
        stderr: String::from_utf8_lossy(&test.stderr).to_string(),
        preview_url: Some(preview_url.to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_app_dir_relative_to_repo() {
        let got = resolve_app_dir(Some("apps/experimental/visualizer".into())).unwrap();
        assert!(got.ends_with("apps/experimental/visualizer"));
    }

    #[test]
    fn preview_status_default_inactive() {
        let state = BrowserState::default();
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let preview = state.preview.lock().await;
            assert!(preview.url.is_none());
            assert_eq!(preview.source, "none");
        });
    }

    #[test]
    fn mcp_data_unwraps_tool_result_envelope() {
        // Mirrors the real `orch.tool_call` envelope: a parsed `ToolResult`.
        let ok = serde_json::json!({
            "success": true,
            "data": { "page_id": "page-123", "url": "https://example.com" }
        });
        let data = mcp_data(&ok).expect("success envelope");
        assert_eq!(
            data.get("page_id").and_then(|v| v.as_str()),
            Some("page-123")
        );
        assert_eq!(extract_page_id_from_mcp(&ok).as_deref(), Some("page-123"));
    }

    #[test]
    fn mcp_data_surfaces_error_and_remediation() {
        let bad = serde_json::json!({
            "success": false,
            "error": "no such page",
            "remediation": "call vox_browser_open first"
        });
        let err = mcp_data(&bad).unwrap_err();
        assert!(err.contains("no such page"));
        assert!(err.contains("vox_browser_open"));
        assert!(extract_page_id_from_mcp(&bad).is_none());
    }

    #[test]
    fn frame_bytes_from_mcp_data_reads_path() {
        let dir = std::env::temp_dir().join(format!("vox-gui-frame-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("f.png");
        let png = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==",
        )
        .unwrap();
        std::fs::write(&path, &png).unwrap();
        let data = serde_json::json!({
            "path": path.to_string_lossy(),
            "width": 1,
            "height": 1
        });
        let (b64, w, h, mime) = frame_bytes_from_mcp_data(&data, &dir).expect("read");
        assert_eq!(mime, "image/png");
        assert_eq!(w, Some(1));
        assert_eq!(h, Some(1));
        assert!(!b64.is_empty());
    }

    #[test]
    fn frame_bytes_from_mcp_data_rejects_escape() {
        let jail = std::env::temp_dir().join(format!("vox-gui-jail-{}", std::process::id()));
        std::fs::create_dir_all(&jail).unwrap();
        let data = serde_json::json!({ "path": "/etc/hosts" });
        assert!(frame_bytes_from_mcp_data(&data, &jail).is_err());
    }

    #[test]
    fn frame_bytes_from_mcp_data_rejects_spoofed_inline_image() {
        let data = serde_json::json!({
            "image_base64": base64::engine::general_purpose::STANDARD.encode(b"<svg/>"),
            "mime": "image/png"
        });
        assert!(frame_bytes_from_mcp_data(&data, std::env::temp_dir().as_path()).is_err());
    }

    #[test]
    fn frame_bytes_from_mcp_data_rejects_dimensions_over_u32() {
        let data = serde_json::json!({
            "image_base64": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==",
            "width": u64::MAX
        });
        assert!(frame_bytes_from_mcp_data(&data, std::env::temp_dir().as_path()).is_err());
    }

    #[test]
    fn preview_url_must_be_loopback() {
        assert!(is_loopback_preview_url("http://127.0.0.1:3000"));
        assert!(is_loopback_preview_url("http://localhost:5173/app"));
        assert!(is_loopback_preview_url("http://[::1]:3000"));
        assert!(require_loopback_preview_url("https://example.com").is_err());
        assert!(!is_loopback_preview_url("https://example.com"));
        assert_eq!(
            approved_loopback_preview_url("http://127.0.0.1:3000"),
            Some("http://127.0.0.1:3000".into())
        );
        assert_eq!(approved_loopback_preview_url("https://evil.example"), None);
        assert_eq!(approved_loopback_preview_url("  "), None);
    }

    #[test]
    fn open_session_none_and_ephemeral_use_vox_browser_open() {
        let none = BrowserOpenInput {
            url: "https://example.com".into(),
            headless: Some(true),
            mode: None,
            profile_id: None,
            save_profile: None,
            cdp_url: None,
        };
        let (tool, args) = open_session_mcp_call(&none).unwrap();
        assert_eq!(tool, "vox_browser_open");
        assert_eq!(
            args,
            serde_json::json!({ "url": "https://example.com", "headless": true })
        );

        let ephemeral = BrowserOpenInput {
            mode: Some("ephemeral".into()),
            ..none
        };
        let (tool, args) = open_session_mcp_call(&ephemeral).unwrap();
        assert_eq!(tool, "vox_browser_open");
        assert_eq!(
            args.get("url").and_then(|v| v.as_str()),
            Some("https://example.com")
        );
    }

    #[test]
    fn open_session_named_and_connect_chrome_use_open_ex() {
        let named = BrowserOpenInput {
            url: "https://example.com".into(),
            headless: Some(false),
            mode: Some("named".into()),
            profile_id: Some("staging-1".into()),
            save_profile: Some(false),
            cdp_url: None,
        };
        let (tool, args) = open_session_mcp_call(&named).unwrap();
        assert_eq!(tool, "vox_browser_open_ex");
        assert_eq!(args.get("mode").and_then(|v| v.as_str()), Some("named"));
        assert_eq!(
            args.get("save_profile").and_then(|v| v.as_bool()),
            Some(false)
        );
        assert_eq!(
            args.get("profile_id").and_then(|v| v.as_str()),
            Some("staging-1")
        );

        let chrome = BrowserOpenInput {
            mode: Some("connect-chrome".into()),
            cdp_url: Some("http://127.0.0.1:9222".into()),
            profile_id: None,
            ..named
        };
        let (tool, args) = open_session_mcp_call(&chrome).unwrap();
        assert_eq!(tool, "vox_browser_open_ex");
        assert_eq!(args.get("mode").and_then(|v| v.as_str()), Some("attach"));
        assert_eq!(
            args.get("cdp_url").and_then(|v| v.as_str()),
            Some("http://127.0.0.1:9222")
        );
    }

    #[test]
    fn snapshot_mcp_args_include_boxes() {
        let args = snapshot_mcp_args("page-1");
        assert_eq!(args.get("page_id").and_then(|v| v.as_str()), Some("page-1"));
        assert_eq!(
            args.get("include_boxes").and_then(|v| v.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn needs_human_reason_reads_mcp_data() {
        let result = serde_json::json!({
            "success": true,
            "data": { "ok": false, "needs_human": true, "reason": "password_field" }
        });
        assert_eq!(
            needs_human_reason(&result).as_deref(),
            Some("password_field")
        );
        let ok = serde_json::json!({ "success": true, "data": { "page_id": "p1" } });
        assert!(needs_human_reason(&ok).is_none());
    }

    #[test]
    fn package_has_script_detects_dev_ssr_upstream() {
        let dir = std::env::temp_dir().join(format!("vox-gui-pkg-test-{}", now_ms()));
        std::fs::create_dir_all(&dir).unwrap();
        let pkg = dir.join("package.json");
        std::fs::write(
            &pkg,
            r#"{ "scripts": { "dev:ssr-upstream": "vite --port 3001 --strictPort" } }"#,
        )
        .unwrap();
        assert!(package_has_script(&pkg, "dev:ssr-upstream"));
        assert!(!package_has_script(&pkg, "nonexistent"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
