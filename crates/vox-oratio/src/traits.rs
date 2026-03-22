//! Public transcript types and the default entrypoints used by CLI / MCP.

use std::path::Path;

use anyhow::{Context, Result};

/// File- or segment-level transcription result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transcript {
    /// Raw model or file output before refinement.
    pub raw_text: String,
    /// Optional refined text (ITN, light cleanup). `None` if refinement skipped.
    pub refined_text: Option<String>,
}

impl Transcript {
    /// Prefer refined text when present, otherwise raw.
    #[must_use]
    pub fn display_text(&self) -> &str {
        self.refined_text.as_deref().unwrap_or(&self.raw_text)
    }
}

/// Human-readable description of which Oratio capabilities are active.
#[must_use]
pub fn transcript_status() -> &'static str {
    #[cfg(feature = "stt-candle")]
    {
        "Vox Oratio: Candle Whisper (Rust) STT enabled; symphonia decode + 16 kHz resample; \
         `.txt`/`.md` passthrough. Env: VOX_ORATIO_MODEL, VOX_ORATIO_REVISION, VOX_ORATIO_LANGUAGE, \
         VOX_ORATIO_CUDA (requires `cuda` feature)."
    }
    #[cfg(not(feature = "stt-candle"))]
    {
        "Vox Oratio: built without `stt-candle`; only `.txt`/`.md` transcript passthrough is available."
    }
}

/// Transcribe `path` through the default Oratio pipeline.
///
/// # Supported inputs
///
/// - **`.txt` / `.md`**: UTF-8 content is read as the raw transcript; `refine::rules::light_trim`
///   produces [`Transcript::refined_text`].
/// - **Audio** (with `stt-candle`): common formats (e.g. wav, mp3, flac, ogg) via symphonia.
pub fn transcribe_path(path: &Path) -> Result<Transcript> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if matches!(ext.as_str(), "txt" | "md") {
        let raw_text = std::fs::read_to_string(path)
            .with_context(|| format!("read transcript fixture {}", path.display()))?;
        let refined = crate::refine::rules::light_trim(&raw_text);
        return Ok(Transcript {
            raw_text,
            refined_text: Some(refined),
        });
    }

    #[cfg(feature = "stt-candle")]
    {
        if matches!(
            ext.as_str(),
            "wav" | "mp3" | "flac" | "ogg" | "oga" | "aac" | "m4a" | "mp4" | "opus"
        ) {
            let raw_text = crate::transcribe_audio_file(path)?;
            let refined = crate::refine::rules::light_trim(&raw_text);
            return Ok(Transcript {
                raw_text,
                refined_text: Some(refined),
            });
        }
    }

    anyhow::bail!(
        "Vox Oratio: unsupported extension {:?} for file {}. \
         Supported: .txt / .md{}. Build with `stt-candle` for audio.",
        path.extension().unwrap_or_default(),
        path.display(),
        {
            #[cfg(feature = "stt-candle")]
            {
                " plus .wav, .mp3, .flac, .ogg, …"
            }
            #[cfg(not(feature = "stt-candle"))]
            {
                ""
            }
        }
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::Builder;

    #[test]
    fn txt_fixture_roundtrip() {
        let mut f = Builder::new().suffix(".txt").tempfile().unwrap();
        writeln!(f, "  hello world  ").unwrap();
        let t = transcribe_path(f.path()).unwrap();
        assert_eq!(t.raw_text, "  hello world  \n");
        assert_eq!(t.refined_text.as_deref(), Some("hello world"));
        assert_eq!(t.display_text(), "hello world");
    }
}
