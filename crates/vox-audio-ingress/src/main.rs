//! HTTP ingress for Oratio STT — aligns with [`contracts/codex-api.openapi.yaml`] `/api/audio/*` paths.
//!
//! **Design:** capture stays on the client (browser/CLI/mobile); this service resolves paths under
//! `VOX_ORATIO_WORKSPACE` (or CWD) and runs `vox-oratio` transcription.
//!
//! Bind: `VOX_DASH_HOST` / `VOX_DASH_PORT` (default `127.0.0.1:3847`).

use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use axum::{
    Json, Router,
    extract::{
        DefaultBodyLimit, Multipart, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tokio::fs;
use tracing::{info, warn};

#[derive(Clone)]
struct AppState {
    workspace_root: PathBuf,
}

fn workspace_root() -> PathBuf {
    std::env::var_os("VOX_ORATIO_WORKSPACE")
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn resolve_audio_path(workspace: &Path, path_str: &str) -> PathBuf {
    let p = Path::new(path_str);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        workspace.join(p)
    }
}

#[derive(Serialize)]
struct StatusBody {
    ok: bool,
    summary: &'static str,
    candle: serde_json::Value,
    runtime: serde_json::Value,
}

async fn api_audio_status() -> impl IntoResponse {
    let body = StatusBody {
        ok: true,
        summary: vox_oratio::transcript_status(),
        candle: vox_oratio::candle_backend_status_json(),
        runtime: vox_oratio::runtime_config_diagnostic_json(
            &vox_oratio::OratioRuntimeConfig::resolve(),
        ),
    };
    (StatusCode::OK, Json(body))
}

#[derive(Deserialize)]
struct TranscribeJsonBody {
    path: String,
    #[serde(default)]
    language_hint: Option<String>,
}

#[derive(Serialize)]
struct TranscribeResponse {
    correlation_id: String,
    path: PathBuf,
    raw_text: String,
    refined_text: String,
    text: String,
    confidence: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    n_best: Option<Vec<String>>,
}

async fn api_audio_transcribe_json(
    State(state): State<AppState>,
    Json(body): Json<TranscribeJsonBody>,
) -> impl IntoResponse {
    let full = resolve_audio_path(&state.workspace_root, &body.path);
    if !full.exists() {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "file_not_found", "path": full })),
        )
            .into_response();
    }
    let rtc = vox_oratio::OratioRuntimeConfig::resolve();
    let ctx = vox_oratio::refine::CorrectionContext::from_runtime(
        &rtc,
        vox_oratio::refine::OratioCorrectionProfile::Balanced,
        false,
    );
    let detail =
        match vox_oratio::transcribe_path_detailed(&full, &ctx, body.language_hint.as_deref()) {
            Ok(d) => d,
            Err(e) => {
                return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "transcribe_failed", "message": e.to_string() })),
            )
                .into_response();
            }
        };
    let correlation_id = vox_oratio::trace::new_correlation_id();
    let resp = TranscribeResponse {
        correlation_id,
        path: full,
        raw_text: detail.raw_text,
        refined_text: detail.refined_text.clone(),
        text: detail.refined_text.clone(),
        confidence: detail.confidence,
        n_best: detail.n_best,
    };
    (StatusCode::OK, Json(resp)).into_response()
}

/// Multipart: field `audio` (WAV preferred) or `file`; optional `language_hint` text field.
async fn api_audio_transcribe_upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let mut language_hint: Option<String> = None;
    let mut saved: Option<PathBuf> = None;

    loop {
        let field = match multipart.next_field().await {
            Ok(Some(f)) => f,
            Ok(None) => break,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": "multipart_read_failed", "message": e.to_string() })),
                )
                    .into_response();
            }
        };
        let name = field.name().map(str::to_string).unwrap_or_default();
        if name == "language_hint" {
            if let Ok(text) = field.text().await {
                language_hint = Some(text);
            }
            continue;
        }
        if name != "audio" && name != "file" {
            continue;
        }
        let orig = field
            .file_name()
            .map(ToString::to_string)
            .unwrap_or_else(|| "upload.bin".into());
        let ext = Path::new(&orig)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("wav");
        let tmp = state.workspace_root.join(".vox/tmp/audio_ingress");
        if let Err(e) = tokio::fs::create_dir_all(&tmp).await {
            warn!("mkdir tmp: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "temp_dir_failed" })),
            )
                .into_response();
        }
        let dest = tmp.join(format!("{}.{}", uuid::Uuid::new_v4(), ext));
        let data = match field.bytes().await {
            Ok(b) => b,
            Err(e) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({ "error": "read_body_failed", "message": e.to_string() })),
                )
                    .into_response();
            }
        };
        if data.is_empty() {
            continue;
        }
        if let Err(e) = fs::write(&dest, &data).await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": "write_failed", "message": e.to_string() })),
            )
                .into_response();
        }
        saved = Some(dest);
        break;
    }

    let Some(path) = saved else {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": "missing_audio_field", "hint": "multipart field 'audio' or 'file'" })),
        )
            .into_response();
    };

    let rtc = vox_oratio::OratioRuntimeConfig::resolve();
    let ctx = vox_oratio::refine::CorrectionContext::from_runtime(
        &rtc,
        vox_oratio::refine::OratioCorrectionProfile::Balanced,
        false,
    );
    let detail = match vox_oratio::transcribe_path_detailed(&path, &ctx, language_hint.as_deref()) {
        Ok(d) => d,
        Err(e) => {
            let _ = tokio::fs::remove_file(&path).await;
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "transcribe_failed", "message": e.to_string() })),
            )
                .into_response();
        }
    };
    let _ = tokio::fs::remove_file(&path).await;
    let correlation_id = vox_oratio::trace::new_correlation_id();
    let resp = TranscribeResponse {
        correlation_id,
        path,
        raw_text: detail.raw_text,
        refined_text: detail.refined_text.clone(),
        text: detail.refined_text,
        confidence: detail.confidence,
        n_best: detail.n_best,
    };
    (StatusCode::OK, Json(resp)).into_response()
}

#[derive(Deserialize)]
struct WsControl {
    op: String,
    #[serde(default)]
    language_hint: Option<String>,
}

fn decode_pcm_i16le_mono(data: &[u8]) -> Vec<f32> {
    data.chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / i16::MAX as f32)
        .collect()
}

fn write_pcm_wav_16k(path: &Path, pcm: &[f32]) -> anyhow::Result<()> {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 16_000,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| anyhow::anyhow!("mkdir {parent:?}: {e}"))?;
    }
    let mut writer =
        hound::WavWriter::create(path, spec).map_err(|e| anyhow::anyhow!("wav create: {e}"))?;
    for &s in pcm {
        let v = (s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        writer
            .write_sample(v)
            .map_err(|e| anyhow::anyhow!("wav write: {e}"))?;
    }
    writer
        .finalize()
        .map_err(|e| anyhow::anyhow!("wav finalize: {e}"))?;
    Ok(())
}

async fn transcribe_pcm_buffer(
    state: &AppState,
    pcm: Vec<f32>,
    language_hint: Option<String>,
) -> anyhow::Result<vox_oratio::TranscribeDetail> {
    let workspace = state.workspace_root.clone();
    tokio::task::spawn_blocking(move || -> anyhow::Result<vox_oratio::TranscribeDetail> {
        let tmp = workspace.join(".vox/tmp/audio_stream");
        std::fs::create_dir_all(&tmp)?;
        let wav = tmp.join(format!("ws-{}.wav", uuid::Uuid::new_v4()));
        write_pcm_wav_16k(&wav, &pcm)?;
        let rtc = vox_oratio::OratioRuntimeConfig::resolve();
        let ctx = vox_oratio::refine::CorrectionContext::from_runtime(
            &rtc,
            vox_oratio::refine::OratioCorrectionProfile::Balanced,
            false,
        );
        let out = vox_oratio::transcribe_path_detailed(&wav, &ctx, language_hint.as_deref());
        let _ = std::fs::remove_file(&wav);
        out
    })
    .await
    .map_err(|e| anyhow::anyhow!("join: {e}"))?
}

async fn send_ws_json(socket: &mut WebSocket, value: serde_json::Value) -> anyhow::Result<()> {
    socket
        .send(Message::Text(value.to_string().into()))
        .await
        .map_err(|e| anyhow::anyhow!("ws send: {e}"))
}

async fn handle_audio_stream_ws(state: AppState, mut socket: WebSocket) {
    let correlation_id = vox_oratio::trace::new_correlation_id();
    let mut language_hint: Option<String> = None;
    let mut pcm: Vec<f32> = Vec::new();
    let mut last_emit = Instant::now();
    let mut last_progress_log = Instant::now();
    let max_buffer_ms = std::env::var("VOX_ORATIO_STREAM_MAX_BUFFER_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(300_000);
    let max_samples = ((max_buffer_ms as usize).saturating_mul(16_000) / 1000).max(16_000);

    let _ = send_ws_json(
        &mut socket,
        serde_json::json!({
            "type": "ready",
            "correlation_id": correlation_id,
            "sample_rate_hz": 16000,
            "input_format": "pcm_s16le_mono",
        }),
    )
    .await;

    while let Some(msg) = socket.recv().await {
        let Ok(msg) = msg else {
            break;
        };
        match msg {
            Message::Binary(b) => {
                let mut decoded = decode_pcm_i16le_mono(&b);
                if decoded.is_empty() {
                    continue;
                }
                if pcm.len().saturating_add(decoded.len()) > max_samples {
                    let _ = send_ws_json(
                        &mut socket,
                        serde_json::json!({
                            "type": "error",
                            "error": "buffer_overflow",
                            "max_buffer_ms": max_buffer_ms,
                        }),
                    )
                    .await;
                    break;
                }
                pcm.append(&mut decoded);
                if last_progress_log.elapsed() >= Duration::from_secs(3) {
                    info!(
                        target: "vox_audio_ingress_ws",
                        correlation_id = correlation_id,
                        samples = pcm.len(),
                        elapsed_ms = last_progress_log.elapsed().as_millis() as u64,
                        "audio stream buffering"
                    );
                    last_progress_log = Instant::now();
                }
                if pcm.len() >= 16_000 && last_emit.elapsed() >= Duration::from_secs(3) {
                    match transcribe_pcm_buffer(&state, pcm.clone(), language_hint.clone()).await {
                        Ok(detail) => {
                            let _ = send_ws_json(
                                &mut socket,
                                serde_json::json!({
                                    "type": "partial",
                                    "correlation_id": correlation_id,
                                    "text": detail.refined_text,
                                    "confidence": detail.confidence,
                                    "n_best": detail.n_best,
                                }),
                            )
                            .await;
                        }
                        Err(e) => {
                            warn!(target: "vox_audio_ingress_ws", correlation_id = correlation_id, error = %e, "partial transcribe failed");
                        }
                    }
                    last_emit = Instant::now();
                }
            }
            Message::Text(t) => {
                let ctrl: WsControl = match serde_json::from_str(&t) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                match ctrl.op.as_str() {
                    "set_language" => {
                        language_hint = ctrl.language_hint.clone();
                    }
                    "cancel" => {
                        let _ = send_ws_json(
                            &mut socket,
                            serde_json::json!({
                                "type": "cancelled",
                                "correlation_id": correlation_id,
                            }),
                        )
                        .await;
                        break;
                    }
                    "commit" => {
                        match transcribe_pcm_buffer(&state, pcm.clone(), language_hint.clone())
                            .await
                        {
                            Ok(detail) => {
                                let _ = send_ws_json(
                                    &mut socket,
                                    serde_json::json!({
                                        "type": "final",
                                        "correlation_id": correlation_id,
                                        "raw_text": detail.raw_text,
                                        "refined_text": detail.refined_text,
                                        "text": detail.refined_text,
                                        "confidence": detail.confidence,
                                        "n_best": detail.n_best,
                                    }),
                                )
                                .await;
                            }
                            Err(e) => {
                                let _ = send_ws_json(
                                    &mut socket,
                                    serde_json::json!({
                                        "type": "error",
                                        "correlation_id": correlation_id,
                                        "error": "transcribe_failed",
                                        "message": e.to_string(),
                                    }),
                                )
                                .await;
                            }
                        }
                        break;
                    }
                    _ => {}
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}

async fn api_audio_transcribe_stream_ws(
    State(state): State<AppState>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_audio_stream_ws(state, socket))
}

async fn health() -> impl IntoResponse {
    (StatusCode::OK, Json(serde_json::json!({ "status": "ok" })))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let host = std::env::var("VOX_DASH_HOST")
        .unwrap_or_else(|_| std::net::Ipv4Addr::LOCALHOST.to_string());
    let port: u16 = std::env::var("VOX_DASH_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3847);
    let workspace_root = workspace_root();
    info!(
        workspace = %workspace_root.display(),
        "vox-audio-ingress starting"
    );

    let state = AppState { workspace_root };

    let app = Router::new()
        .route("/health", get(health))
        .route("/api/audio/status", get(api_audio_status))
        .route("/api/audio/transcribe", post(api_audio_transcribe_json))
        .route(
            "/api/audio/transcribe/stream",
            get(api_audio_transcribe_stream_ws),
        )
        .route(
            "/api/audio/transcribe/upload",
            post(api_audio_transcribe_upload).layer(DefaultBodyLimit::max(32 * 1024 * 1024)),
        )
        .with_state(state);

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(%addr, "listening");
    axum::serve(listener, app).await?;
    Ok(())
}
