//! MCP tools for Vox Oratio (Candle Whisper STT).

use std::path::{Path, PathBuf};

use crate::server::ServerState;

fn resolve_audio_path(state: &ServerState, path: &str) -> PathBuf {
    let p = Path::new(path);
    if p.is_absolute() {
        p.to_path_buf()
    } else {
        state.repository.root.join(p)
    }
}

/// `vox_oratio_transcribe`: transcribe an audio file under the repo (or absolute path).
pub fn transcribe(state: &ServerState, args: serde_json::Value) -> anyhow::Result<String> {
    let path = args
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("missing string field `path`"))?;
    let full = resolve_audio_path(state, path);
    let t = vox_oratio::transcribe_path(&full)?;
    Ok(serde_json::to_string(&serde_json::json!({
        "path": full,
        "raw_text": t.raw_text,
        "refined_text": t.refined_text,
        "text": t.display_text(),
    }))?)
}

/// `vox_oratio_status`: static line + Candle backend JSON (model env defaults).
pub fn status() -> String {
    serde_json::to_string(&serde_json::json!({
        "summary": vox_oratio::transcript_status(),
        "candle": vox_oratio::candle_backend_status_json(),
    }))
    .unwrap_or_else(|_| "{\"error\":\"serialize\"}".to_string())
}
