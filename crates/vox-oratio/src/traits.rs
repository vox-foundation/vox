//! Public transcript types and the default entrypoints used by CLI / MCP.

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::refine::{CorrectionContext, CorrectionTrace};

fn contextual_bias_phrases_for_session() -> Vec<String> {
    const DEFAULT_MAX: usize = 256;
    let max_phrases: usize = std::env::var("VOX_ORATIO_MAX_BIAS_PHRASES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_MAX);
    let contextual_on = match std::env::var("VOX_ORATIO_CONTEXTUAL_BIAS") {
        Ok(s) if s == "0" || s.eq_ignore_ascii_case("false") => false,
        _ => true,
    };
    if !contextual_on {
        return Vec::new();
    }
    let mut lex_phrases = Vec::new();
    if let Ok(p) = std::env::var("VOX_ORATIO_SPEECH_LEXICON_PATH") {
        let path = std::path::Path::new(p.trim());
        if let Ok(bytes) = std::fs::read(path) {
            if let Ok(lex) = crate::speech_lexicon::SpeechLexicon::from_json_slice(&bytes) {
                lex_phrases = lex.bias_phrases_sorted(max_phrases);
            }
        }
    }
    let extra: Vec<String> = std::env::var("VOX_ORATIO_SESSION_HOTWORDS")
        .map(|s| crate::contextual_bias::parse_hotword_csv(&s))
        .unwrap_or_default();
    crate::contextual_bias::merge_bias_phrases(lex_phrases, &extra, max_phrases)
}

fn finalize_after_refine(
    raw_text: String,
    refined: crate::refine::RefineOutput,
) -> TranscribeDetail {
    let refined_after_lex = apply_optional_project_lexicon(&refined.text);
    let bias = contextual_bias_phrases_for_session();
    let candidates = crate::transcript_rerank::rerank_candidates_best_first_with_context(
        crate::transcript_rerank::build_transcript_candidates(&raw_text, &refined_after_lex),
        &bias,
        Some(raw_text.as_str()),
    );
    let refined_text = candidates
        .first()
        .cloned()
        .unwrap_or_else(|| refined_after_lex.clone());
    let n_best = (candidates.len() > 1).then_some(candidates);
    TranscribeDetail {
        raw_text,
        refined_text,
        confidence: refined.confidence,
        correction_trace: refined.trace,
        n_best,
    }
}

fn apply_optional_project_lexicon(text: &str) -> String {
    if let Ok(p) = std::env::var("VOX_ORATIO_SPEECH_LEXICON_PATH") {
        let path = std::path::Path::new(p.trim());
        if let Ok(bytes) = std::fs::read(path) {
            if let Ok(lex) = crate::speech_lexicon::SpeechLexicon::from_json_slice(&bytes) {
                return lex.apply(text);
            }
        }
    }
    text.to_string()
}

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

/// Deterministic transcription output including refinement metadata (for MCP / contracts).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscribeDetail {
    /// Raw model or file output before refinement.
    pub raw_text: String,
    /// Text after deterministic refinement.
    pub refined_text: String,
    /// Confidence estimate from the refinement stage.
    pub confidence: f32,
    /// Trace of applied refinement rules.
    pub correction_trace: Vec<CorrectionTrace>,
    /// When the STT backend exposes alternatives, list them here (best-first). Usually `None`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub n_best: Option<Vec<String>>,
}

impl TranscribeDetail {
    #[must_use]
    /// Display text (refined).
    pub fn text(&self) -> &str {
        &self.refined_text
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

/// Transcribe `path` with explicit refinement context and optional Whisper language override.
///
/// For `.txt` / `.md`, `language_hint` is ignored. For audio, it is forwarded to the Candle backend.
pub fn transcribe_path_detailed(
    path: &Path,
    ctx: &CorrectionContext,
    language_hint: Option<&str>,
) -> Result<TranscribeDetail> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    if matches!(ext.as_str(), "txt" | "md") {
        let raw_text = std::fs::read_to_string(path)
            .with_context(|| format!("read transcript fixture {}", path.display()))?;
        let refined = crate::refine::refine_transcript(&raw_text, ctx);
        return Ok(finalize_after_refine(raw_text, refined));
    }

    #[cfg(feature = "stt-candle")]
    {
        if matches!(
            ext.as_str(),
            "wav" | "mp3" | "flac" | "ogg" | "oga" | "aac" | "m4a" | "mp4" | "opus"
        ) {
            let (_diag, whisper_lang) = crate::language::prepare_language_hint(language_hint);
            let raw_text =
                crate::transcribe_audio_file_with_language(path, whisper_lang.as_deref())?;
            let refined = crate::refine::refine_transcript(&raw_text, ctx);
            return Ok(finalize_after_refine(raw_text, refined));
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

/// Transcribe `path` through the default Oratio pipeline.
///
/// # Supported inputs
///
/// - **`.txt` / `.md`**: UTF-8 content is read as the raw transcript; `refine::rules::light_trim`
///   produces [`Transcript::refined_text`].
/// - **Audio** (with `stt-candle`): common formats (e.g. wav, mp3, flac, ogg) via symphonia.
pub fn transcribe_path(path: &Path) -> Result<Transcript> {
    let d = transcribe_path_detailed(path, &CorrectionContext::default(), None)?;
    Ok(Transcript {
        raw_text: d.raw_text,
        refined_text: Some(d.refined_text),
    })
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
